//! `motor_balance` (health-flag bit 2) — **correlation-only, advisory**.
//!
//! # Honest scope
//!
//! The telemetry contract carries **no ESC RPM, no per-motor current, no ESC
//! temperature** — only the normalized actuator outputs the mixer commanded.
//! We therefore cannot *measure* a motor fault; we can only notice that the
//! controller is **persistently asking one motor to work harder** than its
//! peers to hold the vehicle level. That is a weak, indirect signal, so this
//! lane is **WARN-only and advisory** (confidence hard-capped ≤ 70) until
//! hardware-in-the-loop and the false-positive audit justify more — exactly the
//! reduced-observability caveat the design records.
//!
//! # The heading-invariance discriminator (the panel's sharpest idea)
//!
//! A persistently high command on one motor has two very different causes:
//! * **Wind** — a steady breeze tilts the vehicle; the mixer loads whichever
//!   motors oppose it. This asymmetry is **earth-fixed**: as the vehicle yaws,
//!   the *loaded motor rotates* with heading.
//! * **A weak motor** — a failing motor always needs more command **regardless
//!   of heading**. This asymmetry is **body-fixed**.
//!
//! So we accumulate each motor's excess-over-collective-mean while the vehicle
//! visits **different headings**, and only trust a body-fixed offset once enough
//! heading diversity has been seen for the earth-fixed wind component to average
//! down. With the vehicle pointing one way the whole time, wind and a weak motor
//! are **unobservable-apart** — we say so (`HEADING_STATIONARY_ASYM`, lower
//! confidence) rather than guess.
//!
//! # Signals
//! * `OUTPUT_OFFSET` — body-fixed per-motor excess beyond threshold, heading
//!   diversity sufficient.
//! * `HEADING_STATIONARY_ASYM` — asymmetry seen but heading too stationary to
//!   attribute (advisory, low confidence).
//! * `ACTUATOR_SATURATION` — a motor pinned near max output (sustained): the
//!   mixer is out of authority on that arm.
//! * `MULTI_SIGNAL` — offset **and** saturation on the same motor (the strongest
//!   correlation-only case).
//!
//! Assumes `actuator_output[i]` is the PX4 normalized motor command (~`[0,1]`).
//! Steady-phase gated; NaN outputs skip the frame. `CRITICAL` is intentionally
//! never emitted here (it would require the independent vibration corroborator
//! plus saturation — cross-lane, post-audit).

use crate::finding::{flags, AlgoOutput, HealthFinding};
use crate::phase::FlightPhase;
use crate::stats::{confidence_percent, Ewma};
use crate::{detail, EvalCtx, HealthAlgorithm};
use cc_health_tx::{Action, Severity};
use cc_ingest::{StreamId, TelemetryEvent};
use cc_protocol::cc_dialect::CcSubsystem;

const MAX_MOTORS: usize = 8;
const OFFSET_WARN: f64 = 0.08; // 8 % of normalized command, body-fixed
const SAT_LEVEL: f64 = 0.95;
const SAT_STREAK: u32 = 20;
const HEADING_SECTORS: u32 = 8; // 45° bins
const MIN_SECTORS: u32 = 3; // ≥135° of heading coverage to attribute body-fixed
const WARMUP_SAMPLES: u64 = 100;
const CONF_CAP: u8 = 70;

pub struct MotorBalance {
    excess: [Ewma; MAX_MOTORS],
    sat_streak: [u32; MAX_MOTORS],
    motor_count: usize,
    heading_sectors: u32, // bitmask of visited 45° sectors
    heading: f64,
    samples: u64,
}

impl Default for MotorBalance {
    fn default() -> Self {
        Self::new()
    }
}

impl MotorBalance {
    pub fn new() -> Self {
        Self {
            excess: std::array::from_fn(|_| Ewma::new(0.005)),
            sat_streak: [0; MAX_MOTORS],
            motor_count: 0,
            heading_sectors: 0,
            heading: 0.0,
            samples: 0,
        }
    }

    fn sectors_visited(&self) -> u32 {
        self.heading_sectors.count_ones()
    }

    /// Worst body-fixed motor by |mean excess|, returning `(index, signed_excess)`.
    fn worst_offset(&self) -> Option<(usize, f64)> {
        let mut worst: Option<(usize, f64)> = None;
        for i in 0..self.motor_count {
            if self.excess[i].count() < 20 {
                continue;
            }
            let e = self.excess[i].mean();
            if worst.is_none_or(|(_, w)| e.abs() > w.abs()) {
                worst = Some((i, e));
            }
        }
        worst
    }

    fn worst_saturated(&self) -> Option<usize> {
        (0..self.motor_count).find(|&i| self.sat_streak[i] >= SAT_STREAK)
    }
}

impl HealthAlgorithm for MotorBalance {
    fn subsystem(&self) -> CcSubsystem {
        CcSubsystem::CC_SUBSYS_MOTOR
    }

