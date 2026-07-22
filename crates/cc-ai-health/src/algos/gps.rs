//! `gps_quality` (health-flag bit 8).
//!
//! # Many weak indicators, fused by independence
//!
//! No single GNSS field is trustworthy on its own — `eph` spikes in an urban
//! canyon, `jamming_indicator` twitches on a passing transmitter, a satellite
//! count dips under foliage — all while the fix stays perfectly usable. A
//! monitor that warns on any one of them WARN-spams every real flight. So we run
//! a **panel of per-indicator detectors** and escalate on **corroboration across
//! independent causes**, not on any single reading.
//!
//! Indicators and their **cause groups** (correlated fields share a group and so
//! cannot corroborate each other):
//! * **geometry** — `fix_type`, `satellites_used`, `eph`, `epv` (all degrade
//!   together when the sky view is poor);
//! * **RF** — `noise_per_ms`, `jamming_indicator` (receiver interference);
//! * **consistency** — GPS ground-speed vs the EKF's horizontal speed (gated on
//!   `estimator_valid`).
//!
//! # Detector choices
//! * `eph`/`epv` are heavy-tailed, so their step detector runs on **`ln`** of
//!   the value (robust z + Page-Hinkley up) plus an absolute limit.
//! * `noise`/`jamming` have **no universal absolute** (they are receiver- and
//!   site-specific), so only an **adaptive** upward step (robust z) is used —
//!   this is the environment gate that stops benign-but-noisy sites warning.
//! * `satellites_used` uses an absolute floor **and** a downward drift.
//!
//! # Escalation (the panel's rule)
//! * `fix_type < 3` sustained is PX4's **definitive** loss-of-3D-fix — not a
//!   noisy field — so it reaches `CRITICAL` on its own.
//! * Otherwise a `CRITICAL` needs bad indicators in **two different groups**; a
//!   single group is `WARN`.
//! * Action `CRITICAL → Hold`: with GNSS unhealthy, RTL (which *flies a GPS
//!   course home*) is unsafe. The merge further blocks any RTL when GPS is bad.

use crate::finding::{flags, AlgoOutput, HealthFinding};
use crate::phase::FlightPhase;
use crate::stats::{clamp01, confidence_percent, ln, logistic};
use crate::stats::{Cusum, CusumTrip, Direction, PageHinkley, RobustScale};
use crate::{detail, EvalCtx, HealthAlgorithm};
use cc_health_tx::{Action, Severity};
use cc_ingest::{StreamId, TelemetryEvent};
use cc_protocol::cc_dialect::CcSubsystem;

const FIX_3D: u8 = 3;
const SAT_WARN: u8 = 7; // marginal below this
const EPH_WARN: f64 = 3.0; // metres
const EPH_CRIT: f64 = 6.0;
const EPV_WARN: f64 = 5.0;
const SPEED_DIV_MS: f64 = 3.0; // GPS↔EKF horizontal-speed disagreement
const Z_STEP: f64 = 4.0; // robust-z step for RF/eph indicators
const STREAK: u32 = 5; // ≈1 s sustained at 5 Hz GPS
const WARMUP_SAMPLES: u64 = 30;

pub struct GpsQuality {
    // geometry
    fix_bad_streak: u32,
    sat_low_streak: u32,
    sat_cusum: Cusum, // downward drift in satellite count
    sat_val: f64,
    eph_scale: RobustScale,
    eph_ph: PageHinkley,
    eph_abs_streak: u32,
    eph_val: f64,
    epv_abs_streak: u32,
    epv_val: f64,
    // RF (adaptive, no absolute)
    noise_scale: RobustScale,
    noise_z: f64,
    jam_scale: RobustScale,
    jam_z: f64,
    // consistency
    ekf_hspeed: f64,
    estimator_valid: bool,
    speed_div_streak: u32,
    speed_div: f64,
    samples: u64,
}

impl Default for GpsQuality {
    fn default() -> Self {
        Self::new()
    }
}

impl GpsQuality {
    pub fn new() -> Self {
        Self {
            fix_bad_streak: 0,
            sat_low_streak: 0,
            sat_cusum: Cusum::new(0.5, 4.0),
            sat_val: 0.0,
            eph_scale: RobustScale::new(64, 0.05),
            eph_ph: PageHinkley::new(Direction::Up, 0.5, 6.0),
            eph_abs_streak: 0,
            eph_val: 0.0,
            epv_abs_streak: 0,
            epv_val: 0.0,
            noise_scale: RobustScale::new(64, 1.0),
            noise_z: 0.0,
            jam_scale: RobustScale::new(64, 1.0),
            jam_z: 0.0,
            ekf_hspeed: 0.0,
            estimator_valid: false,
            speed_div_streak: 0,
            speed_div: 0.0,
            samples: 0,
        }
    }

