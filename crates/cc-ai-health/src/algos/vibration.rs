//! `vibration_anomaly` (health-flag bit 4).
//!
//! # The three metrics are *different physical quantities*, not x/y/z
//!
//! PX4's `vibration_metric[3]` is **not** an xyz vector (a common misreading the
//! adversarial panel caught). It is three independent scalars:
//! * `[0]` **accel** vibration level (m/s²),
//! * `[1]` **gyro** vibration level (rad/s),
//! * `[2]` **delta-angle coning** metric (integration-artefact indicator).
//!
//! They have different units, magnitudes, and failure signatures, so each gets
//! its **own** baseline and detector; we never pool them into one z-score.
//!
//! # Detector — throttle-normalized residual, per metric
//!
//! Vibration rises with rotor speed, so raw level is throttle-dependent and a
//! fixed threshold false-positives on every spirited climb. We regress each
//! metric on a **throttle proxy** (mean actuator output) with a small recursive
//! least-squares fit `metric ≈ θ₀ + θ₁·u + θ₂·u²` (`u` = throttle), and watch the
//! **residual** — the vibration *unexplained by throttle*. A one-sided
//! Page-Hinkley on the robust z-score of that residual catches a slow structural
//! rise (a loosening mount, a chipped prop) that a raw threshold misses, while a
//! healthy hard climb moves along the fitted curve and stays quiet.
//!
//! # Absolute backstop — accel only, maneuver-gated
//!
//! Only the accel metric carries PX4's documented absolute limits
//! (`WARN ≥ 30`, `CRITICAL ≥ 60 m/s²`). These are **maneuver-gated**: acro
//! flight legitimately spikes accel vibration, so the absolute rule is only
//! trusted in a steady phase (panel fix). The gyro/coning metrics have no
//! meaningful absolute limit and rely solely on the residual detector.
//!
//! # Clipping — a high-specificity independent channel
//!
//! IMU `clipping_count` is a cumulative, per-boot counter. Its **rate**
//! `Δcount/Δt` is an almost false-positive-free saturation signal (the ADC
//! rails only under real mechanical trouble): `WARN ≈ 2/s`, `CRITICAL ≈ 20/s`.
//! Boot-reset aware (a decrease means the FC rebooted → re-baseline). Clipping
//! is trusted in any armed phase.
//!
//! # Escalation & FP guards
//! `CRITICAL` when **≥2 metrics trip together** (corroboration), or clip-rate
//! high, or accel ≥ 60. Baselines adapt only in Steady; detectors freeze on
//! anomaly; NaN metric → that channel is skipped, never a fault. Action
//! `CRITICAL → Land`.

use crate::finding::{flags, AlgoOutput, HealthFinding};
use crate::phase::FlightPhase;
use crate::stats::{clamp01, confidence_percent, logistic};
use crate::stats::{Direction, PageHinkley, Rls3, RobustScale};
use crate::{detail, EvalCtx, HealthAlgorithm};
use cc_health_tx::{Action, Severity};
use cc_ingest::{StreamId, TelemetryEvent};
use cc_protocol::cc_dialect::CcSubsystem;

const N: usize = 3; // accel, gyro, coning
const ACCEL_WARN: f64 = 30.0;
const ACCEL_CRIT: f64 = 60.0;
const CLIP_WARN_RATE: f64 = 2.0;
const CLIP_CRIT_RATE: f64 = 20.0;
const WARMUP_SAMPLES: u64 = 150; // ≈3 s of IMU at 50 Hz in Steady
/// Robust-scale floors per metric (units differ): accel m/s², gyro rad/s,
/// coning (dimensionless-ish). Prevents a divide-by-tiny-MAD z blow-up.
const SCALE_FLOOR: [f64; N] = [0.5, 0.02, 1.0e-3];
const DETAIL_STEP: [u16; N] =
    [detail::VIBE_ACCEL_STEP, detail::VIBE_GYRO_STEP, detail::VIBE_CONING_STEP];

pub struct VibrationAnomaly {
    rls: [Rls3; N],
    res: [RobustScale; N],
    ph: [PageHinkley; N],
    tripped: [bool; N],
    z_at_trip: [f64; N],
    // throttle proxy from the actuator stream
    throttle: f64,
    // clipping rate tracking
    last_clip: Option<u32>,
    last_clip_ns: i64,
    clip_rate: f64,
    // absolute accel backstop (latched)
    accel_warn: bool,
    accel_crit: bool,
    accel_val: f64,
    // warm-up
    samples: u64,
}

impl Default for VibrationAnomaly {
    fn default() -> Self {
        Self::new()
    }
}

