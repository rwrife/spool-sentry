//! The domain core: sampling pipeline, fault classification, and device
//! state, wired only through injected adapter traits (FW-002).
//!
//! `DeviceCore::observe` is the single place raw readings become an
//! observation: it classifies each channel independently, keeps
//! disconnect/saturation/out-of-range/stale/invalid states distinct,
//! refuses to reuse stale mass after calibration invalidation, and emits
//! `null` for every unavailable number.

use crate::calibration::{Calibration, CalibrationState};
use crate::faults::{Fault, FaultList, push_unique};
use crate::filter::MassFilter;
use crate::fresh::{ENV_STALE_AFTER_MS, Freshness, MASS_STALE_AFTER_MS, classify_freshness};
use crate::mass::{SpoolMetadata, net_estimate};
use crate::observation::{ChannelState, EnvChannel, MassChannel, ObservationSource};
use crate::sensors::{EnvReading, EnvSource, MassReading, MassSource, SensorHub};

/// Wall-time source; `None` until a trusted clock exists, in which case
/// `sampled_at` must be null (protocol.md).
pub trait WallClock {
    fn unix_epoch_ms(&self) -> Option<u64>;
}

/// Aggregate device state plus per-sample pipelines.
#[derive(Clone, Debug)]
pub struct DeviceCore {
    pub calibration: Calibration,
    pub spool: SpoolMetadata,
    filter: MassFilter,
    sequence: u64,
    /// Set when the load cell is replaced (adapter signals this).
    load_cell_replaced: bool,
}

impl Default for DeviceCore {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceCore {
    pub fn new() -> Self {
        Self {
            calibration: Calibration::new(),
            spool: SpoolMetadata::default(),
            filter: MassFilter::new(),
            sequence: 0,
            load_cell_replaced: false,
        }
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Adapter hook: the physical load cell changed; calibration is
    /// invalidated and mass history is discarded.
    pub fn note_load_cell_replaced(&mut self) {
        self.load_cell_replaced = true;
    }

    /// Produce the next observation from injected adapters.
    pub fn observe<'a, E: EnvSource, M: MassSource>(
        &mut self,
        hub: &mut SensorHub<'a, E, M>,
        clock: &dyn WallClock,
        now_ms: u64,
    ) -> ObservationSource<'a> {
        if self.load_cell_replaced {
            self.calibration.invalidate();
            self.filter.reset();
            self.load_cell_replaced = false;
        }

        let mut faults: FaultList = FaultList::new();

        // ---- environmental channel --------------------------------------
        let env_reading = hub.env.read(now_ms);
        let (mut env_state, env_capture_ms) = match &env_reading {
            EnvReading::Ok(sample) => {
                let fresh = classify_freshness(now_ms, Some(sample.at_ms), ENV_STALE_AFTER_MS)
                    == Freshness::Fresh;
                let state = match sample.validate() {
                    Ok(_) if fresh => ChannelState::Fresh,
                    Ok(_) => ChannelState::Stale,
                    Err(_) => ChannelState::Invalid,
                };
                (state, Some(sample.at_ms))
            }
            EnvReading::Stale(at) => (ChannelState::Stale, Some(*at)),
            EnvReading::Disconnected => (ChannelState::Disconnected, None),
        };
        match env_state {
            ChannelState::Disconnected => {
                push_unique(&mut faults, Fault::SensorDisconnected);
            }
            ChannelState::Invalid => {
                push_unique(&mut faults, Fault::ImpossibleValue);
            }
            ChannelState::OutOfRange => {
                push_unique(&mut faults, Fault::OutOfRange);
            }
            _ => {}
        }
        let (temperature_c, relative_humidity_pct) = match &env_reading {
            EnvReading::Ok(sample) if env_state != ChannelState::Invalid => (
                Some(sample.temperature_c),
                Some(sample.relative_humidity_pct),
            ),
            _ => (None, None),
        };
        if matches!(env_state, ChannelState::Stale) && temperature_c.is_some() {
            // Stale environment data never re-displays numbers.
            env_state = ChannelState::Stale;
        }
        let (temperature_c, relative_humidity_pct) = if env_state == ChannelState::Fresh {
            (temperature_c, relative_humidity_pct)
        } else {
            (None, None)
        };