    // ---- indicator readouts (pure) ----
    fn geometry_bad(&self) -> bool {
        self.sat_low_streak >= STREAK
            || self.sat_cusum.trip() == CusumTrip::Down
            || self.eph_abs_streak >= STREAK
            || self.eph_ph.tripped()
            || self.epv_abs_streak >= STREAK
    }
    fn rf_bad(&self) -> bool {
        self.noise_z >= Z_STEP || self.jam_z >= Z_STEP
    }
    fn consistency_bad(&self) -> bool {
        self.speed_div_streak >= STREAK
    }
}

impl HealthAlgorithm for GpsQuality {
    fn subsystem(&self) -> CcSubsystem {
        CcSubsystem::CC_SUBSYS_GPS
    }

    fn on_event(&mut self, ev: &TelemetryEvent, phase: FlightPhase) {
        // track EKF horizontal speed + validity from the State stream
        if let TelemetryEvent::State(d, _) = ev {
            self.ekf_hspeed =
                libm::sqrt((d.velocity_ned[0] as f64).powi(2) + (d.velocity_ned[1] as f64).powi(2));
            self.estimator_valid = d.estimator_valid != 0;
            return;
        }

        let TelemetryEvent::Gps(d, _) = ev else {
            return;
        };
        if phase == FlightPhase::Disarmed {
            return;
        }
        self.samples = self.samples.saturating_add(1);
        let adapt = phase.is_steady();

        // --- geometry ---
        if d.fix_type < FIX_3D {
            self.fix_bad_streak = self.fix_bad_streak.saturating_add(1);
        } else {
            self.fix_bad_streak = 0;
        }

        self.sat_val = d.satellites_used as f64;
        if d.satellites_used < SAT_WARN {
            self.sat_low_streak = self.sat_low_streak.saturating_add(1);
        } else {
            self.sat_low_streak = 0;
        }
        // downward drift vs a nominal healthy count (12)
        self.sat_cusum.update(self.sat_val, 12.0);

        let eph = d.eph as f64;
        self.eph_val = eph;
        if eph.is_finite() && eph > 0.0 {
            let leph = ln(eph);
            let z = self.eph_scale.z(leph);
            if self.eph_scale.is_warm(20) && z > 0.0 {
                self.eph_ph.update(z);
            }
            if adapt {
                self.eph_scale.update(leph);
            }
        }
        if eph > EPH_WARN {
            self.eph_abs_streak = self.eph_abs_streak.saturating_add(1);
        } else {
            self.eph_abs_streak = 0;
        }

        let epv = d.epv as f64;
        self.epv_val = epv;
        if epv > EPV_WARN {
            self.epv_abs_streak = self.epv_abs_streak.saturating_add(1);
        } else {
            self.epv_abs_streak = 0;
        }

        // --- RF (adaptive upward step, no absolute) ---
        // Update the baseline unless this sample is clearly a step. `z` is NaN
        // while the window is empty (warm-up), and `!(NaN >= Z_STEP)` is true,
        // so the bootstrap sample is still learned; a genuine step (`z ≥ STEP`)
        // is withheld so the anomaly is never absorbed into normal.
        let noise = d.noise_per_ms as f64;
        self.noise_z = self.noise_scale.z(noise);
        if adapt && !(self.noise_z >= Z_STEP) {
            self.noise_scale.update(noise);
        }
        let jam = d.jamming_indicator as f64;
        self.jam_z = self.jam_scale.z(jam);
        if adapt && !(self.jam_z >= Z_STEP) {
            self.jam_scale.update(jam);
        }

        // --- consistency (only when the EKF solution is valid) ---
        if self.estimator_valid && d.ground_speed.is_finite() {
            self.speed_div = (d.ground_speed as f64 - self.ekf_hspeed).abs();
            if self.speed_div > SPEED_DIV_MS {
                self.speed_div_streak = self.speed_div_streak.saturating_add(1);
            } else {
                self.speed_div_streak = 0;
            }
        } else {
            self.speed_div_streak = 0;
        }
    }