impl VibrationAnomaly {
    pub fn new() -> Self {
        Self {
            rls: [Rls3::new(0.999, 100.0), Rls3::new(0.999, 100.0), Rls3::new(0.999, 100.0)],
            res: [
                RobustScale::new(128, SCALE_FLOOR[0]),
                RobustScale::new(128, SCALE_FLOOR[1]),
                RobustScale::new(128, SCALE_FLOOR[2]),
            ],
            ph: [
                PageHinkley::new(Direction::Up, 0.5, 6.0),
                PageHinkley::new(Direction::Up, 0.5, 6.0),
                PageHinkley::new(Direction::Up, 0.5, 6.0),
            ],
            tripped: [false; N],
            z_at_trip: [0.0; N],
            throttle: 0.0,
            last_clip: None,
            last_clip_ns: 0,
            clip_rate: 0.0,
            accel_warn: false,
            accel_crit: false,
            accel_val: 0.0,
            samples: 0,
        }
    }

    fn n_tripped(&self) -> usize {
        self.tripped.iter().filter(|t| **t).count()
    }
}

impl HealthAlgorithm for VibrationAnomaly {
    fn subsystem(&self) -> CcSubsystem {
        CcSubsystem::CC_SUBSYS_VIBRATION
    }

    fn on_event(&mut self, ev: &TelemetryEvent, phase: FlightPhase) {
        // throttle proxy: mean of the active actuator outputs
        if let TelemetryEvent::Actuator(d, _) = ev {
            let n = (d.motor_count as usize).clamp(1, d.actuator_output.len());
            let sum: f64 = d.actuator_output[..n].iter().map(|x| *x as f64).sum();
            let mean = sum / n as f64;
            if mean.is_finite() {
                self.throttle = mean;
            }
            return;
        }

        let TelemetryEvent::Imu(d, m) = ev else {
            return;
        };
        let cc_ns = m.cc_receive_time_ns;

        // clipping rate — trusted in any armed phase, boot-reset aware
        if phase != FlightPhase::Disarmed {
            if let Some(prev) = self.last_clip {
                if d.clipping_count < prev {
                    // FC rebooted: re-baseline, no rate this step
                    self.last_clip = Some(d.clipping_count);
                    self.last_clip_ns = cc_ns;
                } else {
                    let dt = (cc_ns - self.last_clip_ns) as f64 * 1e-9;
                    if dt > 0.0 {
                        let rate = (d.clipping_count - prev) as f64 / dt;
                        // light EWMA to reject single-frame spikes
                        self.clip_rate = 0.7 * self.clip_rate + 0.3 * rate;
                        self.last_clip = Some(d.clipping_count);
                        self.last_clip_ns = cc_ns;
                    }
                }
            } else {
                self.last_clip = Some(d.clipping_count);
                self.last_clip_ns = cc_ns;
            }
        }

        // residual detectors + accel absolute: only in a steady phase
        if !phase.is_steady() {
            return;
        }
        self.samples = self.samples.saturating_add(1);
        let u = self.throttle;
        let phi = [1.0, u, u * u];

        for k in 0..N {
            let metric = d.vibration_metric[k] as f64;
            if !metric.is_finite() {
                continue;
            }
            if k == 0 {
                self.accel_val = metric;
                if metric >= ACCEL_CRIT {
                    self.accel_crit = true;
                } else if metric >= ACCEL_WARN {
                    self.accel_warn = true;
                }
            }
            if self.tripped[k] {
                continue; // frozen on anomaly
            }
            // fit throttle model, watch the residual
            let pred = self.rls[k].predict(&phi);
            let resid = metric - pred;
            self.rls[k].update(&phi, metric);
            self.res[k].update(resid);
            if self.res[k].is_warm(32) {
                let z = self.res[k].z(resid);
                if self.ph[k].update(z) {
                    self.tripped[k] = true;
                    self.z_at_trip[k] = z;
                    self.res[k].set_frozen(true);
                    self.ph[k].set_frozen(true);
                    self.rls[k].set_frozen(true);
                }
            }
        }
    }

