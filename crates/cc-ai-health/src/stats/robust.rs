//! Rolling **median + MAD** (median absolute deviation) — the outlier-resistant
//! location/scale a z-score needs.
//!
//! A single spike must not move the baseline (unlike a mean/std), because the
//! spike is exactly what we are trying to detect. `z = (x − median) / (k·MAD)`
//! with `k = 1.4826` (so `k·MAD` ≈ σ for Gaussian data). The classic hazard the
//! adversarial review flagged: on **quantized / constant** data more than half
//! the window lands in one value → `MAD = 0` → `z = ±∞`. Guarded two ways here:
//! an ε **floor** on the scale (so `z` is finite) *and* the algorithms always
//! pair this relative detector with an absolute physical backstop, so a
//! degenerate scale can never by itself drive a finding.
//!
//! Determinism: the median/MAD are computed by **copy → total-order sort**
//! ([`f64::total_cmp`], a fixed total order incl. NaN), so the reduction is
//! order-independent of how the ring was filled. `NaN` samples are treated as
//! **missing** — never inserted — so the window holds only real observations.

use super::sort_total;

/// Fixed-capacity rolling window with median/MAD.
#[derive(Debug, Clone)]
pub struct RobustScale {
    buf: Vec<f64>,
    cap: usize,
    head: usize,
    len: usize,
    /// ε floor on `k·MAD` so a degenerate (quantized/constant) window yields a
    /// finite z instead of ±∞.
    scale_floor: f64,
    frozen: bool,
}

impl RobustScale {
    /// `capacity` samples; `scale_floor` is the minimum of `1.4826·MAD` used in
    /// the z denominator (choose ≈ the field's quantization step).
    pub fn new(capacity: usize, scale_floor: f64) -> Self {
        let cap = capacity.max(1);
        Self {
            buf: vec![0.0; cap],
            cap,
            head: 0,
            len: 0,
            scale_floor: if scale_floor > 0.0 { scale_floor } else { 1e-9 },
            frozen: false,
        }
    }

    /// Insert a sample (ignored if NaN or frozen).
    pub fn update(&mut self, x: f64) {
        if self.frozen || x.is_nan() {
            return;
        }
        self.buf[self.head] = x;
        self.head = (self.head + 1) % self.cap;
        if self.len < self.cap {
            self.len += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn is_warm(&self, min_samples: usize) -> bool {
        self.len >= min_samples
    }
    pub fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
    }
    pub fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.frozen = false;
    }

    fn sorted(&self) -> Vec<f64> {
        let mut v: Vec<f64> = self.buf[..self.len].to_vec();
        sort_total(&mut v);
        v
    }

    fn median_of(sorted: &[f64]) -> f64 {
        let n = sorted.len();
        if n == 0 {
            return f64::NAN;
        }
        if n % 2 == 1 {
            sorted[n / 2]
        } else {
            0.5 * (sorted[n / 2 - 1] + sorted[n / 2])
        }
    }

    pub fn median(&self) -> f64 {
        Self::median_of(&self.sorted())
    }

    /// Median absolute deviation (raw, not scaled by 1.4826).
    pub fn mad(&self) -> f64 {
        let s = self.sorted();
        let med = Self::median_of(&s);
        if med.is_nan() {
            return f64::NAN;
        }
        let mut dev: Vec<f64> = s.iter().map(|x| (x - med).abs()).collect();
        sort_total(&mut dev);
        Self::median_of(&dev)
    }

    /// Robust scale estimate `max(1.4826·MAD, scale_floor)`.
    pub fn scale(&self) -> f64 {
        let s = 1.4826 * self.mad();
        if s.is_nan() {
            self.scale_floor
        } else {
            s.max(self.scale_floor)
        }
    }

    /// Robust z-score of `x` against the current window. `NaN` if the window is
    /// empty.
    pub fn z(&self, x: f64) -> f64 {
        let med = self.median();
        if med.is_nan() {
            return f64::NAN;
        }
        (x - med) / self.scale()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_and_mad_basic() {
        let mut r = RobustScale::new(16, 1e-6);
        for x in [1.0, 2.0, 3.0, 4.0, 5.0] {
            r.update(x);
        }
        assert_eq!(r.median(), 3.0);
        // deviations {2,1,0,1,2} → MAD = 1
        assert!((r.mad() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn outlier_does_not_move_median_much() {
        let mut r = RobustScale::new(64, 1e-6);
        for _ in 0..50 {
            r.update(10.0);
        }
        r.update(1000.0); // one spike
        assert!((r.median() - 10.0).abs() < 1e-9, "median robust to a single spike");
        // the spike is a large z
        assert!(r.z(1000.0) > 100.0);
    }

    #[test]
    fn constant_window_uses_scale_floor_not_infinity() {
        let mut r = RobustScale::new(32, 0.5);
        for _ in 0..32 {
            r.update(7.0); // MAD = 0
        }
        assert_eq!(r.mad(), 0.0);
        assert_eq!(r.scale(), 0.5); // floored, not zero
        assert!(r.z(9.0).is_finite());
        assert!((r.z(9.0) - 4.0).abs() < 1e-9); // (9-7)/0.5
    }

    #[test]
    fn nan_not_inserted() {
        let mut r = RobustScale::new(8, 1e-6);
        r.update(1.0);
        r.update(f64::NAN);
        r.update(3.0);
        assert_eq!(r.len(), 2);
        assert_eq!(r.median(), 2.0);
    }

    #[test]
    fn ring_wraps_and_stays_deterministic() {
        let run = || {
            let mut r = RobustScale::new(10, 1e-6);
            for i in 0..1000 {
                r.update((i % 13) as f64);
            }
            (r.median().to_bits(), r.scale().to_bits())
        };
        assert_eq!(run(), run());
    }
}
