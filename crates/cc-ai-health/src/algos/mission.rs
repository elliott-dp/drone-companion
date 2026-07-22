//! `mission_risk` (health-flag bit 128) — the energy-to-home reserve.
//!
//! # The gap this fills
//!
//! The FC's battery failsafe is a **local** SoC threshold — it knows the pack is
//! at 18 %, not that 18 % is not enough to fly the 900 m back home into wind. The
//! companion has the state to close that loop: it learns the vehicle's **cruise
//! energy rate** and **cruise speed** in flight, tracks **distance to home**, and
//! projects the **state of charge it would arrive home with**. It warns while
//! there is still margin to act, and calls the **point of no return** — the last
//! moment an RTL still lands home above reserve.
//!
//! # Model (all learned online, cruise-gated)
//!
//! ```text
//!   t_home      = distance_home / v_rtl
//!   charge_home = I_cruise · t_home        (mAh)
//!   soc_at_home = (remaining·capacity − charge_home) / capacity
//! ```
//!
//! * `I_cruise` — EWMA of battery current in steady flight.
//! * `v_rtl` — EWMA of horizontal speed while **cruising** (falls back to a
//!   nominal RTL speed until enough cruise is seen).
//! * `distance_home` — horizontal range from the arm position (captured at the
//!   first armed frame), from `position_ned`.
//!
//! `soc_at_home < 20 %` → `WARN`; `< 10 %` → `CRITICAL` **point of no return →
//! RTL**. This is the *one* lane that recommends RTL, and even here the merge
//! only lets it stand when GPS **and** the estimator are healthy — an RTL that
//! trusts a bad navigation solution is downgraded to Land.
//!
//! # Guards
//! Only assessed when armed and meaningfully **away from home** (near home the
//! battery lane already covers low SoC). Capacity / reserve / nominal-RTL-speed
//! are configuration (deviation D4; defaults here). Advisory until the FP audit.

use crate::finding::{flags, AlgoOutput, HealthFinding};
use crate::phase::FlightPhase;
use crate::stats::{clamp01, confidence_percent, logistic, Ewma};
use crate::{detail, EvalCtx, HealthAlgorithm};
use cc_health_tx::{Action, Severity};
use cc_ingest::{StreamId, TelemetryEvent};
use cc_protocol::cc_dialect::CcSubsystem;

// configuration defaults (become cc-config params — deviation D4)
const PACK_CAPACITY_MAH: f64 = 5000.0;
const NOMINAL_RTL_SPEED: f64 = 8.0; // m/s until cruise speed is learned
const CRUISE_SPEED_MIN: f64 = 2.0; // horizontal speed that counts as cruise
const MIN_DIST_HOME: f64 = 20.0; // below this, "at home" — defer to battery lane
const WARN_SOC_AT_HOME: f64 = 0.20;
const CRIT_SOC_AT_HOME: f64 = 0.10;
const WARMUP_SAMPLES: u64 = 30;

pub struct MissionRisk {
    home_ne: Option<(f64, f64)>,
    dist_home: f64,
    i_cruise: Ewma, // battery current in steady flight (A)
    v_cruise: Ewma, // horizontal speed while cruising (m/s)
    remaining: f64,
    have_power: bool,
    samples: u64,
}

impl Default for MissionRisk {
    fn default() -> Self {
        Self::new()
    }
}

impl MissionRisk {
    pub fn new() -> Self {
        Self {
            home_ne: None,
            dist_home: 0.0,
            i_cruise: Ewma::new(0.02),
            v_cruise: Ewma::new(0.02),
            remaining: f64::NAN,
            have_power: false,
            samples: 0,
        }
    }

    /// Projected state of charge on arrival home (fraction), or `None` if the
    /// model is not yet identifiable.
    fn soc_at_home(&self) -> Option<f64> {
        if !self.have_power || !self.remaining.is_finite() {
            return None;
        }
        if self.i_cruise.count() < 20 {
            return None;
        }
        let v_rtl = if self.v_cruise.is_warm(20) {
            self.v_cruise.mean().max(CRUISE_SPEED_MIN)
        } else {
            NOMINAL_RTL_SPEED
        };
        let t_home_s = self.dist_home / v_rtl;
        let charge_home_mah = self.i_cruise.mean() * (t_home_s / 3600.0) * 1000.0;
        let avail_mah = self.remaining * PACK_CAPACITY_MAH;
        Some((avail_mah - charge_home_mah) / PACK_CAPACITY_MAH)
    }
}

