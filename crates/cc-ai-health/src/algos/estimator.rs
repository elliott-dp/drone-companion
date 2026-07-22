//! `estimator_consistency` (health-flag bit 16).
//!
//! # Inputs are the EKF's own self-assessment
//!
//! PX4's `CC_TELEMETRY_ESTIMATOR` carries the EKF2 **normalized innovation test
//! ratios** for velocity, position, height and magnetometer (plus airspeed,
//! which is NaN on a multicopter). By construction a ratio `< 1` passes the
//! filter's chi-square gate and `> 1` is a rejected update. So the FC already
//! acts at `1.0`; our job is the **early warning** the FC does not give — a slow
//! sustained climb *toward* 1.0 that precedes an actual divergence.
//!
//! # Why not a fixed sub-1.0 threshold (the panel's key correction)
//!
//! Healthy **dynamic** flight legitimately rides innovation ratios in the
//! 0.5–0.8 band; a fixed "warn at 0.5/0.7" would fire on every brisk manoeuvre.
//! Instead each channel learns an **adaptive baseline** (EWMA of the ratio, in
//! steady flight only) and a one-sided **CUSUM watches for drift above that
//! learned baseline** — so a vehicle that normally sits at 0.7 only warns when
//! it climbs *beyond its own normal*, and the absolute hard rule stays at PX4's
//! own semantics (`ratio > 1.0` sustained), never below.
//!
//! # Independence-aware escalation (the panel's second correction)
//!
//! Velocity and position innovations are **both driven by GNSS** — a GPS glitch
//! moves them together, so they are *not* independent corroboration. Channels
//! are grouped by cause (`{vel, pos} = GNSS`, `height`, `mag`) and a `CRITICAL`
//! requires breaches in **two different groups**. One group bad = `WARN`.
//!
//! # Action ceiling
//!
//! A bad estimator makes *navigation itself* untrustworthy, so this lane never
//! recommends RTL: `WARN → BlockOffboard`, `CRITICAL → Hold`. The merge's
//! nav-health cross-check then further downgrades any RTL from other lanes.
//!
//! # FP guards
//! adaptive baselines + CUSUM (a single glitch cannot accumulate) · sustained
//! streak for the absolute rule · detection only in Steady · airspeed NaN →
//! that channel `Unavailable`, never a fault · auto-recovers when ratios return
//! below baseline.

use crate::finding::{flags, AlgoOutput, HealthFinding};
use crate::phase::FlightPhase;
use crate::stats::{clamp01, confidence_percent, logistic};
use crate::stats::{Cusum, CusumTrip, Ewma};
use crate::{detail, EvalCtx, HealthAlgorithm};
use cc_health_tx::{Action, Severity};
use cc_ingest::{StreamId, TelemetryEvent};
use cc_protocol::cc_dialect::CcSubsystem;

const NCH: usize = 4; // vel, pos, height, mag
/// Cause-group per channel: vel & pos share the GNSS group (0); height=1; mag=2.
const GROUP: [u8; NCH] = [0, 0, 1, 2];
const CH_DETAIL: [u16; NCH] = [
    detail::EST_VEL_BREACH,
    detail::EST_POS_BREACH,
    detail::EST_HEIGHT_BREACH,
    detail::EST_MAG_BREACH,
];
const RATIO_BREACH: f64 = 1.0; // PX4 rejection boundary
const BREACH_STREAK: u32 = 8; // frames > 1.0 to call it sustained (~0.8 s @10 Hz)
const RECOVER_BELOW: f64 = 0.9; // ratio below this clears the streak
const WARMUP_SAMPLES: u64 = 50;
const INNOV_FLAG_STREAK: u32 = 10;

pub struct EstimatorConsistency {
    baseline: [Ewma; NCH],
    cusum: [Cusum; NCH],
    streak: [u32; NCH],
    worst_ratio: [f64; NCH],
    innov_flag_streak: u32,
    samples: u64,
}

impl Default for EstimatorConsistency {
    fn default() -> Self {
        Self::new()
    }
}

impl EstimatorConsistency {
    pub fn new() -> Self {
        let mk_e = || Ewma::new(0.01);
        let mk_c = || Cusum::new(0.20, 2.0); // slack 0.2, threshold 2.0 above baseline
        Self {
            baseline: [mk_e(), mk_e(), mk_e(), mk_e()],
            cusum: [mk_c(), mk_c(), mk_c(), mk_c()],
            streak: [0; NCH],
            worst_ratio: [0.0; NCH],
            innov_flag_streak: 0,
            samples: 0,
        }
    }

    fn channel_bad(&self, k: usize) -> bool {
        self.streak[k] >= BREACH_STREAK || self.cusum[k].trip() == CusumTrip::Up
    }
}

impl HealthAlgorithm for EstimatorConsistency {
    fn subsystem(&self) -> CcSubsystem {
        CcSubsystem::CC_SUBSYS_ESTIMATOR
    }

