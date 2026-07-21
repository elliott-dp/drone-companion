//! `cc-ai-health` — online, unsupervised, **deterministic** companion
//! health-detection (dev-plan Phase 7). It replaces the Phase-6 scripted
//! severity source: it runs change-point-statistics + physics-model algorithms
//! over the live `TelemetryEvent` stream and emits the `severity` +
//! `recommended_action` that go into `CC_HEALTH_REPORT`, which the deterministic
//! FC safety monitor turns into policy.
//!
//! Design (framework, determinism contract, per-algorithm methods, false-
//! positive strategy, deviations): `docs/phase7/phase7_ai_health.md`.
//!
//! ## The determinism spine (the exit criterion)
//! * The only clock is `RxMeta.cc_receive_time_ns` (integer), taken as a
//!   monotone logical time; the 10 Hz grid is anchored at
//!   `floor(first_rx / 100ms) * 100ms`. Live and replay share this code path,
//!   so `cc-replay` re-running a recorded mission produces byte-identical
//!   findings.
//! * **All detector state is folded in [`HealthAlgorithm::on_event`], which
//!   always runs; [`HealthAlgorithm::evaluate`] takes `&self` and is a pure
//!   read.** A slow or skipped `evaluate` therefore cannot change any later
//!   finding (the byte-identical-replay killer, removed by construction).
//! * `LinkStatus` / `StreamStale` events are **not** consumed — they are not
//!   persisted by `cc-mission-log`, so consuming them would make replay diverge
//!   (deviation D7). Link health is reconstructed purely from in-stream
//!   `seq_gap` + timestamp gaps.

pub mod algos;
pub mod detail;
pub mod finding;
pub mod phase;
pub mod stats;

use cc_ingest::{AgeInfo, StreamId, TelemetryEvent};
use cc_protocol::cc_dialect::CcSubsystem;

pub use finding::{merge, AlgoOutput, HealthConclusion, HealthFinding};
use phase::{FlightPhase, FlightPhaseTracker};

/// The 10 Hz evaluation grid period.
pub const TICK_NS: i64 = 100_000_000;

/// Context handed to every algorithm at each 10 Hz tick. Pure data — the
/// algorithm's `evaluate` reads it plus its own already-folded state.
#[derive(Debug, Clone, Copy)]
pub struct EvalCtx {
    pub now_ns: i64,
    pub phase: FlightPhase,
    /// Last `cc_receive_time_ns` seen per [`StreamId`] (0 = never).
    pub last_seen_ns: [i64; 8],
    /// Latest telemetry carried a locked timesync offset.
    pub timesync_locked: bool,
}

impl EvalCtx {
    /// A required stream is fresh if seen within `max_gap_ns` of now.
    pub fn stream_fresh(&self, s: StreamId, max_gap_ns: i64) -> bool {
        let t = self.last_seen_ns[s as usize];
        t != 0 && self.now_ns.saturating_sub(t) <= max_gap_ns
    }
    pub fn stream_seen(&self, s: StreamId) -> bool {
        self.last_seen_ns[s as usize] != 0
    }
}

/// A health-detection algorithm. **State folds in `on_event`; `evaluate` is a
/// pure read** — this split is what makes replay byte-identical regardless of
/// host speed.
pub trait HealthAlgorithm: Send {
    fn subsystem(&self) -> CcSubsystem;
    /// Fold one event into internal streaming estimators. `phase` is the shared
    /// flight-phase gate: adaptive baselines must update **only** when
    /// `phase.is_steady()`. O(1), no allocation.
    fn on_event(&mut self, ev: &TelemetryEvent, phase: FlightPhase);
    /// Produce this tick's verdict from already-folded state + `ctx`. Pure
    /// read (`&self`).
    fn evaluate(&self, ctx: &EvalCtx) -> AlgoOutput;
    /// Reset on a px4_boot_id change.
    fn reset(&mut self);
}

