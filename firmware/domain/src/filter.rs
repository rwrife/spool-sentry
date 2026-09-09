//! Bounded sampling filter and stable-reading detection.
//!
//! The filter is a median-of-last-N (rejects spikes) with a hysteresis
//! stability window (MEAS-004/UX-002): a reading series is `Stable` only
//! after the latest window of samples fits inside a fixed band for a
//! minimum settle time. All storage is fixed-size; no heap.

/// Median window size (odd, small enough for the 10 SPS front-end).
pub const MEDIAN_WINDOW: usize = 7;
/// Stability window size.
pub const STABILITY_WINDOW: usize = 8;
/// Maximum peak-to-peak spread (raw counts) within the stability window
/// before the series counts as settled. Calibrated per-part counts→grams
/// conversion is bench work (issue #6); this is the count-domain gate.
pub const STABLE_BAND_COUNTS: i64 = 40;
/// Minimum time the newest sample must have held inside the band.
pub const MIN_SETTLE_MS: u64 = 1_500;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilteredMass {
    /// Median of the recent window (raw counts).
    pub counts: i64,
    /// True when the series is settled per the hysteresis rule.
    pub stable: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct Sample {
    counts: i64,
    at_ms: u64,
    used: bool,
}

/// Ring-buffer median/stability filter over raw ADC counts.
#[derive(Clone, Debug, Default)]
pub struct MassFilter {
    median_ring: [i64; MEDIAN_WINDOW],
    median_len: usize,
    median_next: usize,
    window: [Sample; STABILITY_WINDOW],
    window_len: usize,
    window_next: usize,
}

impl MassFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push one raw sample and return the filtered view.
    pub fn push(&mut self, counts: i64, at_ms: u64) -> FilteredMass {
        self.median_ring[self.median_next] = counts;
        self.median_next = (self.median_next + 1) % MEDIAN_WINDOW;
        if self.median_len < MEDIAN_WINDOW {
            self.median_len += 1;
        }

        self.window[self.window_next] = Sample {
            counts,
            at_ms,
            used: true,
        };
        self.window_next = (self.window_next + 1) % STABILITY_WINDOW;
        if self.window_len < STABILITY_WINDOW {
            self.window_len += 1;
        }

        let median = self.median();
        FilteredMass {
            counts: median,
            stable: self.is_stable(median, at_ms),
        }
    }

    /// Forget history (e.g. after tare, disconnect, or calibration steps).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn median(&self) -> i64 {
        let mut buf = self.median_ring;
        let n = self.median_len;
        let slice = &mut buf[..n];
        slice.sort_unstable();
        slice[n / 2]
    }

    fn is_stable(&self, median: i64, now_ms: u64) -> bool {
        if self.window_len < STABILITY_WINDOW {
            return false;
        }
        // The whole window must lie within the band around the median and
        // the oldest window sample must be old enough to prove settling.
        let mut oldest_ms = u64::MAX;
        for sample in &self.window {
            if !sample.used {
                return false;
            }
            if (sample.counts - median).abs() > STABLE_BAND_COUNTS / 2 {
                return false;
            }
            oldest_ms = oldest_ms.min(sample.at_ms);
        }
        now_ms.saturating_sub(oldest_ms) >= MIN_SETTLE_MS
    }
}

#[cfg(test)]
mod tests {
    use super::{MassFilter, STABILITY_WINDOW};

    #[test]
    fn median_rejects_spike() {
        let mut filter = MassFilter::new();
        for i in 0..7 {
            filter.push(1_000, i * 100);
        }
        filter.push(500_000, 700); // spike
        let filtered = filter.push(1_000, 800);
        assert!((filtered.counts - 1_000).abs() <= 1);
    }

    #[test]
    fn unstable_until_window_settles() {
        let mut filter = MassFilter::new();
        // Moving load: spread larger than the band.
        for i in 0..(STABILITY_WINDOW * 2) as u64 {
            let filtered = filter.push(1_000 + (i as i64 * 100) % 700, i * 100);
            if i >= STABILITY_WINDOW as u64 {
                assert!(!filtered.stable, "moving load must not report stable");
            }
        }
        // Constant load: needs both a full window and the settle time.
        let mut early_stable = false;
        for i in 0..(STABILITY_WINDOW as u64) {
            early_stable |= filter.push(2_000, 2_000 + i * 100).stable;
        }
        assert!(!early_stable, "window alone must not declare stable");
        let settled = filter.push(2_000, 2_000 + STABILITY_WINDOW as u64 * 100 + 2_000);
        assert!(settled.stable);
    }

    #[test]
    fn reset_clears_history() {
        let mut filter = MassFilter::new();
        for i in 0..16u64 {
            filter.push(1_000, i * 400);
        }
        filter.reset();
        let after = filter.push(3_000, 10_000);
        assert!(!after.stable);
        assert_eq!(after.counts, 3_000);
    }
}
