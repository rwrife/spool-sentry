//! Physical-presence sessions (CONN-002 / mutation rules in protocol.md).
//!
//! The button opens a short-lived session with a random nonce. Mutations
//! require a *recent* session; expiry is automatic and monotonic. The
//! session nonce never leaves the device and is never derived from MAC or
//! serial. Randomness is injected so host tests stay deterministic; the
//! target adapter feeds a hardware RNG.

/// Maximum session lifetime once opened.
pub const PRESENCE_SESSION_TTL_MS: u64 = 60_000;
/// Mutations additionally require the session to have been *refreshed*
/// recently (button press / confirmed UI action) within this window.
pub const PRESENCE_RECENCY_MS: u64 = 15_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresenceError {
    NoActiveSession,
    /// Session expired and must be re-opened with the button.
    SessionExpired,
    /// Session is active but was not refreshed recently enough.
    StalePresence,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Presence {
    opened_at_ms: Option<u64>,
    last_touch_ms: Option<u64>,
    expires_at_ms: Option<u64>,
    nonce: u64,
}

impl Presence {
    pub fn new() -> Self {
        Self::default()
    }

    /// Physical button press: open a fresh session or refresh the current
    /// one. `nonce` must come from an unpredictable source.
    pub fn press(&mut self, now_ms: u64, nonce: u64) {
        self.opened_at_ms = Some(now_ms);
        self.last_touch_ms = Some(now_ms);
        self.expires_at_ms = Some(now_ms.saturating_add(PRESENCE_SESSION_TTL_MS));
        self.nonce = nonce;
    }

    /// Expire the session explicitly (provisioning window end, reset).
    pub fn close(&mut self) {
        *self = Self::default();
    }

    pub fn is_active(&self, now_ms: u64) -> bool {
        match (self.expires_at_ms, self.last_touch_ms) {
            (Some(expires), Some(_)) => now_ms < expires,
            _ => false,
        }
    }

    /// Gate for destructive/confirmable operations.
    pub fn require_recent(&self, now_ms: u64) -> Result<(), PresenceError> {
        let Some(expires) = self.expires_at_ms else {
            return Err(PresenceError::NoActiveSession);
        };
        if now_ms >= expires {
            return Err(PresenceError::SessionExpired);
        }
        let last_touch = self.last_touch_ms.unwrap_or(0);
        if now_ms.saturating_sub(last_touch) > PRESENCE_RECENCY_MS {
            return Err(PresenceError::StalePresence);
        }
        Ok(())
    }

    /// Nonce for binding confirmation tokens to this session.
    pub fn nonce(&self) -> Option<u64> {
        self.expires_at_ms.map(|_| self.nonce)
    }
}

#[cfg(test)]
mod tests {
    use super::{PRESENCE_RECENCY_MS, PRESENCE_SESSION_TTL_MS, Presence, PresenceError};

    #[test]
    fn no_session_means_no_mutation() {
        let p = Presence::new();
        assert_eq!(p.require_recent(1_000), Err(PresenceError::NoActiveSession));
    }

    #[test]
    fn fresh_press_allows_mutation() {
        let mut p = Presence::new();
        p.press(1_000, 0xABCD);
        assert!(p.is_active(1_001));
        assert_eq!(p.require_recent(1_001), Ok(()));
        assert_eq!(p.nonce(), Some(0xABCD));
    }

    #[test]
    fn session_hard_expires_after_ttl() {
        let mut p = Presence::new();
        p.press(0, 7);
        // Even with continuous refresh intent, the session lifetime is bounded.
        assert!(p.is_active(PRESENCE_SESSION_TTL_MS - 1));
        assert!(!p.is_active(PRESENCE_SESSION_TTL_MS));
        assert_eq!(
            p.require_recent(PRESENCE_SESSION_TTL_MS),
            Err(PresenceError::SessionExpired)
        );
    }

    #[test]
    fn stale_touch_is_rejected_before_expiry() {
        let mut p = Presence::new();
        p.press(0, 7);
        assert_eq!(
            p.require_recent(PRESENCE_RECENCY_MS + 1),
            Err(PresenceError::StalePresence)
        );
    }

    #[test]
    fn close_revokes_immediately() {
        let mut p = Presence::new();
        p.press(0, 7);
        p.close();
        assert_eq!(p.require_recent(1), Err(PresenceError::NoActiveSession));
        assert_eq!(p.nonce(), None);
    }
}
