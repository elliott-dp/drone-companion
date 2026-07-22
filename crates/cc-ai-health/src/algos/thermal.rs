//! `thermal_monitor` (health-flag bit 32).
//!
//! # What we can and cannot see
//!
//! The telemetry contract exposes **battery pack temperature** (`Power`) and
//! **IMU die temperature** (`Imu`). There is **no Jetson SoC temperature** in
//! the contract (deviation D2 — flagged as a candidate telemetry addition), so
//! companion-compute thermal throttling is out of scope here.
//!
//! # Method — despike, then absolute + rate, floor-armed
//!
//! Each channel is **median-of-3 despiked** (a single garbage sample cannot move
//! the verdict) and lightly EWMA-smoothed. Two rules run on the smoothed signal:
//! * **absolute limit** — a plain over-temperature threshold; and
//! * **rate-of-rise `dT/dt`** — but **armed only above a temperature floor**, so
//!   the normal cold-start warm-up ramp (ambient → operating temp) is *not*
//!   flagged. A fast rise only matters once the pack/sensor is already hot.
//!
//! # Escalation
//! A battery that is **both hot and rising fast** is the thermal-runaway
//! signature → `CRITICAL → Land`. An absolute battery over-temp is likewise
//! `CRITICAL`. IMU over-temp degrades gyro/accel bias but is not immediately
//! vehicle-fatal → `WARN`. NaN temperature → that channel is `Unavailable`,
//! never a fault.

use crate::finding::{flags, AlgoOutput, HealthFinding};
use crate::phase::FlightPhase;
use crate::stats::{clamp01, confidence_percent, logistic, Ewma};
use crate::{detail, EvalCtx, HealthAlgorithm};
use cc_health_tx::{Action, Severity};
use cc_ingest::{StreamId, TelemetryEvent};
use cc_protocol::cc_dialect::CcSubsystem;

// battery pack (°C, °C/s)
const BATT_WARN: f64 = 55.0;
const BATT_CRIT: f64 = 65.0;
// Rate rule armed only once the pack is above *normal operating* temperature,
// so a fast cold-start warm-up ramp toward ~45 °C never arms it.
const BATT_FLOOR: f64 = 50.0;
const BATT_RATE_WARN: f64 = 0.5;
const BATT_RATE_CRIT: f64 = 1.5;
// IMU die (°C, °C/s). IMUs self-heat to ~60 °C normally, so the rate floor
// sits above that and below the absolute WARN.
const IMU_WARN: f64 = 70.0;
const IMU_CRIT: f64 = 85.0;
const IMU_FLOOR: f64 = 65.0;
const IMU_RATE_WARN: f64 = 1.0;

const WARMUP_SAMPLES: u64 = 15;

fn median3(a: f64, b: f64, c: f64) -> f64 {
    a.max(b).min(a.min(b).max(c)) // median without sorting/alloc
}

struct Channel {
    raw: [f64; 3],
    filled: usize,
    ewma: Ewma,
    last_ns: i64,
    last_temp: f64,
    rate: Ewma, // smoothed dT/dt
    nan: bool,
    samples: u64,
}
impl Channel {
    fn new() -> Self {
        Self {
            raw: [0.0; 3],
            filled: 0,
            ewma: Ewma::new(0.3),
            last_ns: 0,
            last_temp: f64::NAN,
            rate: Ewma::new(0.3),
            nan: false,
            samples: 0,
        }
    }

    fn update(&mut self, cc_ns: i64, temp: f64) {
        if !temp.is_finite() {
            self.nan = true;
            return;
        }
        self.nan = false;
        // median-of-3 despike on the raw ring
        self.raw[self.filled % 3] = temp;
        self.filled += 1;
        let despiked = if self.filled >= 3 {
            median3(self.raw[0], self.raw[1], self.raw[2])
        } else {
            temp
        };
        self.ewma.update(despiked);
        let smoothed = self.ewma.mean();
        if self.last_temp.is_finite() && self.last_ns != 0 {
            let dt = (cc_ns - self.last_ns) as f64 * 1e-9;
            if dt > 0.0 {
                self.rate.update((smoothed - self.last_temp) / dt);
            }
        }
        self.last_temp = smoothed;
        self.last_ns = cc_ns;
        self.samples = self.samples.saturating_add(1);
    }

    fn temp(&self) -> f64 {
        self.ewma.mean()
    }
    fn dtdt(&self) -> f64 {
        self.rate.mean()
    }
    fn ready(&self) -> bool {
        !self.nan && self.samples >= WARMUP_SAMPLES
    }
}

pub struct ThermalMonitor {
    batt: Channel,
    imu: Channel,
}

impl Default for ThermalMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ThermalMonitor {
    pub fn new() -> Self {
        Self { batt: Channel::new(), imu: Channel::new() }
    }
}

