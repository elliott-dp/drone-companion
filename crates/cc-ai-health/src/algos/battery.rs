//! `battery_model` (health-flag bit 1) — the highest-value lane.
//!
//! # Method — a physics model, so healthy discharge is *predicted, not flagged*
//!
//! A naive "voltage low → warn" battery monitor false-positives on every hard
//! climb (sag under load) and every cold pack (high internal resistance), and
//! false-negatives a genuinely weak cell that still reads "not low yet". We
//! instead carry the **electrochemical model** the FC's simple thresholds don't:
//!
//! ```text
//!   v_cell = OCV(SoC) − I · R_int(SoC, T)
//! ```
//!
//! * **`OCV(SoC)`** — open-circuit voltage as a *nonlinear* function of state of
//!   charge, from a per-cell LiPo lookup table. The table captures the
//!   end-of-discharge **knee below ~20 % SoC**, so the steep terminal sag a
//!   healthy pack shows near empty is *expected by the model* rather than raised
//!   as an anomaly (adversarial-panel fix — a linear model flags every normal
//!   landing).
//! * **`R_int(SoC, T)`** — internal resistance with an **Arrhenius** temperature
//!   term: `R = R_ref · exp(B·(1/T − 1/T_ref)) · soc_factor`. A cold pack
//!   legitimately shows ~2× resistance at 0 °C; the model predicts that extra
//!   sag instead of flagging it. `soc_factor` lifts R near empty (real LiPo
//!   behaviour).
//!
//! The measured cell voltage is compared to this *load- and temperature-aware*
//! prediction, so only sag **beyond what the model explains** is anomalous.
//!
//! # Five detectors (each maps to a distinct detail code)
//!
//! 1. **Sag beyond model** — residual `r = v_cell − v_pred`; robust
//!    z-score `z = r / (1.4826·MAD)`; a **one-sided Page–Hinkley on downward z**
//!    catches a slow persistent under-prediction (a tiring cell) that a single
//!    threshold misses. `WARN`.
//! 2. **Internal-resistance growth** — measured `R̂ = (OCV − v_cell)/I` under
//!    load; a **one-sided CUSUM on `R̂/R_model`** flags a pack whose resistance
//!    is climbing faster than the temperature model accounts for (ageing / a
//!    failing cell). `WARN`.
//! 3. **Imminent brownout** — project `v_cell` at the *sustained hover current*
//!    using `R̂`; if it falls below the `3.3 V/cell` floor the pack cannot hold
//!    the vehicle up much longer. `CRITICAL → Land`.
//! 4. **Gauge self-consistency** — `consumed_mah` must be non-decreasing and
//!    `remaining` non-increasing within a boot; a gauge that jumps backwards is
//!    itself a fault (and would corrupt SoC-based reasoning). `WARN`.
//! 5. **Ground-truth PX4 warning echo** — PX4's own `battery_status.warning`
//!    (LOW=1, CRITICAL=2, EMERGENCY=3) is a high-specificity oracle; we mirror
//!    it (LOW → WARN, ≥CRITICAL → `CRITICAL → Land`). Absolute
//!    undervoltage-under-load (`v_cell < 3.3 V` while `I` is high) is the same
//!    tier.
//!
//! # Observability self-gate (adversarial-panel fix)
//!
//! `R̂` is only identifiable when the current *varies*: steady hover makes the
//! regression `[1, I]` near-collinear and `R̂` explodes on noise. The lane
//! therefore **self-reports `Degraded(low_excitation)`** whenever `var(I)` is
//! below a conditioning floor, instead of emitting a resistance finding it
//! cannot trust. The absolute/undervoltage/gauge/PX4-warning detectors still run
//! (they don't need excitation).
//!
//! # False-positive guards
//! `I > I_min` before any R-based detector · model absorbs load current ·
//! freeze-on-anomaly · 20 s **and** ≥50 high-current samples of warm-up ·
//! `connected == 0` or `remaining == NaN` → the lane is `Unavailable`/`Degraded`,
//! never a fault.

use crate::finding::{flags, AlgoOutput, HealthFinding};
use crate::stats::{clamp01, confidence_percent, logistic};
use crate::stats::{Cusum, CusumTrip, Direction, Ewma, PageHinkley, RobustScale};
use crate::{detail, EvalCtx, HealthAlgorithm};
use cc_health_tx::{Action, Severity};
use cc_ingest::{StreamId, TelemetryEvent};
use cc_protocol::cc_dialect::CcSubsystem;

