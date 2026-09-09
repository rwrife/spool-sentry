//! Calibration lifecycle: uncalibrated → pending → valid → expired/invalid.
//!
//! The two-point tare/reference flow (protocol.md `POST
//! /api/v1/calibration/{step}`) is implemented as an explicit state
//! machine. Steps require an active presence session (enforced by the
//! router) and each pending step carries its own single-use confirmation
//! token bound to the operation. Expired or invalidated calibration
//! nulls mass numbers rather than reusing the last good mapping
//! (MEAS-006: invalidation transitions are complete and tested).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationState {
    Uncalibrated,
    Valid,
    Invalid,
    Expired,
}

impl CalibrationState {
    pub const fn wire(self) -> &'static str {
        match self {
            CalibrationState::Uncalibrated => "uncalibrated",
            CalibrationState::Valid => "valid",
            CalibrationState::Invalid => "invalid",
            CalibrationState::Expired => "expired",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CalibrationStep {
    /// No step pending.
    Idle,
    /// Zero/tare sample requested and waiting for a stable sample.
    PendingTare,
    /// Reference-mass step armed with a user-entered mass.
    PendingReference { reference_g: f64 },
}

/// Calibration errors carry stable machine codes for the API layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationError {
    /// No calibration exists yet; mapping cannot be used.
    NotCalibrated,
    /// Calibration was invalidated (e.g. cell swap) or expired.
    NotValid,
    /// A step was requested while another step was pending.
    StepInProgress,
    /// Confirmation token missing, wrong, or already consumed.
    ConfirmationRequired,
    /// Reference mass outside the allowed instrumented window.
    ReferenceOutOfRange,
    /// The reference produced no meaningful deflection.
    NoDeflection,
}

/// Calibration validity horizon. Beyond it the state degrades to
/// `Expired` at the first use (honest stale-data rule MEAS-006).
pub const CALIBRATION_MAX_AGE_MS: u64 = 30 * 24 * 60 * 60 * 1_000;

/// Reference masses accepted for calibration (documented bench range
/// 100 g–2.0 kg with margin, MEAS-002/003).
const REFERENCE_MIN_G: f64 = 50.0;
const REFERENCE_MAX_G: f64 = 2_200.0;

#[derive(Clone, Debug)]
pub struct Calibration {
    pub state: CalibrationState,
    /// Zero raw counts captured at tare.
    zero_counts: i64,
    /// Scale in grams per raw count derived from the reference step.
    grams_per_count: f64,
    /// Monotonic timestamp when calibration became valid.
    valid_at_ms: Option<u64>,
    pub step: CalibrationStep,
    /// Single-use token for the currently pending step.
    pending_token: Option<u64>,
    token_counter: u64,
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            state: CalibrationState::Uncalibrated,
            zero_counts: 0,
            grams_per_count: 0.0,
            valid_at_ms: None,
            step: CalibrationStep::Idle,
            pending_token: None,
            token_counter: 0,
        }
    }
}

impl Calibration {
    pub fn new() -> Self {
        Self::default()
    }

    /// Current effective state, applying expiry.
    pub fn effective_state(&self, now_ms: u64) -> CalibrationState {
        if self.state == CalibrationState::Valid
            && let Some(valid_at) = self.valid_at_ms
            && now_ms.saturating_sub(valid_at) > CALIBRATION_MAX_AGE_MS
        {
            return CalibrationState::Expired;
        }
        self.state
    }

    /// Arm the tare step. Returns the single-use confirmation token.
    pub fn begin_tare(&mut self) -> Result<u64, CalibrationError> {
        if self.step != CalibrationStep::Idle {
            return Err(CalibrationError::StepInProgress);
        }
        self.step = CalibrationStep::PendingTare;
        Ok(self.issue_token())
    }