    fn on_event(&mut self, ev: &TelemetryEvent, phase: FlightPhase) {
        let TelemetryEvent::Estimator(d, _) = ev else {
            return;
        };
        if !phase.is_steady() {
            return;
        }
        self.samples = self.samples.saturating_add(1);

        let ratios = [
            d.velocity_test_ratio as f64,
            d.position_test_ratio as f64,
            d.height_test_ratio as f64,
            d.mag_test_ratio as f64,
        ];
        for k in 0..NCH {
            let r = ratios[k];
            if !r.is_finite() {
                continue; // channel unavailable this frame (e.g. airspeed)
            }
            // sustained-breach streak (absolute rule)
            if r > RATIO_BREACH {
                self.streak[k] = self.streak[k].saturating_add(1);
                self.worst_ratio[k] = self.worst_ratio[k].max(r);
            } else if r < RECOVER_BELOW {
                self.streak[k] = 0;
                self.worst_ratio[k] = 0.0;
                self.cusum[k].reset(); // recovered → forget the drift
            }
            // early-drift CUSUM against the learned baseline
            let base = if self.baseline[k].is_warm(30) {
                self.baseline[k].mean()
            } else {
                r
            };
            if self.cusum[k].update(r, base) == CusumTrip::Up {
                self.worst_ratio[k] = self.worst_ratio[k].max(r);
            }
            // adapt the baseline only while this channel is healthy
            if r < RECOVER_BELOW {
                self.baseline[k].update(r);
            }
        }

        // innovation-flag streak (any rejection bit set, sustained)
        if d.innovation_check_flags != 0 {
            self.innov_flag_streak = self.innov_flag_streak.saturating_add(1);
        } else {
            self.innov_flag_streak = 0;
        }
    }

