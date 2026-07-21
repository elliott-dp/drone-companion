//! One/two-sided **CUSUM** — the change detector for a *sustained level shift*.
//!
//! CUSUM is (log-likelihood-ratio) optimal for detecting a persistent change in
//! mean at a given false-alarm rate, and by construction a single-sample
//! transient never accumulates past the threshold — exactly the
//! sustained-fault-yes / glitch-no behaviour the ~zero-false-positive exit
//! criterion needs.
//!
//! Upward accumulator (detecting an increase above `target`):
//! ```text
//! S⁺_k = max(0, S⁺_{k−1} + (x_k − (target + k_slack)))      trip when S⁺ > h
//! ```
//! Downward is symmetric. `k_slack` (the "allowance", ~half the smallest shift
//! worth flagging) is what makes it robust: values within `±k_slack` of
//! `target` drain the accumulator. `target` is normally a slow EWMA baseline
//! (or a fixed reference, e.g. an innovation test-ratio floor).
//!
//! Determinism: a fixed-order scalar recurrence; `NaN` inputs are ignored.

/// Two-sided CUSUM with independent up/down accumulators.
#[derive(Debug, Clone)]
pub struct Cusum {
    k_slack: f64,
    h: f64,
    s_up: f64,
    s_dn: f64,
    frozen: bool,
}

/// Which side (if any) tripped this update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CusumTrip {
    None,
    Up,
    Down,
}

impl Cusum {
    /// `k_slack` = allowance, `h` = decision threshold (both in the units of the
    /// monitored signal).
    pub fn new(k_slack: f64, h: f64) -> Self {
        Self {
            k_slack: k_slack.max(0.0),
            h: if h > 0.0 { h } else { 1.0 },
            s_up: 0.0,
            s_dn: 0.0,
            frozen: false,
        }
    }

    /// Fold in a sample measured against `target`. Returns whether/which side is
    /// currently tripped. Ignored if `x` or `target` is NaN, or if frozen.
    pub fn update(&mut self, x: f64, target: f64) -> CusumTrip {
        if self.frozen || x.is_nan() || target.is_nan() {
            return self.trip();
        }
        let dev = x - target;
        self.s_up = (self.s_up + dev - self.k_slack).max(0.0);
        self.s_dn = (self.s_dn - dev - self.k_slack).max(0.0);
        self.trip()
    }

    pub fn trip(&self) -> CusumTrip {
        // Up takes priority on the (rare) simultaneous trip — deterministic.
        if self.s_up > self.h {
            CusumTrip::Up
        } else if self.s_dn > self.h {
            CusumTrip::Down
        } else {
            CusumTrip::None
        }
    }

    pub fn s_up(&self) -> f64 {
        self.s_up
    }
    pub fn s_dn(&self) -> f64 {
        self.s_dn
    }
    /// Normalized excess over threshold on the tripped side, `[0, ∞)` — feeds a
    /// confidence/magnitude term.
    pub fn excess(&self) -> f64 {
        match self.trip() {
            CusumTrip::Up => (self.s_up - self.h) / self.h,
            CusumTrip::Down => (self.s_dn - self.h) / self.h,
            CusumTrip::None => 0.0,
        }
    }

    /// Freeze accumulation (anti-masking while a finding is active).
    pub fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
    }

    pub fn reset(&mut self) {
        self.s_up = 0.0;
        self.s_dn = 0.0;
        self.frozen = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sustained_up_shift_trips_transient_does_not() {
        let mut c = Cusum::new(0.5, 4.0);
        // benign noise around target 0 within the slack: never trips
        for i in 0..200 {
            let x = if i % 2 == 0 { 0.3 } else { -0.3 };
            assert_eq!(c.update(x, 0.0), CusumTrip::None);
        }
        // one big transient spike: still below h after draining
        c.update(3.0, 0.0);
        assert_ne!(c.trip(), CusumTrip::Up); // single sample can't cross h=4 with slack
        // a sustained +2 shift accumulates and trips
        let mut trip = CusumTrip::None;
        for _ in 0..10 {
            trip = c.update(2.0, 0.0);
            if trip == CusumTrip::Up {
                break;
            }
        }
        assert_eq!(trip, CusumTrip::Up);
        assert!(c.excess() >= 0.0);
    }

    #[test]
    fn downward_shift_detected() {
        let mut c = Cusum::new(0.5, 3.0);
        let mut trip = CusumTrip::None;
        for _ in 0..20 {
            trip = c.update(-2.0, 0.0);
            if trip == CusumTrip::Down {
                break;
            }
        }
        assert_eq!(trip, CusumTrip::Down);
    }

    #[test]
    fn nan_ignored_and_deterministic() {
        let run = || {
            let mut c = Cusum::new(0.2, 5.0);
            let mut acc = 0u64;
            for i in 0..500 {
                let x = ((i as f64) * 0.01).sin();
                c.update(x, 0.0);
                c.update(f64::NAN, 0.0);
                acc = acc.wrapping_add(c.s_up().to_bits());
            }
            acc
        };
        assert_eq!(run(), run());
    }
}