// ---- physical constants (per cell) -----------------------------------------
const CELL_FLOOR_V: f64 = 3.30; // absolute usable floor
const R_REF_OHM: f64 = 0.020; // internal resistance at 25 °C, mid-SoC
const T_REF_K: f64 = 298.15;
const ARRHENIUS_B: f64 = 2000.0; // K; sets the cold-pack R rise
const I_MIN_A: f64 = 2.0; // load floor for R identification
const VAR_I_FLOOR: f64 = 0.25; // A² — current-excitation floor for R
const WARMUP_NS: i64 = 20_000_000_000; // 20 s
const WARMUP_HI_I: u32 = 50; // high-current samples before any WARN

/// Per-cell resting OCV vs SoC (descending SoC). Typical LiPo; the sub-20 %
/// entries encode the terminal knee so normal end-of-discharge is predicted.
const OCV_LUT: [(f64, f64); 12] = [
    (1.00, 4.20),
    (0.90, 4.05),
    (0.80, 3.95),
    (0.70, 3.87),
    (0.60, 3.81),
    (0.50, 3.77),
    (0.40, 3.74),
    (0.30, 3.70),
    (0.20, 3.64),
    (0.10, 3.52),
    (0.05, 3.40),
    (0.00, 3.30),
];

/// Open-circuit voltage for a state of charge, by linear interpolation on the
/// nonlinear LUT (clamped to `[0,1]`).
fn ocv(soc: f64) -> f64 {
    let s = clamp01(soc);
    let mut hi = 0;
    while hi + 1 < OCV_LUT.len() && OCV_LUT[hi + 1].0 > s {
        hi += 1;
    }
    let (s_hi, v_hi) = OCV_LUT[hi];
    let (s_lo, v_lo) = OCV_LUT[(hi + 1).min(OCV_LUT.len() - 1)];
    let span = s_hi - s_lo;
    if span.abs() < 1e-9 {
        return v_hi;
    }
    let t = (s - s_lo) / span;
    v_lo + t * (v_hi - v_lo)
}

/// Temperature multiplier on internal resistance (cold ⇒ higher R).
fn arrhenius(temp_c: f64) -> f64 {
    let t = if temp_c.is_nan() { 25.0 } else { temp_c } + 273.15;
    libm::exp(ARRHENIUS_B * (1.0 / t - 1.0 / T_REF_K))
}

/// Modelled internal resistance at a given SoC and temperature.
fn r_int_model(soc: f64, temp_c: f64) -> f64 {
    let soc_factor = 1.0 + 0.5 * (0.2 - clamp01(soc)).max(0.0) / 0.2;
    R_REF_OHM * arrhenius(temp_c) * soc_factor
}

/// Latched, prioritised battery verdict (worst wins). Latched until `reset`
/// (a degraded pack stays degraded for the flight).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Verdict {
    Ok,
    SagWarn,
    RGrowthWarn,
    GaugeWarn,
    Px4LowWarn,
    UndervoltCrit,
    BrownoutCrit,
    Px4CritCrit,
}
impl Verdict {
    fn rank(self) -> u8 {
        match self {
            Verdict::Ok => 0,
            Verdict::GaugeWarn => 1,
            Verdict::Px4LowWarn => 2,
            Verdict::RGrowthWarn => 3,
            Verdict::SagWarn => 4,
            Verdict::UndervoltCrit => 5,
            Verdict::BrownoutCrit => 6,
            Verdict::Px4CritCrit => 7,
        }
    }
}

pub struct BatteryModel {
    // detectors
    res: RobustScale,     // residual r = v_cell − v_pred
    ph_sag: PageHinkley,  // downward drift of robust z(r)
    cus_r: Cusum,         // upward drift of R̂/R_model
    i_stats: Ewma,        // current mean (hover load) + variance (excitation)
    // warm-up / gating
    first_ns: Option<i64>,
    last_ns: i64,
    hi_i_count: u32,
    connected: bool,
    nan_seen: bool,
    low_excitation: bool,
    // gauge monotonicity
    last_consumed: Option<f32>,
    last_remaining: Option<f32>,
    // latched worst verdict + its evidence for the report
    verdict: Verdict,
    value: f32,
    limit: f32,
    confidence: u8,
    frozen: bool,
}

impl Default for BatteryModel {
    fn default() -> Self {
        Self::new()
    }
}

