//! Transport-neutral command router shared by LAN HTTP and USB CDC
//! (PROTO-003: identical semantics; malformed input causes no mutation).
//!
//! The router accepts a bounded JSON request, validates it against the
//! frozen command grammar, and dispatches to domain operations. It never
//! mutates state on parse/validation failure. The HTTP adapter and the
//! USB CDC line reader both feed bytes here; neither reimplements semantics.

use crate::MAX_REQUEST_BYTES;
use crate::json::{self, Item, JsonError, Leaf, Value};

pub const MAX_REQUEST_ID: usize = 32;

/// Request budget per rolling window per interface (protocol.md
/// "bounded… request/event rate"). A malicious or stuck client can spend
/// at most `RATE_BURST` requests per `RATE_WINDOW_MS`; the budget refills
/// when a new window starts.
pub const RATE_WINDOW_MS: u64 = 1_000;
pub const RATE_BURST: u32 = 40;

/// Small token budget shared per interface by the transport adapters.
#[derive(Clone, Copy, Debug)]
pub struct RateLimiter {
    tokens: u32,
    window_start_ms: u64,
}

impl RateLimiter {
    pub const fn new() -> Self {
        Self {
            tokens: RATE_BURST,
            window_start_ms: 0,
        }
    }

    /// Spend one token at `now_ms`; false means over budget (caller must
    /// answer `rate_limited` and mutate nothing).
    pub fn allow(&mut self, now_ms: u64) -> bool {
        if now_ms.saturating_sub(self.window_start_ms) >= RATE_WINDOW_MS {
            self.window_start_ms = now_ms;
            self.tokens = RATE_BURST;
        }
        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Machine-readable error codes (protocol.md "Mutation and error rules").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    MalformedRequest,
    RequestTooLarge,
    UnsupportedVersion,
    UnknownOperation,
    MissingField,
    InvalidParams,
    PresenceRequired,
    ConfirmationRequired,
    RateLimited,
    StorageFault,
    Internal,
}

impl ErrorCode {
    pub const fn code(self) -> &'static str {
        match self {
            ErrorCode::MalformedRequest => "malformed_request",
            ErrorCode::RequestTooLarge => "request_too_large",
            ErrorCode::UnsupportedVersion => "unsupported_version",
            ErrorCode::UnknownOperation => "unknown_operation",
            ErrorCode::MissingField => "missing_field",
            ErrorCode::InvalidParams => "invalid_params",
            ErrorCode::PresenceRequired => "presence_required",
            ErrorCode::ConfirmationRequired => "confirmation_required",
            ErrorCode::RateLimited => "rate_limited",
            ErrorCode::StorageFault => "storage_fault",
            ErrorCode::Internal => "internal_error",
        }
    }

    pub const fn safe_text(self) -> &'static str {
        match self {
            ErrorCode::MalformedRequest => "The request could not be parsed.",
            ErrorCode::RequestTooLarge => "The request was too large.",
            ErrorCode::UnsupportedVersion => "This device speaks protocol 0.1.",
            ErrorCode::UnknownOperation => "Unknown operation.",
            ErrorCode::MissingField => "A required field is missing.",
            ErrorCode::InvalidParams => "The parameters were invalid.",
            ErrorCode::PresenceRequired => "Press the device button and retry.",
            ErrorCode::ConfirmationRequired => "A fresh confirmation is required.",
            ErrorCode::RateLimited => "Too many requests; retry shortly.",
            ErrorCode::StorageFault => "Device storage reported a fault.",
            ErrorCode::Internal => "The device could not complete the request.",
        }
    }
}