    fn evaluate(&self, ctx: &EvalCtx) -> AlgoOutput {
        if !super::fresh(ctx, StreamId::Gps) {
            return AlgoOutput::Unavailable(detail::AVAIL_STREAM_STALE);
        }
        if self.samples < WARMUP_SAMPLES {
            return AlgoOutput::Degraded(detail::AVAIL_WARMUP);
        }

        // definitive loss of 3D fix → CRITICAL on its own (not a noisy field)
        if self.fix_bad_streak >= STREAK {
            return crit(detail::GPS_FIX_DEGRADED, 0.0, FIX_3D as f64, 0.95);
        }

        let g = self.geometry_bad();
        let r = self.rf_bad();
        let c = self.consistency_bad();
        let n_groups = [g, r, c].iter().filter(|b| **b).count();

        // pick the dominant indicator + detail for the report
        let (code, value, limit) = self.dominant_indicator(g, r, c);

        if n_groups >= 2 {
            let conf = clamp01(0.8 + 0.15 * logistic(self.eph_val - EPH_WARN));
            return crit(code, value, limit, conf);
        }
        if n_groups == 1 {
            return warn(code, value, limit, 0.75);
        }
        AlgoOutput::Available(ok())
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

impl GpsQuality {
    fn dominant_indicator(&self, g: bool, r: bool, c: bool) -> (u16, f64, f64) {
        if g {
            if self.eph_abs_streak >= STREAK || self.eph_ph.tripped() {
                if self.eph_val > EPH_CRIT {
                    return (detail::GPS_EPH_HIGH, self.eph_val, EPH_CRIT);
                }
                return (detail::GPS_EPH_HIGH, self.eph_val, EPH_WARN);
            }
            if self.epv_abs_streak >= STREAK {
                return (detail::GPS_EPV_HIGH, self.epv_val, EPV_WARN);
            }
            return (detail::GPS_LOW_SATS, self.sat_val, SAT_WARN as f64);
        }
        if r {
            if self.jam_z >= Z_STEP {
                return (detail::GPS_JAMMING, self.jam_z, Z_STEP);
            }
            return (detail::GPS_NOISE_STEP, self.noise_z, Z_STEP);
        }
        if c {
            return (detail::GPS_SPEED_DIVERGENCE, self.speed_div, SPEED_DIV_MS);
        }
        (detail::GPS_COMPOSITE, 0.0, 0.0)
    }
}

fn ok() -> HealthFinding {
    HealthFinding {
        subsystem: CcSubsystem::CC_SUBSYS_GPS,
        flag_bit: flags::GPS,
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
        subsystem: CcSubsystem::CC_SUBSYS_GPS,
        flag_bit: flags::GPS,
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
        subsystem: CcSubsystem::CC_SUBSYS_GPS,
        flag_bit: flags::GPS,
        severity: Severity::Critical,
        action: Action::Hold,
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
    use cc_protocol::cc_dialect::{CC_TELEMETRY_GPS_DATA, CC_TELEMETRY_STATE_DATA};

    fn ctx(now_ns: i64) -> EvalCtx {
        EvalCtx {
            now_ns,
            phase: FlightPhase::Steady,
            last_seen_ns: [now_ns; 8],
            timesync_locked: true,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn gps(
        cc_ns: i64,
        fix: u8,
        sats: u8,
        eph: f32,
        epv: f32,
        noise: u16,
        jam: u16,
        gspeed: f32,
    ) -> TelemetryEvent {
        let d = CC_TELEMETRY_GPS_DATA {
            fc_timestamp_us: cc_ns as u64 / 1000,
            sequence: 0,
            lat: 0,
            lon: 0,
            alt: 0,
            eph,
            epv,
            ground_speed: gspeed,
            heading: 0.0,
            noise_per_ms: noise,
            jamming_indicator: jam,
            fix_type: fix,
            satellites_used: sats,
            schema_version: 1,
        };
        TelemetryEvent::Gps(
            d,
            RxMeta { cc_receive_time_ns: cc_ns, seq_gap: 0, age: AgeInfo::Locked { age_ns: 1 } },
        )
    }

    fn state(cc_ns: i64, vn: f32, ve: f32, valid: u8) -> TelemetryEvent {
        let d = CC_TELEMETRY_STATE_DATA {
            velocity_ned: [vn, ve, 0.0],
            estimator_valid: valid,
            arming_state: 2,
            schema_version: 1,
            ..Default::default()
        };
        TelemetryEvent::State(
            d,
            RxMeta { cc_receive_time_ns: cc_ns, seq_gap: 0, age: AgeInfo::Locked { age_ns: 1 } },
        )
    }

    // healthy 3D fix, good geometry, quiet RF, speed agrees. No finding.
    fn feed_healthy(g: &mut GpsQuality, n: usize, t0: i64) -> i64 {
        let mut t = t0;
        for i in 0..n {
            g.on_event(&state(t, 5.0, 0.0, 1), FlightPhase::Steady);
            let noise = 80 + (i % 3) as u16;
            g.on_event(&gps(t, 4, 14, 0.8, 1.2, noise, 5, 5.0), FlightPhase::Steady);
            t += 200_000_000;
        }
        t
    }

    #[test]
    fn benign_gps_no_finding() {
        let mut g = GpsQuality::new();
        let t = feed_healthy(&mut g, 60, 0);
        match g.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => assert_eq!(f.severity, Severity::Ok),
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn loss_of_fix_is_critical_alone() {
        let mut g = GpsQuality::new();
        let mut t = feed_healthy(&mut g, 60, 0);
        for _ in 0..STREAK + 2 {
            g.on_event(&state(t, 5.0, 0.0, 1), FlightPhase::Steady);
            g.on_event(&gps(t, 1, 3, 0.8, 1.2, 80, 5, 5.0), FlightPhase::Steady);
            t += 200_000_000;
        }
        match g.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Critical);
                assert_eq!(f.action, Action::Hold);
                assert_eq!(f.detail_code, detail::GPS_FIX_DEGRADED);
            }
            other => panic!("expected Critical fix, got {other:?}"),
        }
    }

    #[test]
    fn single_group_geometry_is_warn_only() {
        // high eph alone (one group) → WARN, never CRITICAL
        let mut g = GpsQuality::new();
        let mut t = feed_healthy(&mut g, 60, 0);
        for _ in 0..STREAK + 3 {
            g.on_event(&state(t, 5.0, 0.0, 1), FlightPhase::Steady);
            g.on_event(&gps(t, 4, 14, 8.0, 2.0, 80, 5, 5.0), FlightPhase::Steady);
            t += 200_000_000;
        }
        match g.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Warn);
                assert_eq!(f.detail_code, detail::GPS_EPH_HIGH);
            }
            other => panic!("expected Warn eph, got {other:?}"),
        }
    }