impl HealthAlgorithm for MissionRisk {
    fn subsystem(&self) -> CcSubsystem {
        CcSubsystem::CC_SUBSYS_MISSION
    }

    fn on_event(&mut self, ev: &TelemetryEvent, phase: FlightPhase) {
        match ev {
            TelemetryEvent::State(d, _) => {
                if phase == FlightPhase::Disarmed {
                    return;
                }
                let n = d.position_ned[0] as f64;
                let e = d.position_ned[1] as f64;
                if n.is_finite() && e.is_finite() {
                    let (hn, he) = *self.home_ne.get_or_insert((n, e));
                    self.dist_home = libm::sqrt((n - hn).powi(2) + (e - he).powi(2));
                }
                let h_speed = libm::sqrt(
                    (d.velocity_ned[0] as f64).powi(2) + (d.velocity_ned[1] as f64).powi(2),
                );
                // cruise speed learned only while actually cruising
                if phase.is_steady() && h_speed > CRUISE_SPEED_MIN && h_speed.is_finite() {
                    self.v_cruise.update(h_speed);
                }
            }
            TelemetryEvent::Power(d, _) => {
                if phase == FlightPhase::Disarmed {
                    return;
                }
                self.have_power = d.connected != 0;
                if d.remaining.is_finite() {
                    self.remaining = clamp01(d.remaining as f64);
                }
                // energy rate learned in any steady flight (hover or cruise)
                if phase.is_steady() && (d.current as f64).is_finite() {
                    self.i_cruise.update(d.current as f64);
                    self.samples = self.samples.saturating_add(1);
                }
            }
            _ => {}
        }
    }