    fn evaluate(&self, ctx: &EvalCtx) -> AlgoOutput {
        if !super::fresh(ctx, StreamId::Imu) {
            return AlgoOutput::Unavailable(detail::AVAIL_STREAM_STALE);
        }
        if self.samples < WARMUP_SAMPLES {
            return AlgoOutput::Degraded(detail::AVAIL_WARMUP);
        }

        let n_trip = self.n_tripped();
        // ---- CRITICAL conditions ----
        if self.accel_crit {
            return crit(detail::VIBE_ACCEL_ABSOLUTE, self.accel_val, ACCEL_CRIT, 0.95);
        }
        if self.clip_rate >= CLIP_CRIT_RATE {
            return crit(detail::VIBE_CLIPPING_RATE, self.clip_rate, CLIP_CRIT_RATE, 0.9);
        }
        if n_trip >= 2 {
            let worst = self.worst_metric();
            let conf = clamp01(0.75 + 0.25 * logistic(self.ph[worst].excess()));
            return crit(detail::VIBE_MULTI_METRIC, self.z_at_trip[worst], 0.0, conf);
        }
        // ---- WARN conditions ----
        if self.clip_rate >= CLIP_WARN_RATE {
            return warn(detail::VIBE_CLIPPING_RATE, self.clip_rate, CLIP_WARN_RATE, 0.85);
        }
        if self.accel_warn {
            return warn(detail::VIBE_ACCEL_ABSOLUTE, self.accel_val, ACCEL_WARN, 0.8);
        }
        if n_trip == 1 {
            let k = self.worst_metric();
            let conf = clamp01(0.7 + 0.3 * logistic(self.ph[k].excess()));
            return warn(DETAIL_STEP[k], self.z_at_trip[k], 0.0, conf);
        }
        AlgoOutput::Available(ok())
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

impl VibrationAnomaly {
    /// The tripped metric with the largest z at trip (for the report).
    fn worst_metric(&self) -> usize {
        let mut best = 0;
        let mut best_z = f64::NEG_INFINITY;
        for k in 0..N {
            if self.tripped[k] && self.z_at_trip[k] > best_z {
                best_z = self.z_at_trip[k];
                best = k;
            }
        }
        best
    }
}

fn ok() -> HealthFinding {
    HealthFinding {
        subsystem: CcSubsystem::CC_SUBSYS_VIBRATION,
        flag_bit: flags::VIBRATION,
        severity: Severity::Ok,
        action: Action::None,
        detail_code: 0,
        value: 0.0,
        limit: 0.0,
        confidence: 100,
    }
}

fn warn(code: u16, value: f64, limit: f64, conf01: f64) -> AlgoOutput {
    AlgoOutput::Available(HealthFinding {
        subsystem: CcSubsystem::CC_SUBSYS_VIBRATION,
        flag_bit: flags::VIBRATION,
        severity: Severity::Warn,
        action: Action::WarnOnly,
        detail_code: code,
        value: value as f32,
        limit: limit as f32,
        confidence: confidence_percent(conf01),
    })
}

fn crit(code: u16, value: f64, limit: f64, conf01: f64) -> AlgoOutput {
    AlgoOutput::Available(HealthFinding {
        subsystem: CcSubsystem::CC_SUBSYS_VIBRATION,
        flag_bit: flags::VIBRATION,
        severity: Severity::Critical,
        action: Action::Land,
        detail_code: code,
        value: value as f32,
        limit: limit as f32,
        confidence: confidence_percent(conf01),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_ingest::{AgeInfo, RxMeta};
    use cc_protocol::cc_dialect::{CC_TELEMETRY_ACTUATOR_DATA, CC_TELEMETRY_IMU_DATA};

    fn ctx(now_ns: i64) -> EvalCtx {
        EvalCtx {
            now_ns,
            phase: FlightPhase::Steady,
            last_seen_ns: [now_ns; 8],
            timesync_locked: true,
        }
    }

    fn imu(cc_ns: i64, vib: [f32; 3], clip: u32) -> TelemetryEvent {
        let d = CC_TELEMETRY_IMU_DATA {
            fc_timestamp_us: cc_ns as u64 / 1000,
            sequence: 0,
            clipping_count: clip,
            accel: [0.0, 0.0, -9.8],
            gyro: [0.0; 3],
            delta_angle: [0.0; 3],
            delta_velocity: [0.0; 3],
            vibration_metric: vib,
            temperature: 40.0,
            schema_version: 1,
        };
        TelemetryEvent::Imu(
            d,
            RxMeta { cc_receive_time_ns: cc_ns, seq_gap: 0, age: AgeInfo::Locked { age_ns: 1 } },
        )
    }

    fn actuator(cc_ns: i64, thr: f32) -> TelemetryEvent {
        let d = CC_TELEMETRY_ACTUATOR_DATA {
            fc_timestamp_us: cc_ns as u64 / 1000,
            sequence: 0,
            actuator_output: [thr, thr, thr, thr, 0.0, 0.0, 0.0, 0.0],
            motor_count: 4,
            schema_version: 1,
        };
        TelemetryEvent::Actuator(
            d,
            RxMeta { cc_receive_time_ns: cc_ns, seq_gap: 0, age: AgeInfo::Locked { age_ns: 1 } },
        )
    }

    // Healthy vibration that RISES with throttle (explained by the model) must
    // NOT be flagged.
    fn feed_healthy(v: &mut VibrationAnomaly, n: usize, t0: i64) -> i64 {
        let mut t = t0;
        for i in 0..n {
            let thr = 0.4 + 0.2 * ((i % 7) as f32 / 7.0); // varying throttle
            v.on_event(&actuator(t, thr), FlightPhase::Steady);
            // accel vibration ~ linear in throttle, small + tiny wobble
            let base = 6.0 + 20.0 * thr; // stays under 30 for thr<1.2
            let wob = if i % 2 == 0 { 0.3 } else { -0.3 };
            v.on_event(
                &imu(t, [base + wob, 0.05, 0.0005], 0),
                FlightPhase::Steady,
            );
            t += 20_000_000;
        }
        t
    }

    #[test]
    fn benign_throttle_coupled_vibration_no_finding() {
        let mut v = VibrationAnomaly::new();
        let t = feed_healthy(&mut v, 400, 0);
        match v.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => assert_eq!(f.severity, Severity::Ok),
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn accel_absolute_critical() {
        let mut v = VibrationAnomaly::new();
        let t = feed_healthy(&mut v, 200, 0);
        v.on_event(&imu(t, [65.0, 0.05, 0.0005], 0), FlightPhase::Steady);
        match v.evaluate(&ctx(t + 20_000_000)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Critical);
                assert_eq!(f.action, Action::Land);
                assert_eq!(f.detail_code, detail::VIBE_ACCEL_ABSOLUTE);
            }
            other => panic!("expected Critical, got {other:?}"),
        }
    }

    #[test]
    fn clipping_rate_critical() {
        let mut v = VibrationAnomaly::new();
        let mut t = feed_healthy(&mut v, 200, 0);
        // ramp clipping count fast: +5 per 20 ms = 250/s
        let mut clip = 0u32;
        for _ in 0..20 {
            clip += 5;
            v.on_event(&imu(t, [10.0, 0.05, 0.0005], clip), FlightPhase::Steady);
            t += 20_000_000;
        }
        match v.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Critical);
                assert_eq!(f.detail_code, detail::VIBE_CLIPPING_RATE);
            }
            other => panic!("expected Critical clip, got {other:?}"),
        }
    }

