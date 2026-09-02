#![no_std]

/// The protocol major/minor frozen by issue #1.
pub const PROTOCOL_VERSION: &str = "0.1";

/// Transport-neutral quality state used to prove domain code builds on host and target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Freshness {
    Fresh,
    Stale,
    Invalid,
}

/// Classify a sample using monotonic milliseconds.
///
/// Clock reversal is invalid rather than being converted into a plausible age.
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

#[cfg(test)]
mod tests {
    use super::{Freshness, PROTOCOL_VERSION, classify_freshness};

    #[test]
    fn protocol_version_is_frozen() {
        assert_eq!(PROTOCOL_VERSION, "0.1");
    }

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
}