    fn on_event(&mut self, ev: &TelemetryEvent, phase: FlightPhase) {
        if let TelemetryEvent::State(d, _) = ev {
            let h = d.heading as f64;
            if h.is_finite() {
                self.heading = h;
                if phase.is_steady() {
                    // map heading (−π..π] to a sector and mark it visited
                    let frac = (h + std::f64::consts::PI) / (2.0 * std::f64::consts::PI);
                    let sector = (frac * HEADING_SECTORS as f64) as i64;
                    let sector = sector.rem_euclid(HEADING_SECTORS as i64) as u32;
                    self.heading_sectors |= 1 << sector;
                }
            }
            return;
        }

        let TelemetryEvent::Actuator(d, _) = ev else {
            return;
        };
        if !phase.is_steady() {
            return;
        }
        let n = (d.motor_count as usize).clamp(1, MAX_MOTORS);
        self.motor_count = n;
        let outs = &d.actuator_output[..n];
        if outs.iter().any(|x| !x.is_finite()) {
            return;
        }
        let mean = outs.iter().map(|x| *x as f64).sum::<f64>() / n as f64;
        self.samples = self.samples.saturating_add(1);
        for (i, &o) in outs.iter().enumerate() {
            let o = o as f64;
            self.excess[i].update(o - mean);
            if o >= SAT_LEVEL {
                self.sat_streak[i] = self.sat_streak[i].saturating_add(1);
            } else {
                self.sat_streak[i] = 0;
            }
        }
    }

    fn evaluate(&self, ctx: &EvalCtx) -> AlgoOutput {
        if !super::fresh(ctx, StreamId::Actuator) {
            return AlgoOutput::Unavailable(detail::AVAIL_STREAM_STALE);
        }
        if self.samples < WARMUP_SAMPLES {
            return AlgoOutput::Degraded(detail::AVAIL_WARMUP);
        }

        let saturated = self.worst_saturated();
        let offset = self.worst_offset().filter(|(_, e)| e.abs() > OFFSET_WARN);

        match (offset, saturated) {
            // strongest correlation-only case: same-ish arm offset + saturation
            (Some((_, e)), Some(_)) => {
                warn(detail::MOTOR_MULTI_SIGNAL, e.abs(), OFFSET_WARN, 70)
            }
            (Some((_, e)), None) => {
                if self.sectors_visited() >= MIN_SECTORS {
                    // body-fixed asymmetry survived heading averaging
                    let conf = conf_from_excess(e.abs());
                    warn(detail::MOTOR_OUTPUT_OFFSET, e.abs(), OFFSET_WARN, conf)
                } else {
                    // can't separate wind from a weak motor yet — say so
                    warn(detail::MOTOR_HEADING_STATIONARY_ASYM, e.abs(), OFFSET_WARN, 45)
                }
            }
            (None, Some(_)) => {
                warn(detail::MOTOR_ACTUATOR_SATURATION, 1.0, SAT_LEVEL, 60)
            }
            (None, None) => AlgoOutput::Available(ok()),
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

/// Confidence rises with excess magnitude but is hard-capped at `CONF_CAP`.
fn conf_from_excess(excess_abs: f64) -> u8 {
    let raw = 0.4 + 3.0 * (excess_abs - OFFSET_WARN); // ramps above threshold
    confidence_percent(raw.clamp(0.0, 1.0)).min(CONF_CAP)
}

fn ok() -> HealthFinding {
    HealthFinding {
        subsystem: CcSubsystem::CC_SUBSYS_MOTOR,
        flag_bit: flags::MOTOR,
        severity: Severity::Ok,
        action: Action::None,
        detail_code: 0,
        value: 0.0,
        limit: 0.0,
        confidence: 100,
    }
}

fn warn(code: u16, value: f64, limit: f64, conf: u8) -> AlgoOutput {
    AlgoOutput::Available(HealthFinding {
        subsystem: CcSubsystem::CC_SUBSYS_MOTOR,
        flag_bit: flags::MOTOR,
        severity: Severity::Warn,
        action: Action::WarnOnly,
        detail_code: code,
        value: value as f32,
        limit: limit as f32,
        confidence: conf.min(CONF_CAP),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_ingest::{AgeInfo, RxMeta};
    use cc_protocol::cc_dialect::{CC_TELEMETRY_ACTUATOR_DATA, CC_TELEMETRY_STATE_DATA};

    fn ctx(now_ns: i64) -> EvalCtx {
        EvalCtx {
            now_ns,
            phase: FlightPhase::Steady,
            last_seen_ns: [now_ns; 8],
            timesync_locked: true,
        }
    }

    fn actuator(cc_ns: i64, outs: [f32; 4]) -> TelemetryEvent {
        let d = CC_TELEMETRY_ACTUATOR_DATA {
            fc_timestamp_us: cc_ns as u64 / 1000,
            sequence: 0,
            actuator_output: [outs[0], outs[1], outs[2], outs[3], 0.0, 0.0, 0.0, 0.0],
            motor_count: 4,
            schema_version: 1,
        };
        TelemetryEvent::Actuator(
            d,
            RxMeta { cc_receive_time_ns: cc_ns, seq_gap: 0, age: AgeInfo::Locked { age_ns: 1 } },
        )
    }

    fn state(cc_ns: i64, heading: f32) -> TelemetryEvent {
        let d = CC_TELEMETRY_STATE_DATA {
            heading,
            arming_state: 2,
            schema_version: 1,
            ..Default::default()
        };
        TelemetryEvent::State(
            d,
            RxMeta { cc_receive_time_ns: cc_ns, seq_gap: 0, age: AgeInfo::Locked { age_ns: 1 } },
        )
    }

    // symmetric hover across many headings → no finding
    fn feed_symmetric(m: &mut MotorBalance, n: usize, t0: i64) -> i64 {
        let mut t = t0;
        for i in 0..n {
            let h = -3.0 + 6.0 * (i as f32 / n as f32); // sweep heading
            m.on_event(&state(t, h), FlightPhase::Steady);
            m.on_event(&actuator(t, [0.5, 0.5, 0.5, 0.5]), FlightPhase::Steady);
            t += 60_000_000;
        }
        t
    }

    #[test]
    fn symmetric_hover_no_finding() {
        let mut m = MotorBalance::new();
        let t = feed_symmetric(&mut m, 200, 0);
        match m.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => assert_eq!(f.severity, Severity::Ok),
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn body_fixed_offset_across_headings_warns_capped() {
        let mut m = MotorBalance::new();
        let mut t = 0;
        // motor 0 always ~12% high regardless of heading (body-fixed)
        for i in 0..300 {
            let h = -3.0 + 6.0 * ((i % 50) as f32 / 50.0);
            m.on_event(&state(t, h), FlightPhase::Steady);
            m.on_event(&actuator(t, [0.60, 0.48, 0.48, 0.48]), FlightPhase::Steady);
            t += 60_000_000;
        }
        match m.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Warn);
                assert_eq!(f.detail_code, detail::MOTOR_OUTPUT_OFFSET);
                assert!(f.confidence <= CONF_CAP, "confidence must be capped");
            }
            other => panic!("expected Warn offset, got {other:?}"),
        }
    }

    #[test]
    fn stationary_heading_offset_is_ambiguous() {
        let mut m = MotorBalance::new();
        let mut t = 0;
        // same offset but heading NEVER changes → cannot attribute to a motor
        for _ in 0..300 {
            m.on_event(&state(t, 0.2), FlightPhase::Steady);
            m.on_event(&actuator(t, [0.60, 0.48, 0.48, 0.48]), FlightPhase::Steady);
            t += 60_000_000;
        }
        match m.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.detail_code, detail::MOTOR_HEADING_STATIONARY_ASYM);
                assert!(f.confidence < 50);
            }
            other => panic!("expected ambiguous asym, got {other:?}"),
        }
    }

