//! **Page-Hinkley** change detector — a drift-in-mean test with an explicit
//! minimum-magnitude slack `δ`.
//!
//! Complementary to CUSUM: where CUSUM watches a shift vs a *fixed target*,
//! Page-Hinkley tracks the running mean itself and flags when the cumulative
//! deviation from it (net of `δ`) departs by more than `λ`. Used on
//! residual-style signals (battery sag `z`, vibration residual) where "how far
//! has this drifted from where it settled" is the question.
//!
//! Upward variant (detecting an increase):
//! ```text
//! μ_k  = running mean of x
//! m_k  = m_{k−1} + (x_k − μ_k − δ)
//! M_k  = min_j m_j
//! PH_k = m_k − M_k            trip when PH_k > λ
//! ```
//! Downward is symmetric (`μ_k − x_k − δ`, track the max, `PH = max − m`).
//!
//! `δ` (magnitude slack) sets specificity: only drifts larger than `δ` per
//! sample accumulate, so benign small wander never trips. Determinism: fixed
//! scalar recurrence; NaN ignored; the running mean is a plain incremental
//! average (fixed order).

/// Direction the detector watches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

#[derive(Debug, Clone)]
pub struct PageHinkley {
    dir: Direction,
    delta: f64,
    lambda: f64,
    n: u64,
    mean: f64,
    m: f64,
    extreme: f64, // min (Up) or max (Down) of m
    frozen: bool,
}

impl PageHinkley {
    /// `delta` = per-sample magnitude slack, `lambda` = decision threshold.
    pub fn new(dir: Direction, delta: f64, lambda: f64) -> Self {
        Self {
            dir,
            delta: delta.max(0.0),
            lambda: if lambda > 0.0 { lambda } else { 1.0 },
            n: 0,
            mean: 0.0,
            m: 0.0,
            extreme: 0.0,
            frozen: false,
        }
    }

    /// Fold in a sample; returns `true` if currently tripped. NaN / frozen are
    /// ignored.
    pub fn update(&mut self, x: f64) -> bool {
        if self.frozen || x.is_nan() {
            return self.tripped();
        }
        self.n += 1;
        // incremental running mean (fixed order)
        self.mean += (x - self.mean) / self.n as f64;
        // Symmetric CUSUM-of-mean-deviation. Up subtracts the slack so a steady
        // signal drifts `m` DOWN (extreme = running min, stat = m − min);
        // Down adds the slack so a steady signal drifts `m` UP (extreme =
        // running max, stat = max − m). Either way a steady input yields
        // stat ≈ 0 and only a genuine shift in the watched direction grows it.
        let contrib = match self.dir {
            Direction::Up => x - self.mean - self.delta,
            Direction::Down => x - self.mean + self.delta,
        };
        self.m += contrib;
        if self.n == 1 {
            self.extreme = self.m;
        } else {
            match self.dir {
                Direction::Up => {
                    if self.m < self.extreme {
                        self.extreme = self.m;
                    }
                }
                Direction::Down => {
                    if self.m > self.extreme {
                        self.extreme = self.m;
                    }
                }
            }
        }
        self.tripped()
    }

    /// The Page-Hinkley statistic `PH = m − min m` (Up) / `max m − m` (Down).
    pub fn stat(&self) -> f64 {
        match self.dir {
            Direction::Up => self.m - self.extreme,
            Direction::Down => self.extreme - self.m,
        }
    }

    pub fn tripped(&self) -> bool {
        self.stat() > self.lambda
    }

    /// Normalized excess over threshold `[0, ∞)`.
    pub fn excess(&self) -> f64 {
        ((self.stat() - self.lambda) / self.lambda).max(0.0)
    }

    pub fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
    }

    pub fn reset(&mut self) {
        self.n = 0;
        self.mean = 0.0;
        self.m = 0.0;
        self.extreme = 0.0;
        self.frozen = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upward_drift_trips() {
        let mut ph = PageHinkley::new(Direction::Up, 0.1, 3.0);
        // benign: zero-mean small noise, no trip
        for i in 0..300 {
            let x = if i % 2 == 0 { 0.2 } else { -0.2 };
            ph.update(x);
        }
        assert!(!ph.tripped());
        // sustained positive drift accumulates and trips
        let mut tripped = false;
        for _ in 0..100 {
            if ph.update(1.0) {
                tripped = true;
                break;
            }
        }
        assert!(tripped);
        assert!(ph.excess() >= 0.0);
    }

    #[test]
    fn downward_drift_trips() {
        // establish a steady baseline, then step the mean DOWN
        let mut ph = PageHinkley::new(Direction::Down, 0.1, 3.0);
        for _ in 0..50 {
            assert!(!ph.update(0.0), "steady baseline must not trip");
        }
        let mut tripped = false;
        for _ in 0..100 {
            if ph.update(-1.0) {
                tripped = true;
                break;
            }
        }
        assert!(tripped, "a sustained downward shift must trip");
    }

    #[test]
    fn steady_signal_never_trips_either_direction() {
        // regression: a constant (or slowly varying) input must NOT trip a
        // one-sided detector — the inverted-Down bug did exactly this.
        for dir in [Direction::Up, Direction::Down] {
            let mut ph = PageHinkley::new(dir, 0.1, 3.0);
            for i in 0..500 {
                // constant level with bounded (non-accumulating) noise
                let x = 5.0 + if i % 2 == 0 { 0.05 } else { -0.05 };
                assert!(!ph.update(x), "steady input tripped {dir:?}");
            }
        }
    }

    #[test]
    fn single_spike_does_not_trip() {
        let mut ph = PageHinkley::new(Direction::Up, 0.5, 5.0);
        for _ in 0..100 {
            ph.update(0.0);
        }
        ph.update(3.0); // one spike, below lambda net of the running mean
        assert!(!ph.tripped());
    }

    #[test]
    fn deterministic_repeat() {
        let run = || {
            let mut ph = PageHinkley::new(Direction::Up, 0.05, 4.0);
            let mut acc = 0u64;
            for i in 0..777 {
                ph.update(((i as f64) * 0.013).cos());
                acc = acc.wrapping_add(ph.stat().to_bits());
            }
            acc
        };
        assert_eq!(run(), run());
    }
}
