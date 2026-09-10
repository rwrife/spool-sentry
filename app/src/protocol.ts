// Protocol v0.1 types and version gate.
//
// Mirrors docs/protocol.md and the firmware domain's rpc/observation
// modules. The companion consumes these payloads; the firmware is
// authoritative. Unavailable numbers are null, never reassuring zeros.

export const PROTOCOL_VERSION = "0.1";

/** Bounded request id, matching firmware rpc::MAX_REQUEST_ID. */
export const MAX_REQUEST_ID = 32;
/** Request budget, matching firmware MAX_REQUEST_BYTES. */
export const MAX_REQUEST_BYTES = 4096;

export type CalibrationState = "valid" | "uncalibrated" | "invalid" | "expired";

export type ChannelState =
  | "fresh"
  | "stale"
  | "disconnected"
  | "out_of_range"
  | "invalid";

export type MassChannelState =
  | "fresh"
  | "settling"
  | "stale"
  | "disconnected"
  | "saturated"
  | "out_of_range"
  | "invalid";

export interface EnvQuality {
  state: ChannelState;
  sample_age_ms: number | null;
}

export interface MassQuality {
  state: MassChannelState;
  sample_age_ms: number | null;
  uncertainty_g: number | null;
}

/** Observation envelope; canonical schema in docs/schemas/. */
export interface Observation {
  protocol_version: string;
  device_id: string;
  sequence: number;
  sampled_at: string | null;
  temperature_c: number | null;
  relative_humidity_pct: number | null;
  gross_mass_g: number | null;
  net_mass_estimate_g: number | null;
  stable: boolean;
  calibration_state: CalibrationState;
  quality: { environment: EnvQuality; mass: MassQuality };
  faults: string[];
}

export type ErrorCode =
  | "malformed_request"
  | "request_too_large"
  | "unsupported_version"
  | "unknown_operation"
  | "missing_field"
  | "invalid_params"
  | "presence_required"
  | "confirmation_required"
  | "rate_limited"
  | "storage_fault"
  | "internal_error";

export interface DeviceError {
  code: ErrorCode;
  message: string;
}

export interface ResponseEnvelope<T> {
  protocol_version: string;
  request_id: string;
  ok: boolean;
  data?: T;
  error?: DeviceError;
}

/**
 * Major-version gate (PROTO-001): reject unsupported majors, accept
 * compatible minor/additive payloads. Returns the validated envelope or
 * throws with a machine code the UI can render textually.
 */
export function checkVersion(
  payload: unknown,
): asserts payload is { protocol_version: string } {
  if (
    typeof payload !== "object" ||
    payload === null ||
    typeof (payload as { protocol_version?: unknown }).protocol_version !==
      "string"
  ) {
    throw new ProtocolError(
      "missing_field",
      "Response has no protocol_version",
    );
  }
  assertMinorCompatible(
    (payload as { protocol_version: string }).protocol_version,
  );
}

export class ProtocolError extends Error {
  constructor(
    public code:
      | "unsupported_version"
      | "missing_field"
      | "malformed_request"
      | "invalid_params",
    message: string,
  ) {
    super(message);
    this.name = "ProtocolError";
  }
}

export function assertMinorCompatible(version: string): void {
  const major = PROTOCOL_VERSION.split(".")[0] as string;
  const [gotMajor, gotMinor] = version.split(".");
  if (
    gotMajor !== major ||
    gotMinor === undefined ||
    !/^\d+$/.test(gotMajor) ||
    !/^\d+$/.test(gotMinor)
  ) {
    throw new ProtocolError(
      "unsupported_version",
      `Device speaks protocol ${version}; this companion speaks ${PROTOCOL_VERSION}`,
    );
  }
}

let nextRequestId = 0;

/** Bounded opaque request id — never user-identifying. */
export function newRequestId(): string {
  nextRequestId = (nextRequestId + 1) % 1_000_000;
  return `c${Date.now().toString(36)}-${nextRequestId}`;
}

export function buildRequest(
  op: string,
  params?: Record<string, unknown>,
): string {
  const body: Record<string, unknown> = {
    request_id: newRequestId(),
    protocol_version: PROTOCOL_VERSION,
    op,
  };
  if (params !== undefined) body.params = params;
  const text = JSON.stringify(body);
  if (new TextEncoder().encode(text).length > MAX_REQUEST_BYTES) {
    throw new ProtocolError("invalid_params", "Request exceeds size budget");
  }
  return text;
}

/** Whether the environment channel is stale per protocol.md (120 s). */
export const ENV_STALE_MS = 120_000;
/** Whether the mass channel is stale per protocol.md (10 s). */
export const MASS_STALE_MS = 10_000;

export function isFresh(ageMs: number | null, limitMs: number): boolean {
  return ageMs !== null && ageMs <= limitMs;
}
