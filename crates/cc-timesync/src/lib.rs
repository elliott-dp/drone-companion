//! # cc-timesync — FC↔CC clock correlation (spec §5.4)
//!
//! Standard MAVLink TIMESYNC exchange: the CC sends `{tc1: 0, ts1:
//! cc_mono_ns}`; PX4 replies `{tc1: fc_ns, ts1: echoed cc_ns}`. Each reply
//! yields an RTT-compensated offset sample:
//!
//! ```text
//! rtt       = now_cc − ts1
//! offset_ns = tc1 − (ts1 + rtt/2)        // fc_ns ≈ cc_ns + offset_ns
//! ```
//!
//! [`Filter`] is **pure** (no clocks, no I/O — the dev plan requires unit
//! tests against synthetic jitter traces): a 32-sample window, rejection of
//! samples whose RTT exceeds 1.5 × the window p90, median offset, and a
//! quality judgement (LOCKED / DEGRADED / UNLOCKED) from window fill, RTT
//! jitter and rejection rate.
//!
//! [`runner`] is the async half: 10 Hz fast-lock for the first 5 s, then
//! 1 Hz; requests leave at P0; consumers read an atomic [`Snapshot`] from a
//! watch channel and **never re-derive** (spec). `px4_boot_id` changes or
//! an FC timestamp regression invalidate the filter and re-enter fast-lock.

use std::collections::VecDeque;

pub mod runner;

/// Timesync confidence (spec §5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    /// ≥ 8 samples, low RTT jitter, low rejection rate: conversions valid.
    Locked,
    /// Some samples but not yet trustworthy; timing-sensitive consumers
    /// reduce confidence (spec §11).
    Degraded,
    /// No usable estimate (startup, post-reboot, link loss).
    Unlocked,
}

/// Atomic snapshot for consumers (logger, ingest age computation, status).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snapshot {
    /// Median RTT-compensated offset: `fc_ns ≈ cc_ns + offset_ns`.
    pub offset_ns: i64,
    /// Median round-trip time of the current window.
    pub rtt_ns: i64,
    pub quality: Quality,
    pub window_len: usize,
    /// Samples rejected as RTT outliers since the last invalidation.
    pub rejected: u32,
}

impl Snapshot {
    pub const UNLOCKED: Snapshot = Snapshot {
        offset_ns: 0,
        rtt_ns: 0,
        quality: Quality::Unlocked,
        window_len: 0,
        rejected: 0,
    };

    /// FC µs-since-boot → CC monotonic ns. Only meaningful when LOCKED
    /// (callers gate on quality; spec §5.5 flags ages `unknown_offset`
    /// otherwise).
    pub fn fc_us_to_cc_ns(&self, fc_us: u64) -> i64 {
        (fc_us as i64) * 1000 - self.offset_ns
    }

    /// CC monotonic ns → FC µs-since-boot.
    pub fn cc_ns_to_fc_us(&self, cc_ns: i64) -> i64 {
        (cc_ns + self.offset_ns) / 1000
    }
}

/// Window size (spec §5.4 "rolling window (e.g., 32 samples)").
pub const WINDOW: usize = 32;
/// Minimum samples for LOCKED.
const LOCK_MIN_SAMPLES: usize = 8;
/// Minimum samples for DEGRADED.
const DEGRADED_MIN_SAMPLES: usize = 4;
/// RTT jitter (p90 − p10) ceiling for LOCKED. Loopback sits ~µs; the
/// 921600-baud UART with ~70-byte frames sits low-ms; 20 ms is comfortably
/// above healthy and below broken.
const LOCK_JITTER_NS: i64 = 20_000_000;
/// Rejection-rate ceiling for LOCKED (over the recent outcome window).
const LOCK_MAX_REJECT_RATE: f64 = 0.30;
/// A sample is an outlier when rtt > p90(window) × this factor.
const REJECT_FACTOR_NUM: i64 = 3;
const REJECT_FACTOR_DEN: i64 = 2;
/// Outlier rejection only engages once the window has this many samples.
const REJECT_MIN_SAMPLES: usize = 8;

/// Pure offset estimator. Feed replies, read [`Filter::estimate`].
#[derive(Debug, Default)]
pub struct Filter {
    /// (offset_ns, rtt_ns), newest at the back.
    window: VecDeque<(i64, i64)>,
    /// Recent accept(=false)/reject(=true) outcomes for the rate metric.
    outcomes: VecDeque<bool>,
    rejected_total: u32,
}