        // ---- mass channel -------------------------------------------------
        let mass_reading = hub.mass.read(now_ms);
        let mut mass_state = ChannelState::Fresh;
        let mut raw_counts: Option<i64> = None;
        let mut mass_capture_ms: Option<u64> = None;
        let mut stable = false;
        match mass_reading {
            MassReading::Raw { counts, at_ms } => {
                let filtered = self.filter.push(counts, at_ms);
                raw_counts = Some(filtered.counts);
                stable = filtered.stable;
                mass_capture_ms = Some(at_ms);
                if !filtered.stable {
                    mass_state = ChannelState::Settling;
                }
                if classify_freshness(now_ms, Some(at_ms), MASS_STALE_AFTER_MS) == Freshness::Stale
                {
                    mass_state = ChannelState::Stale;
                    stable = false;
                    raw_counts = None; // stale mass never shows last-good numbers
                }
            }
            MassReading::Saturated { at_ms } => {
                mass_state = ChannelState::Saturated;
                mass_capture_ms = Some(at_ms);
                self.filter.reset();
                push_unique(&mut faults, Fault::AdcSaturated);
            }
            MassReading::OutOfRange { at_ms } => {
                mass_state = ChannelState::OutOfRange;
                mass_capture_ms = Some(at_ms);
                self.filter.reset();
                push_unique(&mut faults, Fault::OutOfRange);
            }
            MassReading::Disconnected => {
                mass_state = ChannelState::Disconnected;
                self.filter.reset();
                push_unique(&mut faults, Fault::MassDisconnected);
            }
        }

        let calibration_state = self.calibration.effective_state(now_ms);
        match calibration_state {
            CalibrationState::Invalid => {
                push_unique(&mut faults, Fault::CalibrationInvalid);
            }
            CalibrationState::Expired => {
                push_unique(&mut faults, Fault::CalibrationExpired);
            }
            _ => {}
        }
        if calibration_state != CalibrationState::Valid {
            // No honest mapping exists: numbers are null, never last-good.
            raw_counts = None;
            stable = false;
            if matches!(mass_state, ChannelState::Fresh | ChannelState::Settling) {
                mass_state = ChannelState::Invalid;
            }
        }

        let gross_g = match (raw_counts, calibration_state) {
            (Some(counts), CalibrationState::Valid) => self
                .calibration
                .to_grams(counts, now_ms)
                .ok()
                .filter(|v| v.is_finite()),
            _ => None,
        };
        let net = net_estimate(gross_g, &self.spool);
        if net.negative_beyond_tolerance {
            push_unique(&mut faults, Fault::NetNegative);
        }

        let uncertainty_g = net.uncertainty_g;
        crate::observation::enforce_uncertainty_fault(&mut faults, uncertainty_g);

        self.sequence = self.sequence.wrapping_add(1);

        ObservationSource {
            device_id: hub.device_id(),
            sequence: self.sequence,
            now_ms,
            wall_epoch_ms: clock.unix_epoch_ms(),
            env: EnvChannel {
                state: env_state,
                sample_at_ms: env_capture_ms,
                temperature_c,
                relative_humidity_pct,
            },
            mass: MassChannel {
                state: mass_state,
                sample_at_ms: mass_capture_ms,
                gross_g,
                net: net.net_g,
                uncertainty_g,
                stable,
            },
            calibration: calibration_state,
            faults,
        }
    }