impl BatteryModel {
    pub fn new() -> Self {
        Self {
            res: RobustScale::new(128, 0.005), // 5 mV MAD floor
            ph_sag: PageHinkley::new(Direction::Down, 0.5, 6.0),
            cus_r: Cusum::new(0.5, 6.0),
            i_stats: Ewma::new(0.02),
            first_ns: None,
            last_ns: 0,
            hi_i_count: 0,
            connected: true,
            nan_seen: false,
            low_excitation: true,
            last_consumed: None,
            last_remaining: None,
            verdict: Verdict::Ok,
            value: 0.0,
            limit: 0.0,
            confidence: 0,
            frozen: false,
        }
    }

    fn warm(&self, now_ns: i64) -> bool {
        self.first_ns
            .is_some_and(|f0| now_ns.saturating_sub(f0) >= WARMUP_NS)
            && self.hi_i_count >= WARMUP_HI_I
    }

    /// Latch a verdict only if it outranks the current one; record evidence.
    fn raise(&mut self, v: Verdict, value: f64, limit: f64, conf01: f64) {
        if v.rank() > self.verdict.rank() {
            self.verdict = v;
            self.value = value as f32;
            self.limit = limit as f32;
            self.confidence = confidence_percent(conf01);
        }
    }

    fn freeze(&mut self) {
        self.frozen = true;
        self.res.set_frozen(true);
        self.ph_sag.set_frozen(true);
        self.cus_r.set_frozen(true);
    }
}

impl HealthAlgorithm for BatteryModel {
    fn subsystem(&self) -> CcSubsystem {
        CcSubsystem::CC_SUBSYS_BATTERY
    }

    fn on_event(&mut self, ev: &TelemetryEvent, _phase: crate::phase::FlightPhase) {
        let TelemetryEvent::Power(d, m) = ev else {
            return;
        };
        let cc_ns = m.cc_receive_time_ns;

        if d.connected == 0 {
            self.connected = false;
            return;
        }
        self.connected = true;

        if d.remaining.is_nan() || d.voltage.is_nan() || d.current.is_nan() {
            self.nan_seen = true;
            return;
        }
        self.nan_seen = false;

        let cells = d.cell_count.max(1) as f64;
        let v_cell = d.voltage as f64 / cells;
        let i = d.current as f64;
        let soc = clamp01(d.remaining as f64);
        let temp_c = d.temperature as f64;

        self.first_ns.get_or_insert(cc_ns);
        self.last_ns = cc_ns;
        self.i_stats.update(i);
        if i > I_MIN_A {
            self.hi_i_count = self.hi_i_count.saturating_add(1);
        }
        self.low_excitation = self.i_stats.var() < VAR_I_FLOOR;

        // (5) PX4 ground-truth warning echo + absolute undervoltage under load
        match d.warning {
            1 => self.raise(Verdict::Px4LowWarn, d.warning as f64, 1.0, 0.9),
            w if w >= 2 => self.raise(Verdict::Px4CritCrit, w as f64, 2.0, 0.98),
            _ => {}
        }
        // Absolute usable-floor backstop only. A shallower "warn" band here
        // would fire on healthy high-load sag (that is precisely what the model
        // residual detector is for), so the sole absolute rule is the 3.3 V
        // floor → CRITICAL.
        if i > I_MIN_A && v_cell < CELL_FLOOR_V {
            self.raise(Verdict::UndervoltCrit, v_cell, CELL_FLOOR_V, 0.97);
        }

        // (4) gauge monotonicity within a boot
        if let Some(pc) = self.last_consumed {
            if d.consumed_mah + 1.0 < pc {
                self.raise(Verdict::GaugeWarn, d.consumed_mah as f64, pc as f64, 0.85);
            }
        }
        if let Some(pr) = self.last_remaining {
            if d.remaining > pr + 0.05 {
                self.raise(Verdict::GaugeWarn, d.remaining as f64, pr as f64, 0.85);
            }
        }
        self.last_consumed = Some(d.consumed_mah);
        self.last_remaining = Some(d.remaining);

        // model-based detectors: warm, loaded, excited, not frozen
        let v_pred = ocv(soc) - i * r_int_model(soc, temp_c);
        let r = v_cell - v_pred;
        if self.warm(cc_ns) && i > I_MIN_A && !self.low_excitation && !self.frozen {
            // (1) sag beyond model
            self.res.update(r);
            if self.res.is_warm(32) {
                let z = self.res.z(r);
                if self.ph_sag.update(z) {
                    let conf = clamp01(0.7 + 0.3 * logistic(self.ph_sag.excess()));
                    self.raise(Verdict::SagWarn, v_cell, v_pred, conf);
                    self.freeze();
                }
            }
            // (2) internal-resistance growth
            let r_model = r_int_model(soc, temp_c);
            let r_hat = (ocv(soc) - v_cell) / i;
            if r_hat.is_finite() && r_model > 0.0 {
                let ratio = r_hat / r_model;
                if self.cus_r.update(ratio, 1.0) == CusumTrip::Up {
                    let conf = clamp01(0.7 + 0.3 * logistic(self.cus_r.excess()));
                    self.raise(Verdict::RGrowthWarn, r_hat, r_model, conf);
                    self.freeze();
                }
                // (3) imminent brownout at sustained hover current
                let i_hover = self.i_stats.mean();
                let v_hover = ocv(soc) - i_hover * r_hat.max(r_model);
                if v_hover < CELL_FLOOR_V {
                    self.raise(Verdict::BrownoutCrit, v_hover, CELL_FLOOR_V, 0.9);
                }
            }
        }
    }