impl Filter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingest one TIMESYNC reply. `tc1_ns` = FC time from the reply,
    /// `ts1_ns` = our echoed request timestamp, `now_ns` = CC time at
    /// reply receipt. Returns whether the sample was accepted.
    pub fn add_reply(&mut self, tc1_ns: i64, ts1_ns: i64, now_ns: i64) -> bool {
        let rtt = now_ns - ts1_ns;
        if rtt < 0 {
            // clock nonsense (reply "before" request): reject outright
            self.push_outcome(true);
            return false;
        }

        if self.window.len() >= REJECT_MIN_SAMPLES {
            let p90 = self.rtt_percentile(90);
            if rtt > p90 * REJECT_FACTOR_NUM / REJECT_FACTOR_DEN {
                self.push_outcome(true);
                return false;
            }
        }

        let offset = tc1_ns - (ts1_ns + rtt / 2);
        self.window.push_back((offset, rtt));
        if self.window.len() > WINDOW {
            self.window.pop_front();
        }
        self.push_outcome(false);
        true
    }

    /// Drop everything (FC reboot / timestamp regression, spec §5.4).
    pub fn invalidate(&mut self) {
        self.window.clear();
        self.outcomes.clear();
        self.rejected_total = 0;
    }

    pub fn estimate(&self) -> Snapshot {
        if self.window.is_empty() {
            return Snapshot::UNLOCKED;
        }

        let offset = self.offset_median();
        let rtt_p50 = self.rtt_percentile(50);
        let jitter = self.rtt_percentile(90) - self.rtt_percentile(10);
        let reject_rate = if self.outcomes.is_empty() {
            0.0
        } else {
            self.outcomes.iter().filter(|r| **r).count() as f64 / self.outcomes.len() as f64
        };

        let quality = if self.window.len() >= LOCK_MIN_SAMPLES
            && jitter <= LOCK_JITTER_NS
            && reject_rate < LOCK_MAX_REJECT_RATE
        {
            Quality::Locked
        } else if self.window.len() >= DEGRADED_MIN_SAMPLES {
            Quality::Degraded
        } else {
            Quality::Unlocked
        };

        Snapshot {
            offset_ns: offset,
            rtt_ns: rtt_p50,
            quality,
            window_len: self.window.len(),
            rejected: self.rejected_total,
        }
    }

    fn push_outcome(&mut self, rejected: bool) {
        if rejected {
            self.rejected_total += 1;
        }
        self.outcomes.push_back(rejected);
        if self.outcomes.len() > WINDOW {
            self.outcomes.pop_front();
        }
    }

    fn offset_median(&self) -> i64 {
        let mut v: Vec<i64> = self.window.iter().map(|(o, _)| *o).collect();
        v.sort_unstable();
        v[v.len() / 2]
    }

    fn rtt_percentile(&self, pct: usize) -> i64 {
        let mut v: Vec<i64> = self.window.iter().map(|(_, r)| *r).collect();
        v.sort_unstable();
        let idx = (v.len() * pct / 100).min(v.len() - 1);
        v[idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic pseudo-noise (xorshift64*), same discipline as the
    /// Phase 1 fuzz suite: reproducible traces, no rand dependency.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        /// uniform in [lo, hi)
        fn range(&mut self, lo: i64, hi: i64) -> i64 {
            lo + (self.next() % (hi - lo) as u64) as i64
        }
    }

    /// Simulate an exchange: CC clock `cc`, true offset `off` (fc = cc +
    /// off), request at cc time t, one-way delays d1/d2.
    fn reply(t: i64, off: i64, d1: i64, d2: i64) -> (i64, i64, i64) {
        let tc1 = t + d1 + off; // FC stamps its clock when request arrives
        (tc1, t, t + d1 + d2) // (tc1_ns, ts1_ns, now_ns at reply receipt)
    }

    #[test]
    fn clean_trace_locks_with_exact_offset() {
        let mut f = Filter::new();
        let off = 123_456_789_000; // fc ahead of cc by ~123.5 ms
        for i in 0..WINDOW as i64 {
            let (tc1, ts1, now) = reply(i * 100_000_000, off, 500_000, 500_000);
            assert!(f.add_reply(tc1, ts1, now));
        }
        let e = f.estimate();
        assert_eq!(e.quality, Quality::Locked);
        assert_eq!(e.offset_ns, off, "symmetric path -> exact offset");
        assert_eq!(e.rtt_ns, 1_000_000);
        assert_eq!(e.window_len, WINDOW);
        assert_eq!(e.rejected, 0);
    }

    #[test]
    fn asymmetric_jitter_stays_within_half_rtt_bound() {
        let mut f = Filter::new();
        let off = -42_000_000; // fc behind cc
        let mut rng = Rng(7);
        for i in 0..64 {
            let d1 = rng.range(200_000, 3_000_000);
            let d2 = rng.range(200_000, 3_000_000);
            let (tc1, ts1, now) = reply(i * 100_000_000, off, d1, d2);
            f.add_reply(tc1, ts1, now);
        }
        let e = f.estimate();
        assert_eq!(e.quality, Quality::Locked);
        // offset error is bounded by half the worst path asymmetry
        assert!((e.offset_ns - off).abs() < 1_500_000,
                "median offset {} vs true {}", e.offset_ns, off);
    }

    #[test]
    fn outlier_bursts_are_rejected_and_estimate_holds() {
        let mut f = Filter::new();
        let off = 5_000_000_000;
        for i in 0..16 {
            let (tc1, ts1, now) = reply(i * 100_000_000, off, 400_000, 400_000);
            f.add_reply(tc1, ts1, now);
        }
        let before = f.estimate();
        // burst of wildly delayed replies (e.g. a scheduler stall):
        // rtt 100 ms >> p90(0.8ms) * 1.5
        let mut rejected = 0;
        for i in 16..24 {
            let (tc1, ts1, now) = reply(i * 100_000_000, off, 50_000_000, 50_000_000);
            if !f.add_reply(tc1, ts1, now) {
                rejected += 1;
            }
        }
        assert_eq!(rejected, 8, "every outlier rejected");
        let after = f.estimate();
        assert_eq!(after.offset_ns, before.offset_ns, "estimate unmoved by outliers");
        assert_eq!(after.rejected, 8);
    }

    #[test]
    fn sustained_rejections_degrade_quality() {
        let mut f = Filter::new();
        for i in 0..12 {
            let (tc1, ts1, now) = reply(i * 100_000_000, 0, 400_000, 400_000);
            f.add_reply(tc1, ts1, now);
        }
        assert_eq!(f.estimate().quality, Quality::Locked);
        // now 12 outliers: rejection rate over the outcome window crosses 30%
        for i in 12..24 {
            let (tc1, ts1, now) = reply(i * 100_000_000, 0, 80_000_000, 80_000_000);
            f.add_reply(tc1, ts1, now);
        }
        assert_eq!(f.estimate().quality, Quality::Degraded);
    }

    #[test]
    fn invalidate_returns_to_unlocked_then_relocks() {
        let mut f = Filter::new();
        for i in 0..WINDOW as i64 {
            let (tc1, ts1, now) = reply(i * 100_000_000, 1_000_000, 300_000, 300_000);
            f.add_reply(tc1, ts1, now);
        }
        assert_eq!(f.estimate().quality, Quality::Locked);

        f.invalidate(); // FC rebooted
        assert_eq!(f.estimate().quality, Quality::Unlocked);
        assert_eq!(f.estimate(), Snapshot::UNLOCKED);

        // new boot: different offset (FC clock restarted)
        let new_off = -987_000_000_000;
        for i in 0..LOCK_MIN_SAMPLES as i64 {
            let (tc1, ts1, now) = reply(1_000_000_000_000 + i * 100_000_000, new_off, 300_000, 300_000);
            f.add_reply(tc1, ts1, now);
        }
        let e = f.estimate();
        assert_eq!(e.quality, Quality::Locked);
        assert_eq!(e.offset_ns, new_off);
    }

    #[test]
    fn high_jitter_never_reaches_locked() {
        let mut f = Filter::new();
        let mut rng = Rng(99);
        for i in 0..WINDOW as i64 {
            // RTTs all over 0.2–80 ms: jitter far beyond LOCK_JITTER_NS.
            // Keep them monotically plausible; rejection may trim some.
            let d = rng.range(100_000, 40_000_000);
            let (tc1, ts1, now) = reply(i * 100_000_000, 0, d, d);
            f.add_reply(tc1, ts1, now);
        }
        assert_ne!(f.estimate().quality, Quality::Locked);
    }

    #[test]
    fn conversions_round_trip() {
        let s = Snapshot {
            offset_ns: 123_456_789,
            rtt_ns: 1,
            quality: Quality::Locked,
            window_len: WINDOW,
            rejected: 0,
        };
        let fc_us = 42_000_000u64;
        let cc = s.fc_us_to_cc_ns(fc_us);
        assert_eq!(s.cc_ns_to_fc_us(cc), fc_us as i64);
        // sign convention: fc = cc + offset  =>  cc = fc*1000 - offset
        assert_eq!(cc, 42_000_000_000 - 123_456_789);
    }
}