/// A validated request: borrowed request id, operation, and parsed params.
#[derive(Clone, Debug, PartialEq)]
pub struct Request<'a> {
    pub request_id: &'a str,
    pub op: Operation<'a>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Operation<'a> {
    Status,
    Snapshot,
    Observations { limit: u32 },
    Event { kind: &'a str, detail: &'a str },
    CalibrationBeginTare,
    CalibrationBeginReference { reference_g: f64 },
    CalibrationCapture { token: u64 },
    CalibrationCancel,
    SetEmptyMass { grams: f64 },
    Export { format: &'a str },
    RestoreValidate,
    RestoreCommit { digest: u64 },
    Delete { scope: &'a str, token: u64 },
}

/// Parse and validate one request. Returns `Err((code, request_id_if_known))`
/// so error responses can echo the bounded request id.
pub fn parse_request(input: &[u8]) -> Result<Request<'_>, (ErrorCode, Option<&str>)> {
    if input.len() > MAX_REQUEST_BYTES {
        return Err((ErrorCode::RequestTooLarge, None));
    }
    let root = match json::parse(input) {
        Ok(value) => value,
        Err(JsonError::TooLarge) => return Err((ErrorCode::RequestTooLarge, None)),
        Err(_) => return Err((ErrorCode::MalformedRequest, None)),
    };
    // `parse` already enforces the root-object shape and the depth cap.
    let Value::Object(entries) = &root;
    let _ = entries;
    let request_id = match root.get("request_id").and_then(Item::as_str) {
        Some(id) if !id.is_empty() && id.len() <= MAX_REQUEST_ID => id,
        _ => return Err((ErrorCode::MissingField, None)),
    };
    let fail = |code: ErrorCode| Err((code, Some(request_id)));

    match root.get("protocol_version").and_then(Item::as_str) {
        Some(crate::PROTOCOL_VERSION) => {}
        Some(_) => return Err((ErrorCode::UnsupportedVersion, Some(request_id))),
        None => return fail(ErrorCode::MissingField),
    }
    let op_name = match root.get("op").and_then(Item::as_str) {
        Some(name) => name,
        None => return fail(ErrorCode::MissingField),
    };
    let params = root.get("params");

    let op = match op_name {
        "status" => Operation::Status,
        "snapshot" => Operation::Snapshot,
        "observations" => {
            let limit = params
                .and_then(|p| p.get("limit"))
                .and_then(Leaf::as_f64)
                .filter(|v| *v >= 1.0 && *v <= 500.0 && *v == (*v as u32) as f64)
                .map(|v| v as u32);
            match limit {
                Some(limit) => Operation::Observations { limit },
                None => return fail(ErrorCode::InvalidParams),
            }
        }
        "event" => {
            let (Some(kind), Some(detail)) = (
                params.and_then(|p| p.get("kind")).and_then(Leaf::as_str),
                params.and_then(|p| p.get("detail")).and_then(Leaf::as_str),
            ) else {
                return fail(ErrorCode::InvalidParams);
            };
            if kind.len() > 24 || detail.len() > 96 {
                return fail(ErrorCode::InvalidParams);
            }
            Operation::Event { kind, detail }
        }
        "calibration_begin_tare" => Operation::CalibrationBeginTare,
        "calibration_begin_reference" => {
            let Some(reference_g) = params
                .and_then(|p| p.get("reference_g"))
                .and_then(Leaf::as_f64)
            else {
                return fail(ErrorCode::InvalidParams);
            };
            Operation::CalibrationBeginReference { reference_g }
        }
        "calibration_capture" => {
            let Some(token) = token_of(params) else {
                return fail(ErrorCode::InvalidParams);
            };
            Operation::CalibrationCapture { token }
        }
        "calibration_cancel" => Operation::CalibrationCancel,
        "set_empty_mass" => {
            let Some(grams) = params
                .and_then(|p| p.get("empty_mass_g"))
                .and_then(Leaf::as_f64)
            else {
                return fail(ErrorCode::InvalidParams);
            };
            Operation::SetEmptyMass { grams }
        }
        "export" => {
            let format = params
                .and_then(|p| p.get("format"))
                .and_then(Leaf::as_str)
                .unwrap_or("json");
            if format != "json" && format != "csv" {
                return fail(ErrorCode::InvalidParams);
            }
            Operation::Export { format }
        }
        "restore_validate" => Operation::RestoreValidate,
        "restore_commit" => {
            let Some(digest) = token_of(params) else {
                return fail(ErrorCode::InvalidParams);
            };
            Operation::RestoreCommit { digest }
        }
        "delete" => {
            let (Some(scope), Some(token)) = (
                params.and_then(|p| p.get("scope")).and_then(Leaf::as_str),
                token_of(params),
            ) else {
                return fail(ErrorCode::InvalidParams);
            };
            Operation::Delete { scope, token }
        }
        _ => return Err((ErrorCode::UnknownOperation, Some(request_id))),
    };
    Ok(Request { request_id, op })
}

fn token_of(params: Option<&Item<'_>>) -> Option<u64> {
    params
        .and_then(|p| p.get("token"))
        .and_then(Leaf::as_f64)
        .filter(|v| *v >= 0.0 && *v <= u64::MAX as f64 && *v == (*v as u64) as f64)
        .map(|v| v as u64)
}

/// Serialize a response envelope. `data` is pre-rendered JSON or `None`
/// for errors.
pub fn render_response<W: core::fmt::Write>(
    out: &mut W,
    request_id: &str,
    result: Result<&str, (ErrorCode, &str)>,
) -> core::fmt::Result {
    write!(
        out,
        "{{\"protocol_version\":\"{}\",\"request_id\":",
        crate::PROTOCOL_VERSION
    )?;
    json::write_str(out, request_id)?;
    match result {
        Ok(data) => writeln!(out, ",\"ok\":true,\"data\":{data}}}"),
        Err((code, _)) => writeln!(
            out,
            ",\"ok\":false,\"error\":{{\"code\":\"{}\",\"message\":\"{}\"}}}}",
            code.code(),
            code.safe_text()
        ),
    }
}

/// Convenience wrapper rendering into a fixed-size buffer.
pub fn render_response_into(
    buf: &mut heapless::String<2048>,
    request_id: &str,
    result: Result<&str, (ErrorCode, &str)>,
) -> Option<usize> {
    struct Sink<'b>(&'b mut heapless::String<2048>);
    impl core::fmt::Write for Sink<'_> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            self.0.push_str(s).map_err(|_| core::fmt::Error)
        }
    }
    let mut sink = Sink(buf);
    render_response(&mut sink, request_id, result).ok()?;
    Some(buf.len())
}