/// `(cc_receive_time_ns, StreamId, age_locked)` for a consumable telemetry
/// event; `None` for the control variants we deliberately ignore (D7).
fn event_meta(ev: &TelemetryEvent) -> Option<(i64, StreamId, bool)> {
    let (rx, sid) = match ev {
        TelemetryEvent::State(_, m) => (m, StreamId::State),
        TelemetryEvent::Imu(_, m) => (m, StreamId::Imu),
        TelemetryEvent::Power(_, m) => (m, StreamId::Power),
        TelemetryEvent::Gps(_, m) => (m, StreamId::Gps),
        TelemetryEvent::Estimator(_, m) => (m, StreamId::Estimator),
        TelemetryEvent::Actuator(_, m) => (m, StreamId::Actuator),
        TelemetryEvent::Event(_, m) => (m, StreamId::Event),
        TelemetryEvent::SafetyStatus(_, m) => (m, StreamId::SafetyStatus),
        TelemetryEvent::LinkStatus(_) | TelemetryEvent::StreamStale(_) => return None,
    };
    Some((rx.cc_receive_time_ns, sid, matches!(rx.age, AgeInfo::Locked { .. })))
}

fn norm3(v: &[f32; 3]) -> f64 {
    let x = v[0] as f64;
    let y = v[1] as f64;
    let z = v[2] as f64;
    libm::sqrt(x * x + y * y + z * z)
}

/// The runner: owns the algorithm registry (fixed order), the flight-phase
/// gate, and the logical clock. `on_event` folds; `tick` evaluates + merges.
pub struct Runner {
    algos: Vec<Box<dyn HealthAlgorithm>>,
    phase: FlightPhaseTracker,
    clock: i64,
    last_seen_ns: [i64; 8],
    timesync_locked: bool,
    last_boot_id: Option<u32>,
}

impl Runner {
    /// Build a runner from a fixed-order algorithm registry.
    pub fn new(algos: Vec<Box<dyn HealthAlgorithm>>) -> Self {
        Self {
            algos,
            phase: FlightPhaseTracker::new(),
            clock: 0,
            last_seen_ns: [0; 8],
            timesync_locked: false,
            last_boot_id: None,
        }
    }

    pub fn clock(&self) -> i64 {
        self.clock
    }

    /// Fold one telemetry event. Updates the logical clock, freshness, the
    /// flight phase (from State), and every algorithm's streaming state. The
    /// control variants (`LinkStatus`/`StreamStale`) are ignored (D7).
    pub fn on_event(&mut self, ev: &TelemetryEvent) {
        let Some((cc_ns, sid, age_locked)) = event_meta(ev) else {
            return;
        };
        self.clock = self.clock.max(cc_ns);
        self.last_seen_ns[sid as usize] = cc_ns;
        self.timesync_locked = age_locked;

        if let TelemetryEvent::State(d, m) = ev {
            // px4_boot_id change → reset every detector (fault-free fresh start)
            if self.last_boot_id.is_some_and(|b| b != d.px4_boot_id) {
                for a in &mut self.algos {
                    a.reset();
                }
                self.phase.reset();
            }
            self.last_boot_id = Some(d.px4_boot_id);

            let omega = norm3(&d.angular_velocity);
            let hspeed =
                libm::sqrt((d.velocity_ned[0] as f64).powi(2) + (d.velocity_ned[1] as f64).powi(2));
            self.phase.on_state(m.cc_receive_time_ns, d.arming_state, omega, hspeed);
        }

        let ph = self.phase.phase();
        for a in &mut self.algos {
            a.on_event(ev, ph);
        }
    }

    /// Evaluate every algorithm at a grid boundary and merge → one conclusion.
    pub fn tick(&self, now_ns: i64) -> HealthConclusion {
        let ctx = EvalCtx {
            now_ns,
            phase: self.phase.phase(),
            last_seen_ns: self.last_seen_ns,
            timesync_locked: self.timesync_locked,
        };
        let outs: Vec<AlgoOutput> = self.algos.iter().map(|a| a.evaluate(&ctx)).collect();
        merge(&outs, self.timesync_locked)
    }