    fn evaluate(&self, ctx: &EvalCtx) -> AlgoOutput {
        if !super::fresh(ctx, StreamId::Power) || !super::fresh(ctx, StreamId::State) {
            return AlgoOutput::Unavailable(detail::AVAIL_STREAM_STALE);
        }
        if ctx.phase == FlightPhase::Disarmed {
            return AlgoOutput::Available(ok());
        }
        if self.samples < WARMUP_SAMPLES {
            return AlgoOutput::Degraded(detail::AVAIL_WARMUP);
        }
        // near home → the battery lane owns low-SoC; nothing to project
        if self.dist_home < MIN_DIST_HOME {
            return AlgoOutput::Available(ok());
        }
        let Some(soc_home) = self.soc_at_home() else {
            return AlgoOutput::Degraded(detail::AVAIL_LOW_EXCITATION);
        };

        if soc_home < CRIT_SOC_AT_HOME {
            let conf = clamp01(0.85 + 0.15 * logistic((CRIT_SOC_AT_HOME - soc_home) * 30.0));
            return AlgoOutput::Available(HealthFinding {
                subsystem: CcSubsystem::CC_SUBSYS_MISSION,
                flag_bit: flags::MISSION,
                severity: Severity::Critical,
                action: Action::Rtl,
                detail_code: detail::MISSION_POINT_OF_NO_RETURN,
                value: soc_home as f32,
                limit: CRIT_SOC_AT_HOME as f32,
                confidence: confidence_percent(conf),
            });
        }
        if soc_home < WARN_SOC_AT_HOME {
            return AlgoOutput::Available(HealthFinding {
                subsystem: CcSubsystem::CC_SUBSYS_MISSION,
                flag_bit: flags::MISSION,
                severity: Severity::Warn,
                action: Action::WarnOnly,
                detail_code: detail::MISSION_ENERGY_RESERVE_LOW,
                value: soc_home as f32,
                limit: WARN_SOC_AT_HOME as f32,
                confidence: confidence_percent(0.8),
            });
        }
        AlgoOutput::Available(ok())
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

fn ok() -> HealthFinding {
    HealthFinding {
        subsystem: CcSubsystem::CC_SUBSYS_MISSION,
        flag_bit: flags::MISSION,
        severity: Severity::Ok,
        action: Action::None,
        detail_code: 0,
        value: 0.0,
        limit: 0.0,
        confidence: 100,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_ingest::{AgeInfo, RxMeta};
    use cc_protocol::cc_dialect::{CC_TELEMETRY_POWER_DATA, CC_TELEMETRY_STATE_DATA};

    fn ctx(now_ns: i64) -> EvalCtx {
        EvalCtx {
            now_ns,
            phase: FlightPhase::Steady,
            last_seen_ns: [now_ns; 8],
            timesync_locked: true,
        }
    }

    fn state(cc_ns: i64, n: f32, e: f32, vn: f32) -> TelemetryEvent {
        let d = CC_TELEMETRY_STATE_DATA {
            position_ned: [n, e, -10.0],
            velocity_ned: [vn, 0.0, 0.0],
            arming_state: 2,
            schema_version: 1,
            ..Default::default()
        };
        TelemetryEvent::State(
            d,
            RxMeta { cc_receive_time_ns: cc_ns, seq_gap: 0, age: AgeInfo::Locked { age_ns: 1 } },
        )
    }

    fn power(cc_ns: i64, remaining: f32, current: f32) -> TelemetryEvent {
        let d = CC_TELEMETRY_POWER_DATA {
            fc_timestamp_us: cc_ns as u64 / 1000,
            sequence: 0,
            voltage: 15.0,
            current,
            power: 15.0 * current,
            consumed_mah: 0.0,
            remaining,
            temperature: 30.0,
            cell_count: 4,
            warning: 0,
            connected: 1,
            schema_version: 1,
        };
        TelemetryEvent::Power(
            d,
            RxMeta { cc_receive_time_ns: cc_ns, seq_gap: 0, age: AgeInfo::Locked { age_ns: 1 } },
        )
    }

    // fly outbound to `dist` metres, cruising, at SoC `remaining` and current `cur`
    fn fly(m: &mut MissionRisk, n: usize, t0: i64, dist: f32, remaining: f32, cur: f32) -> i64 {
        let mut t = t0;
        for i in 0..n {
            // arm at home (0,0) then move out to `dist`
            let x = if i == 0 { 0.0 } else { dist };
            m.on_event(&state(t, x, 0.0, 6.0), FlightPhase::Steady);
            m.on_event(&power(t, remaining, cur), FlightPhase::Steady);
            t += 100_000_000;
        }
        t
    }

    #[test]
    fn plenty_of_reserve_no_finding() {
        let mut m = MissionRisk::new();
        // 300 m out, 80 % SoC, modest draw → arrives home well above reserve
        let t = fly(&mut m, 60, 0, 300.0, 0.8, 15.0);
        match m.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => assert_eq!(f.severity, Severity::Ok),
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn point_of_no_return_is_critical_rtl() {
        let mut m = MissionRisk::new();
        // far out (1500 m), low SoC (18 %), high draw → won't make it home + reserve
        let t = fly(&mut m, 60, 0, 1500.0, 0.18, 45.0);
        match m.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Critical);
                assert_eq!(f.action, Action::Rtl);
                assert_eq!(f.detail_code, detail::MISSION_POINT_OF_NO_RETURN);
            }
            other => panic!("expected Critical RTL, got {other:?}"),
        }
    }

    #[test]
    fn marginal_reserve_warns() {
        let mut m = MissionRisk::new();
        // 600 m out, 30 % SoC, 30 A → arrives home at ~13 % (10–20 % band)
        let t = fly(&mut m, 60, 0, 600.0, 0.30, 30.0);
        match m.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Warn);
                assert_eq!(f.detail_code, detail::MISSION_ENERGY_RESERVE_LOW);
            }
            other => panic!("expected Warn, got {other:?}"),
        }
    }

    #[test]
    fn near_home_defers_to_battery_lane() {
        let mut m = MissionRisk::new();
        // only 5 m from home even at low SoC → mission_risk stays quiet
        let t = fly(&mut m, 60, 0, 5.0, 0.12, 40.0);
        match m.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => assert_eq!(f.severity, Severity::Ok),
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn warmup_is_degraded() {
        let mut m = MissionRisk::new();
        let t = fly(&mut m, 5, 0, 500.0, 0.5, 20.0);
        assert!(matches!(m.evaluate(&ctx(t)), AlgoOutput::Degraded(detail::AVAIL_WARMUP)));
    }

    #[test]
    fn deterministic_repeat() {
        let run = || {
            let mut m = MissionRisk::new();
            let t = fly(&mut m, 60, 0, 1500.0, 0.18, 45.0);
            match m.evaluate(&ctx(t)) {
                AlgoOutput::Available(f) => (f.severity as u8, f.detail_code, f.value.to_bits()),
                _ => (255, 0, 0),
            }
        };
        assert_eq!(run(), run());
    }
}