/// A per-channel verdict, ranked so the worst wins across channels.
struct V {
    sev: Severity,
    action: Action,
    code: u16,
    value: f64,
    limit: f64,
    conf: f64,
}

fn rank(s: Severity) -> u8 {
    match s {
        Severity::Ok => 0,
        Severity::Warn => 1,
        Severity::Stale => 2,
        Severity::Critical => 3,
    }
}

impl HealthAlgorithm for ThermalMonitor {
    fn subsystem(&self) -> CcSubsystem {
        CcSubsystem::CC_SUBSYS_THERMAL
    }

    fn on_event(&mut self, ev: &TelemetryEvent, _phase: FlightPhase) {
        match ev {
            TelemetryEvent::Power(d, m) => {
                self.batt.update(m.cc_receive_time_ns, d.temperature as f64)
            }
            TelemetryEvent::Imu(d, m) => self.imu.update(m.cc_receive_time_ns, d.temperature as f64),
            _ => {}
        }
    }

    fn evaluate(&self, ctx: &EvalCtx) -> AlgoOutput {
        let batt_fresh = super::fresh(ctx, StreamId::Power);
        let imu_fresh = super::fresh(ctx, StreamId::Imu);
        if !batt_fresh && !imu_fresh {
            return AlgoOutput::Unavailable(detail::AVAIL_STREAM_STALE);
        }

        let mut worst: Option<V> = None;
        let mut consider = |v: V| {
            if worst.as_ref().is_none_or(|w| rank(v.sev) > rank(w.sev)) {
                worst = Some(v);
            }
        };

        // ---- battery channel ----
        if batt_fresh {
            if !self.batt.ready() {
                if self.batt.nan {
                    return AlgoOutput::Degraded(detail::AVAIL_NAN_INPUT);
                }
            } else {
                let t = self.batt.temp();
                let r = self.batt.dtdt();
                if t >= BATT_CRIT {
                    consider(V { sev: Severity::Critical, action: Action::Land, code: detail::THERM_BATT_HIGH, value: t, limit: BATT_CRIT, conf: 0.95 });
                } else if t >= BATT_FLOOR && r >= BATT_RATE_CRIT {
                    consider(V { sev: Severity::Critical, action: Action::Land, code: detail::THERM_BATT_RATE, value: r, limit: BATT_RATE_CRIT, conf: 0.9 });
                } else if t >= BATT_WARN {
                    consider(V { sev: Severity::Warn, action: Action::WarnOnly, code: detail::THERM_BATT_HIGH, value: t, limit: BATT_WARN, conf: 0.8 });
                } else if t >= BATT_FLOOR && r >= BATT_RATE_WARN {
                    consider(V { sev: Severity::Warn, action: Action::WarnOnly, code: detail::THERM_BATT_RATE, value: r, limit: BATT_RATE_WARN, conf: 0.75 });
                }
            }
        }

        // ---- IMU channel (WARN ceiling) ----
        if imu_fresh && self.imu.ready() {
            let t = self.imu.temp();
            let r = self.imu.dtdt();
            if t >= IMU_CRIT {
                consider(V { sev: Severity::Warn, action: Action::WarnOnly, code: detail::THERM_IMU_HIGH, value: t, limit: IMU_CRIT, conf: 0.85 });
            } else if t >= IMU_WARN {
                consider(V { sev: Severity::Warn, action: Action::WarnOnly, code: detail::THERM_IMU_HIGH, value: t, limit: IMU_WARN, conf: 0.75 });
            } else if t >= IMU_FLOOR && r >= IMU_RATE_WARN {
                consider(V { sev: Severity::Warn, action: Action::WarnOnly, code: detail::THERM_IMU_RATE, value: r, limit: IMU_RATE_WARN, conf: 0.7 });
            }
        }

        match worst {
            None => AlgoOutput::Available(ok()),
            Some(v) => {
                let conf = clamp01(v.conf + 0.1 * logistic(v.value - v.limit));
                AlgoOutput::Available(HealthFinding {
                    subsystem: CcSubsystem::CC_SUBSYS_THERMAL,
                    flag_bit: flags::THERMAL,
                    severity: v.sev,
                    action: v.action,
                    detail_code: v.code,
                    value: v.value as f32,
                    limit: v.limit as f32,
                    confidence: confidence_percent(conf),
                })
            }
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

fn ok() -> HealthFinding {
    HealthFinding {
        subsystem: CcSubsystem::CC_SUBSYS_THERMAL,
        flag_bit: flags::THERMAL,
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
    use cc_protocol::cc_dialect::{CC_TELEMETRY_IMU_DATA, CC_TELEMETRY_POWER_DATA};

    fn ctx(now_ns: i64) -> EvalCtx {
        EvalCtx {
            now_ns,
            phase: FlightPhase::Steady,
            last_seen_ns: [now_ns; 8],
            timesync_locked: true,
        }
    }

    fn power(cc_ns: i64, temp: f32) -> TelemetryEvent {
        let d = CC_TELEMETRY_POWER_DATA {
            fc_timestamp_us: cc_ns as u64 / 1000,
            sequence: 0,
            voltage: 15.0,
            current: 10.0,
            power: 150.0,
            consumed_mah: 0.0,
            remaining: 0.7,
            temperature: temp,
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

    fn imu(cc_ns: i64, temp: f32) -> TelemetryEvent {
        let d = CC_TELEMETRY_IMU_DATA {
            fc_timestamp_us: cc_ns as u64 / 1000,
            sequence: 0,
            clipping_count: 0,
            accel: [0.0, 0.0, -9.8],
            gyro: [0.0; 3],
            delta_angle: [0.0; 3],
            delta_velocity: [0.0; 3],
            vibration_metric: [5.0, 0.05, 0.0005],
            temperature: temp,
            schema_version: 1,
        };
        TelemetryEvent::Imu(
            d,
            RxMeta { cc_receive_time_ns: cc_ns, seq_gap: 0, age: AgeInfo::Locked { age_ns: 1 } },
        )
    }

    #[test]
    fn cold_start_warmup_not_flagged() {
        // temperature ramps ambient(20)→operating(45), fast, but below the floor
        let mut th = ThermalMonitor::new();
        let mut t = 0;
        for i in 0..40 {
            let temp = 20.0 + 0.6 * i as f32; // rises quickly through warm-up
            th.on_event(&power(t, temp), FlightPhase::Steady);
            th.on_event(&imu(t, 30.0 + 0.5 * i as f32), FlightPhase::Steady);
            t += 100_000_000;
        }
        match th.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => assert_eq!(f.severity, Severity::Ok, "warm-up must not flag"),
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn battery_absolute_overtemp_critical_land() {
        let mut th = ThermalMonitor::new();
        let mut t = 0;
        for _ in 0..30 {
            th.on_event(&power(t, 67.0), FlightPhase::Steady);
            t += 100_000_000;
        }
        match th.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Critical);
                assert_eq!(f.action, Action::Land);
                assert_eq!(f.detail_code, detail::THERM_BATT_HIGH);
            }
            other => panic!("expected Critical, got {other:?}"),
        }
    }

    #[test]
    fn battery_runaway_hot_and_rising_critical() {
        let mut th = ThermalMonitor::new();
        let mut t = 0;
        // above the floor and rising ~2°C/s
        for i in 0..30 {
            let temp = 45.0 + 0.2 * i as f32; // 0.2°C per 100 ms = 2°C/s
            th.on_event(&power(t, temp), FlightPhase::Steady);
            t += 100_000_000;
        }
        match th.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Critical);
                assert_eq!(f.detail_code, detail::THERM_BATT_RATE);
            }
            other => panic!("expected runaway Critical, got {other:?}"),
        }
    }

    #[test]
    fn imu_overtemp_is_warn_only() {
        let mut th = ThermalMonitor::new();
        let mut t = 0;
        for _ in 0..30 {
            th.on_event(&power(t, 30.0), FlightPhase::Steady);
            th.on_event(&imu(t, 88.0), FlightPhase::Steady);
            t += 100_000_000;
        }
        match th.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Warn);
                assert_eq!(f.action, Action::WarnOnly);
                assert_eq!(f.detail_code, detail::THERM_IMU_HIGH);
            }
            other => panic!("expected IMU warn, got {other:?}"),
        }
    }

    #[test]
    fn single_spike_despiked() {
        // one 200°C garbage reading amid healthy 30°C must not trip
        let mut th = ThermalMonitor::new();
        let mut t = 0;
        for i in 0..30 {
            let temp = if i == 20 { 200.0 } else { 30.0 };
            th.on_event(&power(t, temp), FlightPhase::Steady);
            t += 100_000_000;
        }
        match th.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => assert_eq!(f.severity, Severity::Ok, "spike must be despiked"),
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn nan_temp_channel_degraded() {
        let mut th = ThermalMonitor::new();
        let mut t = 0;
        for _ in 0..5 {
            th.on_event(&power(t, f32::NAN), FlightPhase::Steady);
            t += 100_000_000;
        }
        assert!(matches!(
            th.evaluate(&ctx(t)),
            AlgoOutput::Degraded(detail::AVAIL_NAN_INPUT)
        ));
    }

    #[test]
    fn deterministic_repeat() {
        let run = || {
            let mut th = ThermalMonitor::new();
            let mut t = 0;
            for i in 0..30 {
                th.on_event(&power(t, 45.0 + 0.2 * i as f32), FlightPhase::Steady);
                t += 100_000_000;
            }
            match th.evaluate(&ctx(t)) {
                AlgoOutput::Available(f) => (f.severity as u8, f.detail_code, f.value.to_bits()),
                _ => (255, 0, 0),
            }
        };
        assert_eq!(run(), run());
    }
}