    /// Arm the reference step with a user-entered mass.
    pub fn begin_reference(&mut self, reference_g: f64) -> Result<u64, CalibrationError> {
        if self.step != CalibrationStep::Idle {
            return Err(CalibrationError::StepInProgress);
        }
        if !reference_g.is_finite()
            || reference_g < REFERENCE_MIN_G
            || reference_g > REFERENCE_MAX_G
        {
            return Err(CalibrationError::ReferenceOutOfRange);
        }
        self.step = CalibrationStep::PendingReference { reference_g };
        Ok(self.issue_token())
    }

    /// Capture the pending tare from a stable median reading.
    pub fn capture_tare(&mut self, stable_counts: i64, token: u64) -> Result<(), CalibrationError> {
        let expected = self.step == CalibrationStep::PendingTare;
        self.take_confirmation(token, expected)?;
        self.zero_counts = stable_counts;
        self.step = CalibrationStep::Idle;
        Ok(())
    }

    /// Complete calibration from a stable median reading of the reference.
    pub fn capture_reference(
        &mut self,
        stable_counts: i64,
        token: u64,
        now_ms: u64,
    ) -> Result<(), CalibrationError> {
        let reference_g = match self.step {
            CalibrationStep::PendingReference { reference_g } => reference_g,
            _ => {
                return Err(CalibrationError::ConfirmationRequired);
            }
        };
        self.take_confirmation(token, true)?;
        let span = stable_counts - self.zero_counts;
        if span.abs() < 10 {
            // Reference produced no meaningful deflection; calibration is
            // unusable and must not stay Valid.
            self.state = CalibrationState::Invalid;
            self.step = CalibrationStep::Idle;
            return Err(CalibrationError::NoDeflection);
        }
        self.grams_per_count = reference_g / span as f64;
        self.state = CalibrationState::Valid;
        self.valid_at_ms = Some(now_ms);
        self.step = CalibrationStep::Idle;
        Ok(())
    }

    /// Abort any pending step (rollback path; committed calibration stays).
    pub fn cancel_step(&mut self) {
        self.step = CalibrationStep::Idle;
        self.pending_token = None;
    }

    /// Physical change of the load cell invalidates calibration.
    pub fn invalidate(&mut self) {
        self.state = CalibrationState::Invalid;
        self.cancel_step();
        self.valid_at_ms = None;
    }

    /// Map filtered raw counts to grams; refuses unless currently valid.
    pub fn to_grams(&self, counts: i64, now_ms: u64) -> Result<f64, CalibrationError> {
        match self.effective_state(now_ms) {
            CalibrationState::Valid => {
                Ok((counts - self.zero_counts) as f64 * self.grams_per_count)
            }
            CalibrationState::Uncalibrated => Err(CalibrationError::NotCalibrated),
            _ => Err(CalibrationError::NotValid),
        }
    }

    fn issue_token(&mut self) -> u64 {
        self.token_counter = self.token_counter.wrapping_add(1);
        self.pending_token = Some(self.token_counter);
        self.token_counter
    }

    fn take_confirmation(
        &mut self,
        token: u64,
        step_matches: bool,
    ) -> Result<(), CalibrationError> {
        if !step_matches {
            self.cancel_step();
            return Err(CalibrationError::ConfirmationRequired);
        }
        if self.pending_token != Some(token) {
            // Wrong or replayed token: drop the pending step entirely.
            self.cancel_step();
            return Err(CalibrationError::ConfirmationRequired);
        }
        self.pending_token = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CALIBRATION_MAX_AGE_MS, Calibration, CalibrationError, CalibrationState, CalibrationStep,
    };

    #[test]
    fn full_two_point_flow() {
        let mut cal = Calibration::new();
        assert_eq!(cal.effective_state(0), CalibrationState::Uncalibrated);
        assert_eq!(cal.to_grams(500, 0), Err(CalibrationError::NotCalibrated));

        let token = cal.begin_tare().unwrap();
        cal.capture_tare(1_000, token).unwrap();
        assert_eq!(cal.step, CalibrationStep::Idle);

        let token = cal.begin_reference(1_000.0).unwrap();
        // Wrong token drops the step and changes nothing.
        assert_eq!(
            cal.capture_reference(10_000, token + 99, 100),
            Err(CalibrationError::ConfirmationRequired)
        );
        assert_eq!(cal.state, CalibrationState::Uncalibrated);

        // Re-arm with the right flow.
        let token = cal.begin_reference(1_000.0).unwrap();
        cal.capture_reference(11_000, token, 200).unwrap();
        assert_eq!(cal.effective_state(200), CalibrationState::Valid);
        let grams = cal.to_grams(6_000, 200).unwrap();
        assert!((grams - 500.0).abs() < 1e-9);
    }

