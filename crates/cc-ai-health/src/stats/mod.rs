//! Deterministic statistical primitives — the reusable core every health
//! algorithm is built from.
//!
//! # Why these, and why hand-rolled
//!
//! The algorithms run **online, unsupervised, on one vehicle with no labels**,
//! so the toolkit is change-point detection and robust estimation, not ML:
//!
//! * [`ewma::Ewma`] — exponentially-weighted mean + variance (Welford form): a
//!   cheap, order-deterministic robust-ish baseline.
//! * [`robust::RobustScale`] — rolling median + MAD (median absolute
//!   deviation): the outlier-resistant location/scale a z-score needs so a
//!   single spike never sets the baseline.
//! * [`cusum::Cusum`] — one/two-sided CUSUM: the optimal detector for a
//!   *sustained level shift* (the signature of a real fault) while rejecting
//!   single-sample transients.
//! * [`page_hinkley::PageHinkley`] — a change detector tuned for a drift in the
//!   mean with an explicit minimum-magnitude slack.
//! * [`rls::Rls3`] — 3-parameter recursive least squares with forgetting: fits
//!   the physics residual models (battery `V = OCV − I·R`, vibration vs
//!   throttle) online.
//!
//! # Determinism contract (the Phase-7 exit criterion)
//!
//! `cc-replay` must produce **byte-identical** findings for identical input.
//! Every primitive here honours:
//! * all reductions are `f64` accumulated in a **fixed order** — no parallel /
//!   tree reductions, no `fma`, no fast-math reassociation;
//! * ordering of floats uses [`f32::total_cmp`] / [`f64::total_cmp`] (a *total*
//!   order, NaN included) — never `partial_cmp`;
//! * no `HashMap`, no RNG, no wall-clock;
//! * transcendentals go through `libm` (bit-reproducible across x86-64 and
//!   aarch64), never `std`'s target intrinsics — see [`ln`];
//! * `NaN` is treated as **missing** and rejected at the primitive boundary, so
//!   a garbage sample degrades availability rather than poisoning a detector.

pub mod cusum;
pub mod ewma;
pub mod page_hinkley;
pub mod rls;
pub mod robust;

/// Clamp to `[0, 1]`. `NaN` maps to `0` (a missing/garbage statistic must not
/// inflate confidence).
#[inline]
pub fn clamp01(x: f64) -> f64 {
    if x.is_nan() {
        0.0
    } else {
        x.clamp(0.0, 1.0)
    }
}

/// Bit-reproducible natural log via `libm` (identical bits on x86-64 and
/// aarch64). `std::f64::ln` may lower to a target-specific intrinsic, which
/// would break the cross-arch golden-hash gate; `libm` is a fixed software
/// implementation. Guards the `x <= 0` domain to `NaN` (caller treats as
/// missing) rather than `-inf` propagating through a detector.
#[inline]
pub fn ln(x: f64) -> f64 {
    if x > 0.0 {
        libm::log(x)
    } else {
        f64::NAN
    }
}

/// Standard logistic `1/(1+e^-x)` via `libm::exp` (bit-reproducible). Used to
/// map a normalized detector statistic into a `[0,1]` confidence factor.
#[inline]
pub fn logistic(x: f64) -> f64 {
    // Guard the tails so exp() cannot overflow to inf and produce a NaN.
    if x >= 40.0 {
        1.0
    } else if x <= -40.0 {
        0.0
    } else {
        1.0 / (1.0 + libm::exp(-x))
    }
}

/// Quantize a confidence in `[0,1]` to an integer percent `0..=100` using
/// **round-half-to-even** (banker's rounding) implemented in scaled integers —
/// so there is no floating-point rounding-mode dependence in the emitted
/// `confidence_percent` byte.
///
/// `round(x*100)` with ties (…exactly .5) going to the nearest even integer.
pub fn confidence_percent(conf01: f64) -> u8 {
    let x = clamp01(conf01) * 100.0;
    // Work in thousandths to make the half-way test exact for our inputs.
    let milli = (x * 1000.0).round() as i64; // coarse; refine the tie below
    let base = milli / 1000; // integer part in percent
    let frac = milli - base * 1000; // 0..=999 (approx thousandths)
    let rounded = match frac.cmp(&500) {
        std::cmp::Ordering::Greater => base + 1,
        std::cmp::Ordering::Less => base,
        // exactly half → round to even
        std::cmp::Ordering::Equal if base % 2 == 0 => base,
        std::cmp::Ordering::Equal => base + 1,
    };
    rounded.clamp(0, 100) as u8
}

/// A total-order ascending sort of a slice of `f64` (NaN sorts last), for the
/// copy-then-sort median/MAD. Deterministic across platforms because
/// [`f64::total_cmp`] is a fixed total order.
pub fn sort_total(v: &mut [f64]) {
    v.sort_by(|a, b| a.total_cmp(b));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp01_handles_nan_and_range() {
        assert_eq!(clamp01(f64::NAN), 0.0);
        assert_eq!(clamp01(-3.0), 0.0);
        assert_eq!(clamp01(2.0), 1.0);
        assert_eq!(clamp01(0.5), 0.5);
    }

    #[test]
    fn ln_and_logistic_guard_domains() {
        assert!(ln(-1.0).is_nan());
        assert!(ln(0.0).is_nan());
        assert!((ln(1.0)).abs() < 1e-12);
        assert_eq!(logistic(100.0), 1.0);
        assert_eq!(logistic(-100.0), 0.0);
        assert!((logistic(0.0) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn confidence_round_half_even() {
        assert_eq!(confidence_percent(0.0), 0);
        assert_eq!(confidence_percent(1.0), 100);
        assert_eq!(confidence_percent(0.90), 90);
        assert_eq!(confidence_percent(0.905), 90); // .5 -> even (90)
        assert_eq!(confidence_percent(0.915), 92); // .5 -> even (92)
        assert_eq!(confidence_percent(0.9151), 92);
        assert_eq!(confidence_percent(1.5), 100); // clamped
    }

    #[test]
    fn sort_total_puts_nan_last_deterministically() {
        let mut v = vec![3.0, f64::NAN, 1.0, 2.0];
        sort_total(&mut v);
        assert_eq!(&v[..3], &[1.0, 2.0, 3.0]);
        assert!(v[3].is_nan());
    }
}
