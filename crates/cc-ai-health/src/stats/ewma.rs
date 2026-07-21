//! Exponentially-weighted moving average with incremental variance.
//!
//! `m_k = α·x_k + (1−α)·m_{k−1}`, and the exponentially-weighted variance via
//! West's incremental update:
//!
//! ```text
//! diff = x − m_prev
//! m    = m_prev + α·diff
//! var  = (1−α)·(var_prev + diff·(α·diff))
//! ```
//!
//! `α ∈ (0,1]` sets the memory: the effective window is ≈ `1/α` samples, or a
//! time constant `τ ≈ Δt/α`. A small `α` (long memory) is used for *baselines*
//! that must not chase an anomaly; a larger `α` for reactive rate/derivative
//! smoothing.
//!
//! Determinism: a single fixed-order `f64` recurrence — no reduction order to
//! get wrong. `NaN` inputs are rejected (treated as missing) so a garbage
//! sample cannot poison the baseline.

/// EWMA location + scale.
#[derive(Debug, Clone)]
pub struct Ewma {
    alpha: f64,
    mean: f64,
    var: f64,
    count: u64,
    /// When `true`, [`Ewma::update`] is a no-op — used to **freeze** a baseline
    /// while a finding is active so an anomaly can never be absorbed into it.
    frozen: bool,
}

impl Ewma {
    /// `alpha` is clamped to `(0, 1]`.
    pub fn new(alpha: f64) -> Self {
        let a = if alpha.is_nan() { 0.1 } else { alpha.clamp(f64::MIN_POSITIVE, 1.0) };
        Self { alpha: a, mean: 0.0, var: 0.0, count: 0, frozen: false }
    }

    /// Fold in a sample. Ignored if `x` is NaN or the estimator is frozen.
    pub fn update(&mut self, x: f64) {
        if self.frozen || x.is_nan() {
            return;
        }
        if self.count == 0 {
            self.mean = x;
            self.var = 0.0;
        } else {
            let diff = x - self.mean;
            let incr = self.alpha * diff;
            self.mean += incr;
            self.var = (1.0 - self.alpha) * (self.var + diff * incr);
        }
        self.count = self.count.saturating_add(1);
    }

    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// Exponentially-weighted variance (`>= 0`).
    pub fn var(&self) -> f64 {
        if self.var < 0.0 {
            0.0
        } else {
            self.var
        }
    }

    /// Exponentially-weighted standard deviation.
    pub fn std(&self) -> f64 {
        libm::sqrt(self.var())
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn is_warm(&self, min_samples: u64) -> bool {
        self.count >= min_samples
    }

    /// Freeze / unfreeze baseline adaptation (anti-masking: freeze while a
    /// finding is active or while disarmed).
    pub fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
    }

    pub fn reset(&mut self) {
        self.mean = 0.0;
        self.var = 0.0;
        self.count = 0;
        self.frozen = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converges_to_constant() {
        let mut e = Ewma::new(0.2);
        for _ in 0..200 {
            e.update(5.0);
        }
        assert!((e.mean() - 5.0).abs() < 1e-9);
        assert!(e.std() < 1e-6);
        assert!(e.is_warm(100));
    }

    #[test]
    fn tracks_variance_of_alternating() {
        let mut e = Ewma::new(0.1);
        for i in 0..1000 {
            e.update(if i % 2 == 0 { 1.0 } else { -1.0 });
        }
        // mean near 0, positive variance
        assert!(e.mean().abs() < 0.2);
        assert!(e.std() > 0.3);
    }

    #[test]
    fn nan_is_ignored() {
        let mut e = Ewma::new(0.3);
        e.update(2.0);
        e.update(f64::NAN);
        assert_eq!(e.count(), 1);
        assert_eq!(e.mean(), 2.0);
    }

    #[test]
    fn freeze_stops_adaptation() {
        let mut e = Ewma::new(0.5);
        for _ in 0..50 {
            e.update(1.0);
        }
        let m = e.mean();
        e.set_frozen(true);
        for _ in 0..50 {
            e.update(100.0); // anomaly — must not move the frozen baseline
        }
        assert!((e.mean() - m).abs() < 1e-9);
    }

    #[test]
    fn deterministic_repeat() {
        let run = || {
            let mut e = Ewma::new(0.17);
            for i in 0..500 {
                e.update((i as f64).sin());
            }
            (e.mean().to_bits(), e.var().to_bits())
        };
        assert_eq!(run(), run());
    }
}
