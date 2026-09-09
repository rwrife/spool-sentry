//! Sensor reading types, adapter traits, and impossible-value validation.
//!
//! The domain consumes readings through these typed envelopes; hardware
//! adapters fill them in and must map raw bus failures onto the explicit
//! fault variants (never onto zero — unavailable numbers are `null`,
//! never reassuring zeros, per protocol.md).

/// Physically plausible envelope for the selected SHT40-class sensor.
/// Values outside this range indicate a corrupt bus read, not weather.
const TEMP_LIMIT_C: f64 = 125.0;
const TEMP_SANITY_MIN_C: f64 = -40.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvSample {
    pub temperature_c: f64,
    pub relative_humidity_pct: f64,
    /// Monotonic capture time.
    pub at_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvError {
    ImpossibleTemperature,
    ImpossibleHumidity,
}

impl EnvSample {
    /// Reject non-finite or physically impossible readings.
    pub fn validate(self) -> Result<Self, EnvError> {
        if !self.temperature_c.is_finite()
            || self.temperature_c < TEMP_SANITY_MIN_C
            || self.temperature_c > TEMP_LIMIT_C
        {
            return Err(EnvError::ImpossibleTemperature);
        }
        if !self.relative_humidity_pct.is_finite()
            || self.relative_humidity_pct < 0.0
            || self.relative_humidity_pct > 100.0
        {
            return Err(EnvError::ImpossibleHumidity);
        }
        Ok(self)
    }
}

/// One environmental-channel read at `now_ms`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EnvReading {
    /// Valid sample (still subject to impossible-value validation).
    Ok(EnvSample),
    /// The last sample is older than the adapter's own tolerance.
    Stale(u64),
    /// Explicit bus/sensor disconnect.
    Disconnected,
}

/// One load-cell channel reading.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MassReading {
    /// Raw ADC counts from the bridge front-end captured at `at_ms`.
    Raw { counts: i64, at_ms: u64 },
    /// The ADC reported saturation; counts are meaningless.
    Saturated { at_ms: u64 },
    /// Explicit out-of-range condition distinct from saturation.
    OutOfRange { at_ms: u64 },
    /// The bridge/sensor link is disconnected.
    Disconnected,
}

/// Injected environmental sensor boundary.
pub trait EnvSource {
    fn read(&mut self, now_ms: u64) -> EnvReading;
}

/// Injected load-cell ADC boundary.
pub trait MassSource {
    fn read(&mut self, now_ms: u64) -> MassReading;
}

/// Aggregate of the injected sensor adapters plus the resettable device id.
pub struct SensorHub<'a, E: EnvSource, M: MassSource> {
    pub env: E,
    pub mass: M,
    device_id: &'a str,
}

impl<'a, E: EnvSource, M: MassSource> SensorHub<'a, E, M> {
    pub fn new(env: E, mass: M, device_id: &'a str) -> Self {
        Self {
            env,
            mass,
            device_id,
        }
    }

    /// The random, resettable device identifier (never MAC/serial-derived).
    pub fn device_id(&self) -> &'a str {
        self.device_id
    }
}

#[cfg(test)]
mod tests {
    use super::{EnvError, EnvSample};

    const OK: EnvSample = EnvSample {
        temperature_c: 23.4,
        relative_humidity_pct: 18.2,
        at_ms: 0,
    };

    #[test]
    fn accepts_plausible_readings() {
        assert_eq!(OK.validate(), Ok(OK));
    }

    #[test]
    fn rejects_impossible_humidity_and_temperature() {
        let bad_humidity = EnvSample {
            temperature_c: 20.0,
            relative_humidity_pct: 100.5,
            at_ms: 0,
        };
        assert_eq!(bad_humidity.validate(), Err(EnvError::ImpossibleHumidity));
        let bad_temp = EnvSample {
            temperature_c: 9_999.0,
            relative_humidity_pct: 50.0,
            at_ms: 0,
        };
        assert_eq!(bad_temp.validate(), Err(EnvError::ImpossibleTemperature));
    }

    #[test]
    fn rejects_non_finite_values() {
        let nan = EnvSample {
            temperature_c: f64::NAN,
            relative_humidity_pct: 50.0,
            at_ms: 0,
        };
        assert!(nan.validate().is_err());
        let inf = EnvSample {
            temperature_c: 20.0,
            relative_humidity_pct: f64::INFINITY,
            at_ms: 0,
        };
        assert!(inf.validate().is_err());
    }
}