    fn evaluate(&self, ctx: &EvalCtx) -> AlgoOutput {
        if !super::fresh(ctx, StreamId::Estimator) {
            return AlgoOutput::Unavailable(detail::AVAIL_STREAM_STALE);
        }
        if self.samples < WARMUP_SAMPLES {
            return AlgoOutput::Degraded(detail::AVAIL_WARMUP);
        }

        // collect bad channels + their independent groups
        let mut bad_groups = [false; 3];
        let mut worst_k = None;
        let mut worst_r = 0.0;
        for k in 0..NCH {
            if self.channel_bad(k) {
                bad_groups[GROUP[k] as usize] = true;
                if self.worst_ratio[k] > worst_r {
                    worst_r = self.worst_ratio[k];
                    worst_k = Some(k);
                }
            }
        }
        let n_groups = bad_groups.iter().filter(|b| **b).count();

        match worst_k {
            None => {
                // no ratio breach; a lingering innovation-flag streak is a soft WARN
                if self.innov_flag_streak >= INNOV_FLAG_STREAK {
                    return warn(
                        detail::EST_INNOV_FLAG_STREAK,
                        self.innov_flag_streak as f64,
                        INNOV_FLAG_STREAK as f64,
                        0.7,
                    );
                }
                AlgoOutput::Available(ok())
            }
            Some(k) => {
                let conf = clamp01(0.7 + 0.3 * logistic(worst_r - RATIO_BREACH));
                if n_groups >= 2 {
                    // independent corroboration → estimator genuinely unreliable
                    AlgoOutput::Available(HealthFinding {
                        subsystem: CcSubsystem::CC_SUBSYS_ESTIMATOR,
                        flag_bit: flags::ESTIMATOR,
                        severity: Severity::Critical,
                        action: Action::Hold,
                        detail_code: detail::EST_MULTI_INDEPENDENT,
                        value: worst_r as f32,
                        limit: RATIO_BREACH as f32,
                        confidence: confidence_percent(clamp01(conf + 0.1)),
                    })
                } else {
                    warn(CH_DETAIL[k], worst_r, RATIO_BREACH, conf)
                }
            }
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

fn ok() -> HealthFinding {
    HealthFinding {
        subsystem: CcSubsystem::CC_SUBSYS_ESTIMATOR,
        flag_bit: flags::ESTIMATOR,
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
        subsystem: CcSubsystem::CC_SUBSYS_ESTIMATOR,
        flag_bit: flags::ESTIMATOR,
        severity: Severity::Warn,
        action: Action::BlockOffboard,
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
    use cc_protocol::cc_dialect::CC_TELEMETRY_ESTIMATOR_DATA;

    fn ctx(now_ns: i64) -> EvalCtx {
        EvalCtx {
            now_ns,
            phase: FlightPhase::Steady,
            last_seen_ns: [now_ns; 8],
            timesync_locked: true,
        }
    }

    fn est(cc_ns: i64, vel: f32, pos: f32, height: f32, mag: f32, flags: u16) -> TelemetryEvent {
        let d = CC_TELEMETRY_ESTIMATOR_DATA {
            fc_timestamp_us: cc_ns as u64 / 1000,
            sequence: 0,
            status_flags: 0,
            velocity_test_ratio: vel,
            position_test_ratio: pos,
            height_test_ratio: height,
            mag_test_ratio: mag,
            airspeed_test_ratio: f32::NAN, // multicopter
            innovation_check_flags: flags,
            solution_status_flags: 0,
            schema_version: 1,
        };
        TelemetryEvent::Estimator(
            d,
            RxMeta { cc_receive_time_ns: cc_ns, seq_gap: 0, age: AgeInfo::Locked { age_ns: 1 } },
        )
    }

    // healthy dynamic flight: ratios ride 0.5–0.8, airspeed NaN. No finding.
    fn feed_dynamic(e: &mut EstimatorConsistency, n: usize, t0: i64) -> i64 {
        let mut t = t0;
        for i in 0..n {
            let w = 0.6 + 0.15 * (((i % 5) as f32 / 5.0) - 0.5);
            e.on_event(&est(t, w, w * 0.9, 0.5, 0.4, 0), FlightPhase::Steady);
            t += 100_000_000;
        }
        t
    }

    #[test]
    fn dynamic_flight_in_05_08_band_no_finding() {
        let mut e = EstimatorConsistency::new();
        let t = feed_dynamic(&mut e, 300, 0);
        match e.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => assert_eq!(f.severity, Severity::Ok),
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn single_group_sustained_breach_warns_blockoffboard() {
        let mut e = EstimatorConsistency::new();
        let mut t = feed_dynamic(&mut e, 200, 0);
        // height (independent group) sustained > 1.0
        for _ in 0..15 {
            e.on_event(&est(t, 0.6, 0.55, 1.4, 0.4, 0), FlightPhase::Steady);
            t += 100_000_000;
        }
        match e.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Warn);
                assert_eq!(f.action, Action::BlockOffboard);
                assert_eq!(f.detail_code, detail::EST_HEIGHT_BREACH);
            }
            other => panic!("expected Warn height, got {other:?}"),
        }
    }

    #[test]
    fn vel_and_pos_together_do_not_corroborate_to_critical() {
        // vel+pos share the GNSS group → both breaching is still ONE group → WARN
        let mut e = EstimatorConsistency::new();
        let mut t = feed_dynamic(&mut e, 200, 0);
        for _ in 0..15 {
            e.on_event(&est(t, 1.5, 1.5, 0.5, 0.4, 0), FlightPhase::Steady);
            t += 100_000_000;
        }
        match e.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Warn, "shared GNSS cause is not corroboration");
            }
            other => panic!("expected Warn, got {other:?}"),
        }
    }

    #[test]
    fn two_independent_groups_critical_hold() {
        // height + mag: two independent causes → CRITICAL, action Hold (never RTL)
        let mut e = EstimatorConsistency::new();
        let mut t = feed_dynamic(&mut e, 200, 0);
        for _ in 0..15 {
            e.on_event(&est(t, 0.6, 0.55, 1.4, 1.6, 0), FlightPhase::Steady);
            t += 100_000_000;
        }
        match e.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Critical);
                assert_eq!(f.action, Action::Hold);
                assert_eq!(f.detail_code, detail::EST_MULTI_INDEPENDENT);
            }
            other => panic!("expected Critical Hold, got {other:?}"),
        }
    }

    #[test]
    fn single_glitch_does_not_trip() {
        let mut e = EstimatorConsistency::new();
        let mut t = feed_dynamic(&mut e, 200, 0);
        // one isolated spike then back to healthy
        e.on_event(&est(t, 1.8, 0.55, 0.5, 0.4, 0), FlightPhase::Steady);
        t += 100_000_000;
        t = feed_dynamic_from(&mut e, 30, t);
        match e.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => assert_eq!(f.severity, Severity::Ok, "one glitch must clear"),
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    fn feed_dynamic_from(e: &mut EstimatorConsistency, n: usize, t0: i64) -> i64 {
        feed_dynamic(e, n, t0)
    }

    #[test]
    fn recovery_clears_the_finding() {
        let mut e = EstimatorConsistency::new();
        let mut t = feed_dynamic(&mut e, 200, 0);
        for _ in 0..15 {
            e.on_event(&est(t, 0.6, 0.55, 1.4, 0.4, 0), FlightPhase::Steady);
            t += 100_000_000;
        }
        assert!(matches!(e.evaluate(&ctx(t)), AlgoOutput::Available(f) if f.severity == Severity::Warn));
        // ratios return healthy → the finding clears
        t = feed_dynamic(&mut e, 40, t);
        match e.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => assert_eq!(f.severity, Severity::Ok),
            other => panic!("expected Ok after recovery, got {other:?}"),
        }
    }

    #[test]
    fn deterministic_repeat() {
        let run = || {
            let mut e = EstimatorConsistency::new();
            let mut t = feed_dynamic(&mut e, 200, 0);
            for _ in 0..15 {
                e.on_event(&est(t, 0.6, 0.55, 1.4, 1.6, 0), FlightPhase::Steady);
                t += 100_000_000;
            }
            match e.evaluate(&ctx(t)) {
                AlgoOutput::Available(f) => (f.severity as u8, f.detail_code, f.value.to_bits()),
                _ => (255, 0, 0),
            }
        };
        assert_eq!(run(), run());
    }
}