    #[test]
    fn saturation_detected() {
        let mut m = MotorBalance::new();
        let mut t = feed_symmetric(&mut m, 150, 0);
        // motor 3 pinned at max, others compensate low, sustained
        for _ in 0..30 {
            m.on_event(&state(t, 0.0), FlightPhase::Steady);
            m.on_event(&actuator(t, [0.5, 0.5, 0.5, 0.99]), FlightPhase::Steady);
            t += 60_000_000;
        }
        match m.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Warn);
                assert!(matches!(
                    f.detail_code,
                    detail::MOTOR_ACTUATOR_SATURATION | detail::MOTOR_MULTI_SIGNAL
                ));
            }
            other => panic!("expected saturation warn, got {other:?}"),
        }
    }

    #[test]
    fn never_emits_critical() {
        // even an extreme sustained offset stays WARN-only (advisory lane)
        let mut m = MotorBalance::new();
        let mut t = 0;
        for i in 0..300 {
            let h = -3.0 + 6.0 * ((i % 50) as f32 / 50.0);
            m.on_event(&state(t, h), FlightPhase::Steady);
            m.on_event(&actuator(t, [0.90, 0.30, 0.30, 0.30]), FlightPhase::Steady);
            t += 60_000_000;
        }
        match m.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_ne!(f.severity, Severity::Critical);
                assert_eq!(f.action, Action::WarnOnly);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn deterministic_repeat() {
        let run = || {
            let mut m = MotorBalance::new();
            let mut t = 0;
            for i in 0..250 {
                let h = -3.0 + 6.0 * ((i % 50) as f32 / 50.0);
                m.on_event(&state(t, h), FlightPhase::Steady);
                m.on_event(&actuator(t, [0.60, 0.48, 0.48, 0.48]), FlightPhase::Steady);
                t += 60_000_000;
            }
            match m.evaluate(&ctx(t)) {
                AlgoOutput::Available(f) => (f.severity as u8, f.detail_code, f.value.to_bits(), f.confidence),
                _ => (255, 0, 0, 0),
            }
        };
        assert_eq!(run(), run());
    }
}