    #[test]
    fn geometry_plus_rf_is_critical() {
        // bad geometry (eph) AND high jamming (RF) → two independent groups
        let mut g = GpsQuality::new();
        let mut t = feed_healthy(&mut g, 60, 0);
        for _ in 0..STREAK + 3 {
            g.on_event(&state(t, 5.0, 0.0, 1), FlightPhase::Steady);
            g.on_event(&gps(t, 4, 14, 8.0, 2.0, 80, 200, 5.0), FlightPhase::Steady);
            t += 200_000_000;
        }
        match g.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Critical);
                assert_eq!(f.action, Action::Hold);
            }
            other => panic!("expected Critical, got {other:?}"),
        }
    }

    #[test]
    fn speed_divergence_gated_on_estimator_valid() {
        // huge GPS/EKF speed disagreement but estimator INVALID → not counted
        let mut g = GpsQuality::new();
        let mut t = feed_healthy(&mut g, 60, 0);
        for _ in 0..STREAK + 3 {
            g.on_event(&state(t, 0.0, 0.0, 0), FlightPhase::Steady); // invalid
            g.on_event(&gps(t, 4, 14, 0.8, 1.2, 80, 5, 20.0), FlightPhase::Steady);
            t += 200_000_000;
        }
        match g.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => assert_eq!(f.severity, Severity::Ok, "invalid EKF gates it"),
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn stale_gps_unavailable() {
        let g = GpsQuality::new();
        let c = EvalCtx {
            now_ns: 100_000_000_000,
            phase: FlightPhase::Steady,
            last_seen_ns: [0; 8],
            timesync_locked: true,
        };
        assert!(matches!(
            g.evaluate(&c),
            AlgoOutput::Unavailable(detail::AVAIL_STREAM_STALE)
        ));
    }

    #[test]
    fn deterministic_repeat() {
        let run = || {
            let mut g = GpsQuality::new();
            let mut t = feed_healthy(&mut g, 60, 0);
            for _ in 0..STREAK + 3 {
                g.on_event(&state(t, 5.0, 0.0, 1), FlightPhase::Steady);
                g.on_event(&gps(t, 4, 14, 8.0, 2.0, 80, 200, 5.0), FlightPhase::Steady);
                t += 200_000_000;
            }
            match g.evaluate(&ctx(t)) {
                AlgoOutput::Available(f) => (f.severity as u8, f.detail_code, f.value.to_bits()),
                _ => (255, 0, 0),
            }
        };
        assert_eq!(run(), run());
    }
}