    #[test]
    fn tokens_are_single_use() {
        let mut cal = Calibration::new();
        let token = cal.begin_tare().unwrap();
        cal.capture_tare(1_000, token).unwrap();
        // Replaying the consumed token for a new step must fail, and the
        // wrong-token attempt drops the pending step entirely (rollback).
        let token2 = cal.begin_tare().unwrap();
        assert_ne!(token, token2);
        assert_eq!(
            cal.capture_tare(1_000, token),
            Err(CalibrationError::ConfirmationRequired)
        );
        let token3 = cal.begin_tare().unwrap();
        cal.capture_tare(1_000, token3).unwrap();
    }

    #[test]
    fn rejects_out_of_range_reference() {
        let mut cal = Calibration::new();
        assert_eq!(
            cal.begin_reference(10.0),
            Err(CalibrationError::ReferenceOutOfRange)
        );
        assert_eq!(
            cal.begin_reference(5_000.0),
            Err(CalibrationError::ReferenceOutOfRange)
        );
        assert_eq!(
            cal.begin_reference(f64::NAN),
            Err(CalibrationError::ReferenceOutOfRange)
        );
        assert!(cal.begin_reference(100.0).is_ok());
    }

    #[test]
    fn second_begin_while_pending_is_rejected() {
        let mut cal = Calibration::new();
        cal.begin_tare().unwrap();
        assert_eq!(cal.begin_tare(), Err(CalibrationError::StepInProgress));
        assert_eq!(
            cal.begin_reference(100.0),
            Err(CalibrationError::StepInProgress)
        );
        cal.cancel_step();
        assert!(cal.begin_reference(100.0).is_ok());
    }

    #[test]
    fn zero_deflection_reference_invalidates() {
        let mut cal = Calibration::new();
        let token = cal.begin_tare().unwrap();
        cal.capture_tare(1_000, token).unwrap();
        let token = cal.begin_reference(1_000.0).unwrap();
        assert_eq!(
            cal.capture_reference(1_002, token, 100),
            Err(CalibrationError::NoDeflection)
        );
        assert_eq!(cal.effective_state(100), CalibrationState::Invalid);
        assert_eq!(cal.to_grams(1_002, 100), Err(CalibrationError::NotValid));
    }

    #[test]
    fn expiry_after_horizon() {
        let mut cal = Calibration::new();
        let token = cal.begin_tare().unwrap();
        cal.capture_tare(0, token).unwrap();
        let token = cal.begin_reference(1_000.0).unwrap();
        cal.capture_reference(10_000, token, 0).unwrap();
        assert_eq!(
            cal.effective_state(CALIBRATION_MAX_AGE_MS),
            CalibrationState::Valid
        );
        assert_eq!(
            cal.effective_state(CALIBRATION_MAX_AGE_MS + 1),
            CalibrationState::Expired
        );
        assert_eq!(
            cal.to_grams(5_000, CALIBRATION_MAX_AGE_MS + 1),
            Err(CalibrationError::NotValid)
        );
    }

    #[test]
    fn invalidation_after_cell_swap() {
        let mut cal = Calibration::new();
        let token = cal.begin_tare().unwrap();
        cal.capture_tare(0, token).unwrap();
        let token = cal.begin_reference(1_000.0).unwrap();
        cal.capture_reference(10_000, token, 0).unwrap();
        cal.invalidate();
        assert_eq!(cal.effective_state(1), CalibrationState::Invalid);
        assert_eq!(cal.to_grams(5_000, 1), Err(CalibrationError::NotValid));
    }
}
