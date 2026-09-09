//! Stable machine-readable fault codes (protocol "faults" array).
//!
//! Every code matches the schema pattern `^[a-z0-9_]+$`, is distinct per
//! condition (disconnect is never merged with saturation, etc.), and the
//! list is bounded to the schema's 32-item limit.

use heapless::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// The load-cell channel reported a disconnect (or the bus read failed).
    MassDisconnected,
    /// The environmental sensor reported a disconnect.
    SensorDisconnected,
    /// The ADC front-end saturated (rail-to-rail input / saturation flag).
    AdcSaturated,
    /// A measurement is explicitly outside the instrumented range.
    OutOfRange,
    /// A sensor delivered a physically impossible value.
    ImpossibleValue,
    /// Calibration data was invalidated (e.g. load-cell replacement).
    CalibrationInvalid,
    /// Calibration exceeded its validity period.
    CalibrationExpired,
    /// Net mass resolved negative beyond the allowed tolerance.
    NetNegative,
    /// A storage read/write/commit failed.
    StorageFault,
    /// Bounded storage refused new data.
    StorageFull,
    /// `uncertainty_g` is null until bench characterization provides a value.
    UncertaintyNotCharacterized,
}

impl Fault {
    pub const fn code(self) -> &'static str {
        match self {
            Fault::MassDisconnected => "mass_disconnected",
            Fault::SensorDisconnected => "sensor_disconnected",
            Fault::AdcSaturated => "adc_saturated",
            Fault::OutOfRange => "out_of_range",
            Fault::ImpossibleValue => "impossible_value",
            Fault::CalibrationInvalid => "calibration_invalid",
            Fault::CalibrationExpired => "calibration_expired",
            Fault::NetNegative => "net_negative",
            Fault::StorageFault => "storage_fault",
            Fault::StorageFull => "storage_full",
            Fault::UncertaintyNotCharacterized => "uncertainty_not_characterized",
        }
    }
}

/// Bounded fault list matching the schema `maxItems: 32`.
pub type FaultList = Vec<Fault, 32>;

/// Push a fault unless it is already present. Returns `false` when the
/// list is full and the fault had to be dropped (the first fault code for
/// a condition always wins, so truncation stays deterministic).
pub fn push_unique(list: &mut FaultList, fault: Fault) -> bool {
    if list.contains(&fault) {
        return true;
    }
    list.push(fault).is_ok()
}
