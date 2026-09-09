//! Observation envelope assembly and deterministic JSON serialization.
//!
//! The writer produces byte-stable JSON that conforms to
//! `docs/schemas/observation-v0.1.schema.json`. Host tests validate the
//! emitted bytes against the same fixtures the companion app uses
//! (PROTO-002/003). Unavailable numbers are `null`, never reassuring
//! zeros, and every state/age pairing stays explicit.

use crate::calibration::CalibrationState;
use crate::faults::{Fault, FaultList};
use crate::fresh::sample_age_ms;
use crate::json;
use heapless::String;

pub type ObsJson = String<2048>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChannelState {
    /// State wire values shared with the schema enums.
    Fresh,
    Settling,
    Stale,
    Disconnected,
    Saturated,
    OutOfRange,
    Invalid,
}

impl ChannelState {
    pub const fn wire(self) -> &'static str {
        match self {
            ChannelState::Fresh => "fresh",
            ChannelState::Settling => "settling",
            ChannelState::Stale => "stale",
            ChannelState::Disconnected => "disconnected",
            ChannelState::Saturated => "saturated",
            ChannelState::OutOfRange => "out_of_range",
            ChannelState::Invalid => "invalid",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MassChannel {
    pub state: ChannelState,
    pub sample_at_ms: Option<u64>,
    pub gross_g: Option<f64>,
    pub net: Option<f64>,
    pub uncertainty_g: Option<f64>,
    pub stable: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct EnvChannel {
    pub state: ChannelState,
    pub sample_at_ms: Option<u64>,
    pub temperature_c: Option<f64>,
    pub relative_humidity_pct: Option<f64>,
}

/// Everything an observation needs, already classified by the domain core.
#[derive(Clone, Debug)]
pub struct ObservationSource<'a> {
    pub device_id: &'a str,
    pub sequence: u64,
    pub now_ms: u64,
    /// Trusted wall time in ms since the Unix epoch; `None` until NTP or
    /// a manual sync is trusted, in which case `sampled_at` is null.
    pub wall_epoch_ms: Option<u64>,
    pub env: EnvChannel,
    pub mass: MassChannel,
    pub calibration: CalibrationState,
    pub faults: FaultList,
}

/// Serialize one observation. Returns `None` if the envelope exceeds the
/// buffer (bounded failure, never a truncation that emits invalid JSON).
pub fn serialize(src: &ObservationSource<'_>) -> Option<ObsJson> {
    let mut out: ObsJson = String::new();
    serialize_into(src, &mut out)?;
    Some(out)
}

pub fn serialize_into(src: &ObservationSource<'_>, out: &mut ObsJson) -> Option<()> {
    let mut cursor = core::mem::take(out);
    let result = write_observation(src, &mut cursor);
    *out = cursor;
    result.map(|_| ())
}

fn write_observation(src: &ObservationSource<'_>, out: &mut ObsJson) -> Option<()> {
    json_w(out, "{")?;
    json_kv_str(out, "protocol_version", crate::PROTOCOL_VERSION)?;
    json_w(out, ",")?;
    json_kv_str(out, "device_id", src.device_id)?;
    json_w(out, ",")?;
    json_w(out, "\"sequence\":")?;
    json::write_u64(out, src.sequence).ok()?;
    json_w(out, ",\"sampled_at\":")?;
    match src.wall_epoch_ms {
        Some(epoch) => {
            let mut buf: heapless::String<32> = heapless::String::new();
            crate::timefmt::write_rfc3339_utc(epoch, &mut buf).ok()?;
            json::write_str(out, &buf).ok()?;
        }
        None => json_w(out, "null")?,
    }
    json_w(out, ",\"temperature_c\":")?;
    optional_f64(out, src.env.temperature_c, 1)?;
    json_w(out, ",\"relative_humidity_pct\":")?;
    optional_f64(out, src.env.relative_humidity_pct, 1)?;
    json_w(out, ",\"gross_mass_g\":")?;
    optional_f64(out, src.mass.gross_g, 1)?;
    json_w(out, ",\"net_mass_estimate_g\":")?;
    optional_f64(out, src.mass.net, 1)?;
    json_w(out, ",\"stable\":")?;
    json::write_bool(out, src.mass.stable).ok()?;
    json_w(out, ",\"calibration_state\":")?;
    json::write_str(out, src.calibration.wire()).ok()?;

    json_w(out, ",\"quality\":{\"environment\":{")?;
    json_kv_str(out, "state", src.env.state.wire())?;
    json_w(out, ",\"sample_age_ms\":")?;
    optional_u64(out, sample_age_ms(src.now_ms, src.env.sample_at_ms))?;
    json_w(out, "},\"mass\":{")?;
    json_kv_str(out, "state", src.mass.state.wire())?;
    json_w(out, ",\"sample_age_ms\":")?;
    optional_u64(out, sample_age_ms(src.now_ms, src.mass.sample_at_ms))?;
    json_w(out, ",\"uncertainty_g\":")?;
    optional_f64(out, src.mass.uncertainty_g, 2)?;
    json_w(out, "}},\"faults\":[")?;
    for (index, fault) in src.faults.iter().enumerate() {
        if index > 0 {
            json_w(out, ",")?;
        }
        json::write_str(out, fault.code()).ok()?;
    }
    json_w(out, "]}")?;
    Some(())
}

fn json_w(out: &mut ObsJson, s: &str) -> Option<()> {
    out.push_str(s).ok()
}

fn json_kv_str(out: &mut ObsJson, key: &str, value: &str) -> Option<()> {
    json::write_str(out, key).ok()?;
    json_w(out, ":")?;
    json::write_str(out, value).ok()?;
    Some(())
}

fn optional_f64(out: &mut ObsJson, value: Option<f64>, decimals: u32) -> Option<()> {
    match value {
        Some(v) => json::write_fixed(out, v, decimals).ok()?,
        None => json_w(out, "null")?,
    }
    Some(())
}

fn optional_u64(out: &mut ObsJson, value: Option<u64>) -> Option<()> {
    match value {
        Some(v) => json::write_u64(out, v).ok()?,
        None => json_w(out, "null")?,
    }
    Some(())
}

/// Guarantee the uncertainty rule stays enforced at the boundary: a null
/// `uncertainty_g` requires the explicit fault code.
pub fn enforce_uncertainty_fault(faults: &mut FaultList, uncertainty_g: Option<f64>) {
    if uncertainty_g.is_none() {
        crate::faults::push_unique(faults, Fault::UncertaintyNotCharacterized);
    }
}