    fn evaluate(&self, ctx: &EvalCtx) -> AlgoOutput {
        if !super::fresh(ctx, StreamId::Power) {
            return AlgoOutput::Unavailable(detail::AVAIL_STREAM_STALE);
        }
        if !self.connected {
            return AlgoOutput::Unavailable(detail::AVAIL_NO_DATA);
        }
        if self.nan_seen {
            return AlgoOutput::Degraded(detail::AVAIL_NAN_INPUT);
        }
        if !self.warm(ctx.now_ns) {
            return AlgoOutput::Degraded(detail::AVAIL_WARMUP);
        }

        let (severity, action, code) = match self.verdict {
            Verdict::Ok => {
                // healthy data, but if R is unidentifiable say so (Degraded)
                if self.low_excitation {
                    return AlgoOutput::Degraded(detail::AVAIL_LOW_EXCITATION);
                }
                (Severity::Ok, Action::None, 0)
            }
            Verdict::SagWarn => (Severity::Warn, Action::WarnOnly, detail::BATT_SAG_BEYOND_MODEL),
            Verdict::RGrowthWarn => (Severity::Warn, Action::WarnOnly, detail::BATT_R_INT_STEP),
            Verdict::GaugeWarn => {
                (Severity::Warn, Action::WarnOnly, detail::BATT_GAUGE_NONMONOTONIC)
            }
            Verdict::Px4LowWarn => (Severity::Warn, Action::WarnOnly, detail::BATT_PX4_WARNING),
            Verdict::UndervoltCrit => {
                (Severity::Critical, Action::Land, detail::BATT_UNDERVOLTAGE_UNDER_LOAD)
            }
            Verdict::BrownoutCrit => {
                (Severity::Critical, Action::Land, detail::BATT_UNDERVOLTAGE_UNDER_LOAD)
            }
            Verdict::Px4CritCrit => (Severity::Critical, Action::Land, detail::BATT_PX4_WARNING),
        };

        AlgoOutput::Available(HealthFinding {
            subsystem: CcSubsystem::CC_SUBSYS_BATTERY,
            flag_bit: flags::BATTERY,
            severity,
            action,
            detail_code: code,
            value: self.value,
            limit: self.limit,
            confidence: if severity == Severity::Ok { 100 } else { self.confidence },
        })
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::AlgoOutput;
    use cc_ingest::{AgeInfo, RxMeta};
    use cc_protocol::cc_dialect::CC_TELEMETRY_POWER_DATA;

    fn ctx(now_ns: i64) -> EvalCtx {
        EvalCtx {
            now_ns,
            phase: crate::phase::FlightPhase::Steady,
            last_seen_ns: [now_ns; 8],
            timesync_locked: true,
        }
    }

    fn power(cc_ns: i64, voltage: f32, current: f32, remaining: f32, warning: u8) -> TelemetryEvent {
        let d = CC_TELEMETRY_POWER_DATA {
            fc_timestamp_us: cc_ns as u64 / 1000,
            sequence: 0,
            voltage,
            current,
            power: voltage * current,
            consumed_mah: (1.0 - remaining) * 5000.0,
            remaining,
            temperature: 25.0,
            cell_count: 4,
            warning,
            connected: 1,
            schema_version: 1,
        };
        TelemetryEvent::Power(
            d,
            RxMeta { cc_receive_time_ns: cc_ns, seq_gap: 0, age: AgeInfo::Locked { age_ns: 1 } },
        )
    }

    // Healthy pack: cell voltage tracks the model under a varying (excited)
    // current. Expect NO finding once warm.
    fn feed_healthy(b: &mut BatteryModel, n: usize, t0: i64) -> i64 {
        let mut t = t0;
        for i in 0..n {
            let soc = 0.8 - 0.3 * (i as f32 / n as f32);
            // vary current 8..16 A to keep var(I) above the excitation floor
            let cur = if i % 2 == 0 { 8.0 } else { 16.0 };
            let cells = 4.0;
            let v_pred = ocv(soc as f64) - cur as f64 * r_int_model(soc as f64, 25.0);
            let voltage = (v_pred as f32) * cells;
            b.on_event(&power(t, voltage, cur, soc, 0), FlightPhase::Steady);
            t += 100_000_000;
        }
        t
    }
    use crate::phase::FlightPhase;

    #[test]
    fn benign_discharge_no_finding() {
        let mut b = BatteryModel::new();
        let t = feed_healthy(&mut b, 400, 0);
        match b.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Ok, "healthy pack must not warn");
            }
            other => panic!("expected Available(Ok), got {other:?}"),
        }
    }

    #[test]
    fn warmup_is_degraded() {
        let mut b = BatteryModel::new();
        b.on_event(&power(0, 15.8, 10.0, 0.8, 0), FlightPhase::Steady);
        assert!(matches!(
            b.evaluate(&ctx(1_000_000_000)),
            AlgoOutput::Degraded(detail::AVAIL_WARMUP)
        ));
    }

    #[test]
    fn px4_critical_warning_is_land() {
        let mut b = BatteryModel::new();
        let t = feed_healthy(&mut b, 400, 0);
        // PX4 declares CRITICAL warning → mirror as CRITICAL/Land
        b.on_event(&power(t, 14.8, 10.0, 0.3, 2), FlightPhase::Steady);
        match b.evaluate(&ctx(t + 100_000_000)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Critical);
                assert_eq!(f.action, Action::Land);
                assert_eq!(f.detail_code, detail::BATT_PX4_WARNING);
            }
            other => panic!("expected Available(Critical), got {other:?}"),
        }
    }

    #[test]
    fn undervoltage_under_load_is_critical() {
        let mut b = BatteryModel::new();
        let t = feed_healthy(&mut b, 400, 0);
        // cell driven below the 3.3 V floor while loaded
        b.on_event(&power(t, 12.8, 12.0, 0.15, 0), FlightPhase::Steady);
        match b.evaluate(&ctx(t + 100_000_000)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Critical);
                assert_eq!(f.action, Action::Land);
            }
            other => panic!("expected Critical, got {other:?}"),
        }
    }

    #[test]
    fn gauge_nonmonotonic_warns() {
        let mut b = BatteryModel::new();
        let t = feed_healthy(&mut b, 400, 0);
        // remaining jumps UP by 0.2 → gauge fault
        b.on_event(&power(t, 15.8, 10.0, 0.95, 0), FlightPhase::Steady);
        match b.evaluate(&ctx(t + 100_000_000)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Warn);
                assert_eq!(f.detail_code, detail::BATT_GAUGE_NONMONOTONIC);
            }
            other => panic!("expected Warn(gauge), got {other:?}"),
        }
    }

    #[test]
    fn disconnected_is_unavailable() {
        let mut b = BatteryModel::new();
        let mut d = power(0, 15.8, 10.0, 0.8, 0);
        if let TelemetryEvent::Power(ref mut p, _) = d {
            p.connected = 0;
        }
        b.on_event(&d, FlightPhase::Steady);
        assert!(matches!(
            b.evaluate(&ctx(1_000_000_000)),
            AlgoOutput::Unavailable(detail::AVAIL_NO_DATA)
        ));
    }

    #[test]
    fn ocv_is_monotonic_and_bracketed() {
        // OCV must be non-decreasing in SoC and within cell bounds
        let mut prev = 0.0;
        for k in 0..=100 {
            let s = k as f64 / 100.0;
            let v = ocv(s);
            assert!(v >= prev - 1e-9, "OCV not monotonic at soc={s}");
            assert!((3.30..=4.20).contains(&v));
            prev = v;
        }
    }

    #[test]
    fn deterministic_repeat() {
        let run = || {
            let mut b = BatteryModel::new();
            let t = feed_healthy(&mut b, 300, 0);
            b.on_event(&power(t, 12.8, 12.0, 0.15, 0), FlightPhase::Steady);
            let out = b.evaluate(&ctx(t + 100_000_000));
            match out {
                AlgoOutput::Available(f) => {
                    (f.severity as u8, f.detail_code, f.value.to_bits(), f.confidence)
                }
                _ => (255, 0, 0, 0),
            }
        };
        assert_eq!(run(), run());
    }
}