#[cfg(test)]
mod tests {
    use super::{
        ErrorCode, Operation, RATE_BURST, RATE_WINDOW_MS, RateLimiter, parse_request,
        render_response_into,
    };

    fn wrap(op: &str, params: &str) -> heapless::Vec<u8, 512> {
        let mut v = heapless::Vec::new();
        use core::fmt::Write;
        let mut s: heapless::String<448> = heapless::String::new();
        write!(
            s,
            "{{\"request_id\":\"r1\",\"protocol_version\":\"0.1\",\"op\":\"{op}\""
        )
        .unwrap();
        if !params.is_empty() {
            write!(s, ",\"params\":{{{params}}}").unwrap();
        }
        let _ = s.push('}');
        v.extend_from_slice(s.as_bytes()).unwrap();
        v
    }

    #[test]
    fn parses_read_only_ops() {
        let input = wrap("status", "");
        let req = parse_request(&input).unwrap();
        assert_eq!(req.request_id, "r1");
        assert!(matches!(req.op, Operation::Status));
        let input = wrap("snapshot", "");
        let req = parse_request(&input).unwrap();
        assert!(matches!(req.op, Operation::Snapshot));
    }

    #[test]
    fn rejects_unsupported_version_without_dispatch() {
        let input = b"{\"request_id\":\"r1\",\"protocol_version\":\"9.9\",\"op\":\"status\"}";
        assert_eq!(
            parse_request(input),
            Err((ErrorCode::UnsupportedVersion, Some("r1")))
        );
    }

    #[test]
    fn rejects_oversized_input() {
        let huge = [b' '; 4097];
        assert_eq!(
            parse_request(&huge),
            Err((ErrorCode::RequestTooLarge, None))
        );
    }

    #[test]
    fn malformed_json_never_yields_operation() {
        for input in [
            b"{".as_slice(),
            b"{}".as_slice(),
            b"{\"request_id\":\"r1\",\"protocol_version\":\"0.1\"}".as_slice(),
            b"{\"request_id\":\"r1\",\"protocol_version\":\"0.1\",\"op\":42}".as_slice(),
            b"{\"request_id\":\"r1\",\"protocol_version\":\"0.1\",\"op\":\"status\",\"op\":\"status\"}".as_slice(),
            b"{\"request_id\":\"r1\",\"protocol_version\":\"0.1\",\"op\":\"status\"} trailing".as_slice(),
        ] {
            assert!(parse_request(input).is_err(), "must reject: {input:?}");
        }
    }

    #[test]
    fn params_are_typed_and_bounded() {
        let input = wrap("observations", "\"limit\":10");
        let req = parse_request(&input).unwrap();
        assert_eq!(req.op, Operation::Observations { limit: 10 });
        for bad in [
            "\"limit\":0",
            "\"limit\":501",
            "\"limit\":-3",
            "\"limit\":\"10\"",
            "\"limit\":1.5",
        ] {
            assert!(
                parse_request(&wrap("observations", bad)).is_err(),
                "must reject {bad}"
            );
        }
    }

    #[test]
    fn calibration_reference_requires_mass() {
        let input = wrap("calibration_begin_reference", "\"reference_g\":1000.0");
        let req = parse_request(&input).unwrap();
        assert!(
            matches!(req.op, Operation::CalibrationBeginReference { reference_g } if reference_g == 1000.0)
        );
        assert!(parse_request(&wrap("calibration_begin_reference", "")).is_err());
    }

    #[test]
    fn unknown_operation_is_rejected() {
        assert_eq!(
            parse_request(&wrap("reboot_printer", "")),
            Err((ErrorCode::UnknownOperation, Some("r1")))
        );
    }

    #[test]
    fn response_envelope_is_stable_json() {
        let mut buf = heapless::String::new();
        render_response_into(&mut buf, "r1", Ok("{\"stable\":true}")).unwrap();
        assert_eq!(
            buf.as_str(),
            "{\"protocol_version\":\"0.1\",\"request_id\":\"r1\",\"ok\":true,\"data\":{\"stable\":true}}\n"
        );

        let mut buf = heapless::String::new();
        render_response_into(&mut buf, "r2", Err((ErrorCode::PresenceRequired, "x"))).unwrap();
        assert!(buf.contains("\"code\":\"presence_required\""));
        assert!(!buf.contains("x\""));
    }

    #[test]
    fn rate_limiter_reflects_the_budget_constants() {
        let mut limiter = RateLimiter::new();
        // Burst capacity is available immediately.
        for _ in 0..RATE_BURST {
            assert!(limiter.allow(0));
        }
        assert!(!limiter.allow(0), "burst must be exhausted");
        assert!(
            !limiter.allow(RATE_WINDOW_MS - 1),
            "mid-window stays blocked"
        );
        // A new window refills tokens.
        assert!(limiter.allow(RATE_WINDOW_MS));
    }
}
