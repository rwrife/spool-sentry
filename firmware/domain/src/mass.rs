//! Gross/net mass arithmetic with honest precision (MEAS-005).
//!
//! `net = gross - empty`, where `empty` is user-entered spool mass — never
//! inferred. Negative net is an explicit fault, never silently clamped to
//! zero, and uncertainty is `None` until bench characterization provides a
//! value (the observation layer then requires the
//! `uncertainty_not_characterized` fault).

/// Maximum accepted user-entered empty-spool mass (grams).
pub const MAX_EMPTY_MASS_G: f64 = 5_000.0;
/// Tolerance below which a slightly negative net is treated as zero
/// (sensor noise around an empty platform), grams.
pub const NET_NEGATIVE_TOLERANCE_G: f64 = 15.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NetEstimate {
    pub net_g: Option<f64>,
    /// Uncertainty bound in grams; `None` until characterized on the bench.
    pub uncertainty_g: Option<f64>,
    /// Net went negative beyond tolerance: an explicit fault.
    pub negative_beyond_tolerance: bool,
    /// No empty mass has been entered yet.
    pub empty_missing: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SpoolMetadata {
    /// User-entered empty spool mass; `None` until provided.
    pub empty_mass_g: Option<f64>,
}

impl SpoolMetadata {
    /// Validate user input before storing.
    pub fn set_empty_mass(&mut self, grams: f64) -> Result<(), EmptyMassError> {
        if !grams.is_finite() || grams < 0.0 || grams > MAX_EMPTY_MASS_G {
            return Err(EmptyMassError::OutOfRange);
        }
        self.empty_mass_g = Some(grams);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmptyMassError {
    OutOfRange,
}

/// Compute the net estimate from a *validated* fresh gross reading.
pub fn net_estimate(gross_g: Option<f64>, metadata: &SpoolMetadata) -> NetEstimate {
    let Some(gross_g) = gross_g else {
        return NetEstimate {
            net_g: None,
            uncertainty_g: None,
            negative_beyond_tolerance: false,
            empty_missing: metadata.empty_mass_g.is_none(),
        };
    };
    let Some(empty_g) = metadata.empty_mass_g else {
        return NetEstimate {
            net_g: None,
            uncertainty_g: None,
            negative_beyond_tolerance: false,
            empty_missing: true,
        };
    };
    let raw = gross_g - empty_g;
    if raw < -NET_NEGATIVE_TOLERANCE_G {
        return NetEstimate {
            net_g: None,
            uncertainty_g: None,
            negative_beyond_tolerance: true,
            empty_missing: false,
        };
    }
    NetEstimate {
        net_g: Some(raw.max(0.0)),
        // No bench uncertainty model exists yet (issue #6 owns it), so the
        // estimate is honest: uncertainty is unknown, not zero.
        uncertainty_g: None,
        negative_beyond_tolerance: false,
        empty_missing: false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EmptyMassError, MAX_EMPTY_MASS_G, NET_NEGATIVE_TOLERANCE_G, SpoolMetadata, net_estimate,
    };

    fn meta(empty: Option<f64>) -> SpoolMetadata {
        SpoolMetadata {
            empty_mass_g: empty,
        }
    }

    #[test]
    fn missing_inputs_yield_explicit_flags() {
        let est = net_estimate(None, &meta(Some(200.0)));
        assert_eq!(est.net_g, None);
        assert!(!est.empty_missing);

        let est = net_estimate(Some(800.0), &meta(None));
        assert_eq!(est.net_g, None);
        assert!(est.empty_missing);
    }

    #[test]
    fn normal_subtraction() {
        let est = net_estimate(Some(812.0), &meta(Some(238.0)));
        assert_eq!(est.net_g, Some(574.0));
        assert_eq!(est.uncertainty_g, None);
        assert!(!est.negative_beyond_tolerance);
    }

    #[test]
    fn small_negative_clamps_but_big_negative_faults() {
        let est = net_estimate(Some(190.0), &meta(Some(200.0))); // -10 g
        assert_eq!(est.net_g, Some(0.0));
        assert!(!est.negative_beyond_tolerance);

        let est = net_estimate(Some(100.0), &meta(Some(200.0))); // -100 g
        assert_eq!(est.net_g, None);
        assert!(est.negative_beyond_tolerance);
        let _ = NET_NEGATIVE_TOLERANCE_G;
    }

    #[test]
    fn empty_mass_validation() {
        let mut m = SpoolMetadata::default();
        assert_eq!(m.set_empty_mass(-1.0), Err(EmptyMassError::OutOfRange));
        assert_eq!(m.set_empty_mass(f64::NAN), Err(EmptyMassError::OutOfRange));
        assert_eq!(
            m.set_empty_mass(MAX_EMPTY_MASS_G + 1.0),
            Err(EmptyMassError::OutOfRange)
        );
        assert!(m.set_empty_mass(0.0).is_ok());
        assert!(m.set_empty_mass(430.0).is_ok());
    }
}
