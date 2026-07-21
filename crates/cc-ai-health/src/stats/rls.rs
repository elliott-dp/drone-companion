//! Fixed 3-parameter **recursive least squares with forgetting** — fits the
//! physics-residual models online (battery `v = θ₀ + θ₁·SoC − θ₂·I`, vibration
//! metric vs throttle) so an anomaly is a *residual from the vehicle's own
//! learned law*, not a raw threshold.
//!
//! Standard RLS with forgetting factor `λ ∈ (0,1]` (memory ≈ `1/(1−λ)`):
//! ```text
//! Pφ    = P·φ
//! K     = Pφ / (λ + φᵀ·Pφ)              (gain)
//! e     = y − φᵀ·θ                       (innovation / residual)
//! θ     = θ + K·e
//! P     = (P − K·(Pφ)ᵀ) / λ
//! ```
//!
//! Determinism: fixed 3×3 / 3-vector arithmetic in a pinned index order — no
//! dynamic sizing, no library BLAS, no reduction-order ambiguity. Ill-
//! conditioning (e.g. battery in steady hover where the current column barely
//! excites `θ₂`) is the **caller's** responsibility to detect via an excitation
//! gate and go DEGRADED — RLS itself just reports its innovation and a
//! conditioning proxy ([`Rls3::p_trace`]). NaN inputs are rejected.

/// 3-parameter RLS. `θ` are the fitted parameters; the caller supplies the
/// regressor `φ` (e.g. `[1, SoC, −I]`).
#[derive(Debug, Clone)]
pub struct Rls3 {
    theta: [f64; 3],
    p: [[f64; 3]; 3],
    lambda: f64,
    n: u64,
    frozen: bool,
}

impl Rls3 {
    /// `lambda` = forgetting factor (clamped to `(0,1]`); `p0` = initial
    /// covariance scale (large ⇒ fast initial adaptation, e.g. `1e3`).
    pub fn new(lambda: f64, p0: f64) -> Self {
        let l = if lambda.is_nan() { 0.995 } else { lambda.clamp(f64::MIN_POSITIVE, 1.0) };
        let p0 = if p0 > 0.0 { p0 } else { 1e3 };
        let mut p = [[0.0; 3]; 3];
        for (i, row) in p.iter_mut().enumerate() {
            row[i] = p0;
        }
        Self { theta: [0.0; 3], p, lambda: l, n: 0, frozen: false }
    }

    #[inline]
    fn dot(a: &[f64; 3], b: &[f64; 3]) -> f64 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    /// Fold in one observation `(φ, y)`. Returns the residual `e = y − φᵀθ`
    /// computed **before** the update (the prediction error). Returns `NaN` and
    /// does nothing if any input is NaN or the estimator is frozen.
    pub fn update(&mut self, phi: &[f64; 3], y: f64) -> f64 {
        if self.frozen || y.is_nan() || phi.iter().any(|v| v.is_nan()) {
            return f64::NAN;
        }
        // Pφ
        let mut pphi = [0.0; 3];
        for (i, pi) in pphi.iter_mut().enumerate() {
            *pi = self.p[i][0] * phi[0] + self.p[i][1] * phi[1] + self.p[i][2] * phi[2];
        }
        let denom = self.lambda + Self::dot(phi, &pphi);
        // guard a degenerate denom (shouldn't happen with p0>0, λ>0)
        if denom <= 0.0 || denom.is_nan() {
            return f64::NAN;
        }
        let gain = [pphi[0] / denom, pphi[1] / denom, pphi[2] / denom];
        let e = y - Self::dot(phi, &self.theta);
        for (t, g) in self.theta.iter_mut().zip(gain.iter()) {
            *t += g * e;
        }
        // P = (P − K·(Pφ)ᵀ) / λ  (fixed 3×3 in pinned index order)
        #[allow(clippy::needless_range_loop)]
        for i in 0..3 {
            for j in 0..3 {
                self.p[i][j] = (self.p[i][j] - gain[i] * pphi[j]) / self.lambda;
            }
        }
        self.n = self.n.saturating_add(1);
        e
    }

    /// Predict `φᵀθ` for a regressor.
    pub fn predict(&self, phi: &[f64; 3]) -> f64 {
        Self::dot(phi, &self.theta)
    }

    pub fn theta(&self) -> [f64; 3] {
        self.theta
    }
    pub fn count(&self) -> u64 {
        self.n
    }
    pub fn is_warm(&self, min_samples: u64) -> bool {
        self.n >= min_samples
    }
    /// Trace of `P` — a cheap conditioning proxy; a large trace after warmup
    /// means a parameter direction is poorly excited (caller may go DEGRADED).
    pub fn p_trace(&self) -> f64 {
        self.p[0][0] + self.p[1][1] + self.p[2][2]
    }
    pub fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
    }
    pub fn reset(&mut self, p0: f64) {
        *self = Rls3::new(self.lambda, p0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_known_linear_law() {
        // y = 3 + 2*a - 0.5*b ; regressor [1, a, b]
        let mut rls = Rls3::new(1.0, 1e3);
        for i in 0..500 {
            let a = ((i * 7) % 11) as f64;
            let b = ((i * 5) % 13) as f64;
            let y = 3.0 + 2.0 * a - 0.5 * b;
            rls.update(&[1.0, a, b], y);
        }
        // RLS with λ=1 is growing-memory LS; after 500 samples it has
        // converged to the true law to within the p0-transient residual.
        let t = rls.theta();
        assert!((t[0] - 3.0).abs() < 1e-3, "theta0 {}", t[0]);
        assert!((t[1] - 2.0).abs() < 1e-3, "theta1 {}", t[1]);
        assert!((t[2] + 0.5).abs() < 1e-3, "theta2 {}", t[2]);
    }

    #[test]
    fn residual_flags_a_law_violation() {
        let mut rls = Rls3::new(0.99, 1e3);
        for i in 0..300 {
            let i_cur = 5.0 + ((i % 7) as f64); // varied current => excited
            let y = 12.0 - 0.1 * i_cur; // V = 12 - R*I, R=0.1
            rls.update(&[1.0, 0.0, i_cur], y);
        }
        // a sudden extra sag (as if R doubled) shows as a large negative residual
        let e = rls.update(&[1.0, 0.0, 10.0], 12.0 - 0.2 * 10.0);
        assert!(e < -0.5, "sag residual should be strongly negative, got {e}");
    }

    #[test]
    fn nan_input_is_rejected() {
        let mut rls = Rls3::new(0.99, 1e3);
        rls.update(&[1.0, 1.0, 1.0], 5.0);
        let n = rls.count();
        assert!(rls.update(&[1.0, f64::NAN, 1.0], 5.0).is_nan());
        assert!(rls.update(&[1.0, 1.0, 1.0], f64::NAN).is_nan());
        assert_eq!(rls.count(), n);
    }

    #[test]
    fn deterministic_repeat() {
        let run = || {
            let mut rls = Rls3::new(0.997, 500.0);
            for i in 0..600 {
                let a = ((i as f64) * 0.01).sin();
                rls.update(&[1.0, a, a * a], 2.0 + a);
            }
            rls.theta().map(|x| x.to_bits())
        };
        assert_eq!(run(), run());
    }
}