    #[test]
    fn structural_residual_step_warns() {
        let mut v = VibrationAnomaly::new();
        let mut t = feed_healthy(&mut v, 400, 0);
        // inject a throttle-independent step on the gyro metric (a fault the
        // model cannot explain): hold throttle fixed, raise gyro vib
        for _ in 0..80 {
            v.on_event(&actuator(t, 0.5), FlightPhase::Steady);
            v.on_event(&imu(t, [16.0, 0.6, 0.0005], 0), FlightPhase::Steady);
            t += 20_000_000;
        }
        match v.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Warn);
                assert_eq!(f.detail_code, detail::VIBE_GYRO_STEP);
            }
            other => panic!("expected Warn gyro step, got {other:?}"),
        }
    }

    #[test]
    fn maneuver_suppresses_residual_and_absolute() {
        let mut v = VibrationAnomaly::new();
        let t = feed_healthy(&mut v, 200, 0);
        // a big accel spike DURING a maneuver must not trip (gated)
        v.on_event(&imu(t, [70.0, 1.0, 0.01], 0), FlightPhase::Maneuver);
        match v.evaluate(&ctx(t + 20_000_000)) {
            AlgoOutput::Available(f) => assert_eq!(f.severity, Severity::Ok),
            other => panic!("expected Ok (suppressed), got {other:?}"),
        }
    }

    #[test]
    fn stale_imu_unavailable() {
        let v = VibrationAnomaly::new();
        let c = EvalCtx {
            now_ns: 10_000_000_000,
            phase: FlightPhase::Steady,
            last_seen_ns: [0; 8], // never seen
            timesync_locked: true,
        };
        assert!(matches!(
            v.evaluate(&c),
            AlgoOutput::Unavailable(detail::AVAIL_STREAM_STALE)
        ));
    }

    #[test]
    fn deterministic_repeat() {
        let run = || {
            let mut v = VibrationAnomaly::new();
            let mut t = feed_healthy(&mut v, 300, 0);
            for _ in 0..60 {
                v.on_event(&actuator(t, 0.5), FlightPhase::Steady);
                v.on_event(&imu(t, [16.0, 0.6, 0.0005], 0), FlightPhase::Steady);
                t += 20_000_000;
            }
            match v.evaluate(&ctx(t)) {
                AlgoOutput::Available(f) => (f.severity as u8, f.detail_code, f.value.to_bits()),
                _ => (255, 0, 0),
            }
        };
        assert_eq!(run(), run());
    }
}