    /// Deterministically drive a **time-ordered** event slice through the 10 Hz
    /// grid, returning `(tick_ns, conclusion)` for each boundary. Used by
    /// `cc-replay` and by tests — the same bucketing the async driver realizes
    /// with a 100 ms interval.
    pub fn run_events(&mut self, events: &[TelemetryEvent]) -> Vec<(i64, HealthConclusion)> {
        let mut out = Vec::new();
        let mut next_tick: Option<i64> = None;
        for ev in events {
            let Some((cc_ns, _, _)) = event_meta(ev) else {
                continue;
            };
            let nt = *next_tick.get_or_insert_with(|| (cc_ns / TICK_NS) * TICK_NS + TICK_NS);
            let mut t = nt;
            // fire every grid boundary at or before this event (state up to the
            // boundary is already folded, since events are time-ordered)
            while t <= cc_ns {
                out.push((t, self.tick(t)));
                t += TICK_NS;
            }
            next_tick = Some(t);
            self.on_event(ev);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_ingest::RxMeta;
    use cc_protocol::cc_dialect::CC_TELEMETRY_POWER_DATA;

    // A trivial algorithm that flags WARN once it has seen >= N power samples,
    // used to exercise the runner/grid/merge plumbing deterministically.
    struct CountAlgo {
        seen: u64,
        threshold: u64,
    }
    impl HealthAlgorithm for CountAlgo {
        fn subsystem(&self) -> CcSubsystem {
            CcSubsystem::CC_SUBSYS_BATTERY
        }
        fn on_event(&mut self, ev: &TelemetryEvent, _p: FlightPhase) {
            if matches!(ev, TelemetryEvent::Power(..)) {
                self.seen += 1;
            }
        }
        fn evaluate(&self, _ctx: &EvalCtx) -> AlgoOutput {
            use cc_health_tx::{Action, Severity};
            if self.seen >= self.threshold {
                AlgoOutput::Available(HealthFinding {
                    subsystem: CcSubsystem::CC_SUBSYS_BATTERY,
                    flag_bit: finding::flags::BATTERY,
                    severity: Severity::Warn,
                    action: Action::WarnOnly,
                    detail_code: detail::BATT_SAG_BEYOND_MODEL,
                    value: self.seen as f32,
                    limit: self.threshold as f32,
                    confidence: 90,
                })
            } else {
                AlgoOutput::Degraded(detail::AVAIL_WARMUP)
            }
        }
        fn reset(&mut self) {
            self.seen = 0;
        }
    }

    fn power(cc_ns: i64) -> TelemetryEvent {
        let d = CC_TELEMETRY_POWER_DATA {
            fc_timestamp_us: cc_ns as u64 / 1000,
            sequence: 0,
            voltage: 16.0,
            current: 5.0,
            power: 80.0,
            consumed_mah: 0.0,
            remaining: 0.9,
            temperature: 25.0,
            cell_count: 4,
            warning: 0,
            connected: 1,
            schema_version: 1,
        };
        TelemetryEvent::Power(d, RxMeta { cc_receive_time_ns: cc_ns, seq_gap: 0, age: AgeInfo::Locked { age_ns: 1 } })
    }

    #[test]
    fn grid_fires_at_100ms_boundaries() {
        let mut r = Runner::new(vec![Box::new(CountAlgo { seen: 0, threshold: 3 })]);
        // 10 power samples at 10 Hz over ~1 s
        let events: Vec<_> = (0..10).map(|i| power(i * 100_000_000 + 5_000_000)).collect();
        let ticks = r.run_events(&events);
        // ~9 grid boundaries crossed
        assert!(ticks.len() >= 8, "got {} ticks", ticks.len());
        // early ticks are Degraded (warmup) → OK severity + ai_degraded flag
        assert_eq!(ticks[0].1.severity, cc_health_tx::Severity::Ok);
        assert_ne!(ticks[0].1.health_flags & finding::flags::AI_DEGRADED, 0);
        // later ticks WARN once the count threshold is met
        assert_eq!(ticks.last().unwrap().1.severity, cc_health_tx::Severity::Warn);
    }

    #[test]
    fn replay_is_byte_identical() {
        let events: Vec<_> = (0..40).map(|i| power(i * 25_000_000 + 3_000_000)).collect();
        let hash = || {
            let mut r = Runner::new(vec![Box::new(CountAlgo { seen: 0, threshold: 5 })]);
            let ticks = r.run_events(&events);
            ticks
                .iter()
                .map(|(t, c)| {
                    (*t as u64)
                        .wrapping_mul(31)
                        .wrapping_add(c.severity as u64)
                        .wrapping_add(c.health_flags as u64)
                        .wrapping_add(c.detail_code as u64)
                        .wrapping_add((c.value.to_bits()) as u64)
                })
                .fold(0u64, |a, x| a.wrapping_mul(1000003).wrapping_add(x))
        };
        assert_eq!(hash(), hash());
    }
}