    /// Commit a successful calibration capture through the core so filters
    /// are cleared (the previous zero point no longer applies).
    pub fn calibration_committed(&mut self) {
        self.filter.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::{DeviceCore, WallClock};
    use crate::calibration::CalibrationState;
    use crate::faults::Fault;
    use crate::observation::ChannelState;
    use crate::sensors::{EnvReading, EnvSample, EnvSource, MassReading, MassSource, SensorHub};

    struct FixedClock(Option<u64>);

    impl WallClock for FixedClock {
        fn unix_epoch_ms(&self) -> Option<u64> {
            self.0
        }
    }

    struct FakeEnv(EnvReading);

    impl EnvSource for FakeEnv {
        fn read(&mut self, _now_ms: u64) -> EnvReading {
            self.0
        }
    }

    struct FakeMass {
        reading: MassReading,
        /// When set, each read advances the sample timestamp so the
        /// stability filter can settle across repeated observes.
        advance_ms: Option<u64>,
        next_at_ms: u64,
    }

    impl MassSource for FakeMass {
        fn read(&mut self, _now_ms: u64) -> MassReading {
            match self.reading {
                MassReading::Raw { counts, at_ms } => {
                    let at_ms = at_ms + self.next_at_ms;
                    if let Some(step) = self.advance_ms {
                        self.next_at_ms += step;
                    }
                    MassReading::Raw { counts, at_ms }
                }
                other => other,
            }
        }
    }

    fn ok_env(at_ms: u64) -> FakeEnv {
        FakeEnv(EnvReading::Ok(EnvSample {
            temperature_c: 23.4,
            relative_humidity_pct: 18.2,
            at_ms,
        }))
    }

    fn stable_mass(counts: i64, at_ms: u64) -> FakeMass {
        FakeMass {
            reading: MassReading::Raw { counts, at_ms },
            advance_ms: None,
            next_at_ms: 0,
        }
    }

    /// Same counts but a fresh timestamp each read (250 ms apart), which
    /// lets the stability filter settle after enough observations.
    fn settling_mass(counts: i64) -> FakeMass {
        FakeMass {
            reading: MassReading::Raw { counts, at_ms: 0 },
            advance_ms: Some(250),
            next_at_ms: 0,
        }
    }

    fn hub<'a>(env: FakeEnv, mass: FakeMass, id: &'a str) -> SensorHub<'a, FakeEnv, FakeMass> {
        SensorHub::new(env, mass, id)
    }

    fn calibrated_core(now_ms: u64) -> DeviceCore {
        let mut core = DeviceCore::new();
        let token = core.calibration.begin_tare().unwrap();
        core.calibration.capture_tare(1_000, token).unwrap();
        let token = core.calibration.begin_reference(1_000.0).unwrap();
        core.calibration
            .capture_reference(11_000, token, now_ms)
            .unwrap();
        core.calibration_committed();
        core
    }

    const ID: &str = "random-resettable-id";

    #[test]
    fn nominal_path_emits_fresh_full_observation() {
        let mut core = calibrated_core(0);
        core.spool.empty_mass_g = Some(238.0);
        let mut h = hub(ok_env(0), settling_mass(6_000), ID);
        let clock = FixedClock(Some(1_788_912_000_000));
        let mut final_obs = None;
        for i in 0..12u64 {
            final_obs = Some(core.observe(&mut h, &clock, i * 250 + 249));
        }
        let obs = final_obs.expect("observations were produced");
        assert_eq!(obs.env.state, ChannelState::Fresh);
        assert_eq!(obs.mass.state, ChannelState::Fresh);
        assert!(
            obs.mass.stable,
            "constant load must settle within 12 samples"
        );
        assert_eq!(obs.calibration, CalibrationState::Valid);
        assert_eq!(obs.mass.gross_g, Some(500.0));
        assert_eq!(obs.mass.net, Some(262.0));
        assert!(obs.faults.contains(&Fault::UncertaintyNotCharacterized));
        assert_eq!(obs.sequence, 12);
    }

    #[test]
    fn mass_disconnect_nulls_numbers_with_distinct_fault() {
        let mut core = calibrated_core(0);
        let mut h = hub(
            ok_env(0),
            FakeMass {
                reading: MassReading::Disconnected,
                advance_ms: None,
                next_at_ms: 0,
            },
            ID,
        );
        let obs = core.observe(&mut h, &FixedClock(None), 1_000);
        assert_eq!(obs.mass.state, ChannelState::Disconnected);
        assert_eq!(obs.mass.gross_g, None);
        assert!(obs.faults.contains(&Fault::MassDisconnected));
        assert!(!obs.faults.contains(&Fault::AdcSaturated));
    }

    #[test]
    fn saturation_is_distinct_from_out_of_range() {
        let mut core = calibrated_core(0);
        let mut h = hub(
            ok_env(0),
            FakeMass {
                reading: MassReading::Saturated { at_ms: 0 },
                advance_ms: None,
                next_at_ms: 0,
            },
            ID,
        );
        let obs = core.observe(&mut h, &FixedClock(None), 0);
        assert_eq!(obs.mass.state, ChannelState::Saturated);
        assert!(obs.faults.contains(&Fault::AdcSaturated));

        let mut core = calibrated_core(0);
        let mut h = hub(
            ok_env(0),
            FakeMass {
                reading: MassReading::OutOfRange { at_ms: 0 },
                advance_ms: None,
                next_at_ms: 0,
            },
            ID,
        );
        let obs = core.observe(&mut h, &FixedClock(None), 0);
        assert_eq!(obs.mass.state, ChannelState::OutOfRange);
        assert!(obs.faults.contains(&Fault::OutOfRange));
        assert!(!obs.faults.contains(&Fault::AdcSaturated));
    }

    #[test]
    fn stale_mass_never_shows_last_good_number() {
        let mut core = calibrated_core(0);
        // Sample captured at 0, observed 60 s later: stale by the 10 s rule.
        let mut h = hub(ok_env(0), stable_mass(6_000, 0), ID);
        let obs = core.observe(&mut h, &FixedClock(None), 60_000);
        assert_eq!(obs.mass.state, ChannelState::Stale);
        assert_eq!(obs.mass.gross_g, None, "stale mass must not show numbers");
    }

    #[test]
    fn uncalibrated_never_maps_counts() {
        let mut core = DeviceCore::new();
        let mut h = hub(ok_env(0), stable_mass(6_000, 0), ID);
        let obs = core.observe(&mut h, &FixedClock(None), 0);
        assert_eq!(obs.mass.gross_g, None);
        assert_eq!(obs.calibration, CalibrationState::Uncalibrated);
        assert_eq!(obs.mass.state, ChannelState::Invalid);
    }

    #[test]
    fn load_cell_swap_invalidates_and_clears_history() {
        let mut core = calibrated_core(0);
        core.note_load_cell_replaced();
        let mut h = hub(ok_env(0), stable_mass(6_000, 0), ID);
        let obs = core.observe(&mut h, &FixedClock(None), 0);
        assert_eq!(obs.calibration, CalibrationState::Invalid);
        assert!(obs.faults.contains(&Fault::CalibrationInvalid));
        assert_eq!(obs.mass.gross_g, None);
    }

    #[test]
    fn impossible_env_value_faults_without_reuse() {
        let mut core = calibrated_core(0);
        let mut h = hub(
            FakeEnv(EnvReading::Ok(EnvSample {
                temperature_c: f64::NAN,
                relative_humidity_pct: 18.2,
                at_ms: 0,
            })),
            stable_mass(6_000, 0),
            ID,
        );
        let obs = core.observe(&mut h, &FixedClock(None), 0);
        assert_eq!(obs.env.state, ChannelState::Invalid);
        assert_eq!(obs.env.temperature_c, None);
        assert!(obs.faults.contains(&Fault::ImpossibleValue));
    }

    #[test]
    fn env_disconnect_is_its_own_fault() {
        let mut core = calibrated_core(0);
        let mut h = hub(FakeEnv(EnvReading::Disconnected), stable_mass(6_000, 0), ID);
        let obs = core.observe(&mut h, &FixedClock(None), 0);
        assert_eq!(obs.env.state, ChannelState::Disconnected);
        assert!(obs.faults.contains(&Fault::SensorDisconnected));
    }

    #[test]
    fn sequence_increments_per_observation() {
        let mut core = calibrated_core(0);
        let mut h = hub(ok_env(0), stable_mass(6_000, 0), ID);
        for expected in 1..=5u64 {
            let obs = core.observe(&mut h, &FixedClock(None), expected * 100);
            assert_eq!(obs.sequence, expected);
        }
    }
}
