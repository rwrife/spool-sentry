//! Transport-neutral freshness classification and staleness thresholds.

/// Protocol staleness thresholds (protocol.md "Core observation").
pub const ENV_STALE_AFTER_MS: u64 = 120_000;
/// Mass samples go stale quickly so a removed load is never shown as fresh.
pub const MASS_STALE_AFTER_MS: u64 = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Freshness {
    Fresh,
    Stale,
    Invalid,
}

/// Classify a sample using monotonic milliseconds.
///
/// Clock reversal is invalid rather than being converted into a plausible
/// age.
pub const fn classify_freshness(
    now_ms: u64,
    sampled_ms: Option<u64>,
    stale_after_ms: u64,
) -> Freshness {
    let Some(sampled_ms) = sampled_ms else {
        return Freshness::Invalid;
    };
    let Some(age_ms) = now_ms.checked_sub(sampled_ms) else {
        return Freshness::Invalid;
    };
    if age_ms <= stale_after_ms {
        Freshness::Fresh
    } else {
        Freshness::Stale
    }
}

/// Monotonic sample age, invalid on clock reversal.
pub const fn sample_age_ms(now_ms: u64, sampled_ms: Option<u64>) -> Option<u64> {
    match sampled_ms {
        Some(sampled_ms) => now_ms.checked_sub(sampled_ms),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Freshness, MASS_STALE_AFTER_MS, classify_freshness, sample_age_ms};

    #[test]
    fn classifies_fresh_and_stale_at_boundary() {
        assert_eq!(classify_freshness(1_010, Some(1_000), 10), Freshness::Fresh);
        assert_eq!(classify_freshness(1_011, Some(1_000), 10), Freshness::Stale);
    }

    #[test]
    fn missing_or_future_sample_is_invalid() {
        assert_eq!(classify_freshness(100, None, 10), Freshness::Invalid);
        assert_eq!(classify_freshness(100, Some(101), 10), Freshness::Invalid);
    }

    #[test]
    fn mass_threshold_is_ten_seconds() {
        assert_eq!(MASS_STALE_AFTER_MS, 10_000);
        assert_eq!(
            classify_freshness(10_000, Some(0), MASS_STALE_AFTER_MS),
            Freshness::Fresh
        );
        assert_eq!(
            classify_freshness(10_001, Some(0), MASS_STALE_AFTER_MS),
            Freshness::Stale
        );
    }

    #[test]
    fn age_saturates_to_none_on_reversal() {
        assert_eq!(sample_age_ms(5, Some(6)), None);
        assert_eq!(sample_age_ms(6, Some(6)), Some(0));
    }
}
