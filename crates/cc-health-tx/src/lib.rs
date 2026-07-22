//! `cc-health-tx` — the companion health report transmitter (dev-plan
//! Phase 6.2, v0: a *scripted* severity source, no AI yet).
//!
//! A [`Scenario`] (config file of timed severity events) drives the companion's
//! health conclusion; [`ReportSource`] turns it into `CC_HEALTH_REPORT`s with
//! the spec rate policy (OK 1 Hz, WARN 2–5 Hz, CRITICAL 5 Hz until acknowledged
//! then 1 Hz) and severity hysteresis, tracking the monitor's ACK
//! (`CC_SAFETY_STATUS.last_report_sequence`). The report is sent at P1 on the
//! cc-link TX (below P0 heartbeat/timesync, above bulk).
//!
//! The decision cores ([`scenario`], [`policy`]) are pure and unit-tested; the
//! async [`spawn`] task is a thin wrapper.

pub mod policy;
pub mod scenario;

use std::time::Duration;

use cc_link::{clock, Priority, TxHandle};
use cc_protocol::cc_dialect::{
    CcHealthFlags, CcRecommendedAction, CcSeverity, MavMessage, CC_HEALTH_REPORT_DATA,
};
use tokio::sync::watch;
use tokio::task::JoinHandle;

pub use policy::Hysteresis;
pub use scenario::{Action, Scenario, Severity};

/// Companion self-telemetry folded into each report (informational; the
/// monitor keys on severity/action). companiond supplies live values.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelfTelemetry {
    pub link_rtt_ms: u16,
    pub telemetry_age_ms: u16,
    pub companion_loop_ms: u16,
    pub dropped_rx_count: u16,
}

/// A merged health conclusion from an upstream detector (Phase 7: the
/// `cc-ai-health` Runner). Primitive fields only — `cc-health-tx` must **not**
/// depend on `cc-ai-health` (that crate already depends on this one), so the
/// caller flattens its `HealthConclusion` into this on the way in.
#[derive(Debug, Clone, Copy)]
pub struct Conclusion {
    pub severity: Severity,
    pub action: Action,
    pub health_flags: u32,
    pub detail_code: u16,
    pub confidence: u8,
}

impl Default for Conclusion {
    fn default() -> Self {
        Self {
            severity: Severity::Ok,
            action: Action::None,
            health_flags: 0,
            detail_code: 0,
            confidence: 100,
        }
    }
}

fn to_cc_severity(s: Severity) -> CcSeverity {
    match s {
        Severity::Ok => CcSeverity::CC_SEVERITY_OK,
        Severity::Warn => CcSeverity::CC_SEVERITY_WARN,
        Severity::Critical => CcSeverity::CC_SEVERITY_CRITICAL,
        Severity::Stale => CcSeverity::CC_SEVERITY_STALE,
    }
}

fn to_cc_action(a: Action) -> CcRecommendedAction {
    match a {
        Action::None => CcRecommendedAction::CC_ACTION_NONE,
        Action::WarnOnly => CcRecommendedAction::CC_ACTION_WARN_ONLY,
        Action::BlockOffboard => CcRecommendedAction::CC_ACTION_BLOCK_OFFBOARD,
        Action::Hold => CcRecommendedAction::CC_ACTION_HOLD,
        Action::Land => CcRecommendedAction::CC_ACTION_LAND,
        Action::Rtl => CcRecommendedAction::CC_ACTION_RTL,
    }
}

/// Turns a scenario + wall-clock into paced `CC_HEALTH_REPORT`s. Pure: the
/// caller ticks it with the elapsed time and the latest acked sequence.
pub struct ReportSource {
    scenario: Scenario,
    mission_id: u32,
    boot_id: u32,
    hyst: Hysteresis,
    seq: u32,
    last_sent_ns: Option<i64>,
    last_sent_seq: u32,
    last_raw_severity: Option<Severity>,
    /// The conclusion currently latched by the de-escalation smoother (Phase 7).
    held: Conclusion,
}

impl ReportSource {
    pub fn new(scenario: Scenario, mission_id: u32, boot_id: u32, hysteresis_debounce_ns: i64) -> Self {
        Self {
            scenario,
            mission_id,
            boot_id,
            hyst: Hysteresis::new(hysteresis_debounce_ns),
            seq: 0,
            last_sent_ns: None,
            last_sent_seq: 0,
            last_raw_severity: None,
            held: Conclusion::default(),
        }
    }

    pub fn last_sent_sequence(&self) -> u32 {
        self.last_sent_seq
    }

    /// Decide whether to emit a report now. `elapsed_ns` is time since scenario
    /// start; `now_ns` a monotonic clock; `last_acked_seq` the monitor's echoed
    /// `last_report_sequence`. Returns the report to send, or `None`.
    ///
    /// v0 reports the scripted conclusion directly; the hysteresis governs the
    /// report RATE (a real noisy AI source in Phase 7 will hysterese the
    /// conclusion itself).
    pub fn tick(
        &mut self,
        elapsed_ns: i64,
        now_ns: i64,
        last_acked_seq: u32,
        tel: SelfTelemetry,
    ) -> Option<CC_HEALTH_REPORT_DATA> {
        let sample = self.scenario.sample_at(elapsed_ns);
        let rate_sev = self.hyst.apply(sample.severity, now_ns);

        // acked once the monitor's echo has caught up to our latest sent seq.
        let acked = self.last_sent_seq != 0 && last_acked_seq >= self.last_sent_seq;
        let interval = policy::interval_ns(rate_sev, acked);

        // send immediately on a severity change (CRITICAL "immediate on
        // transition"); otherwise honor the rate interval.
        let edge = self.last_raw_severity != Some(sample.severity);
        let due = edge || self.last_sent_ns.is_none_or(|t| now_ns - t >= interval);
        if !due {
            return None;
        }

        self.seq = self.seq.wrapping_add(1);
        self.last_sent_ns = Some(now_ns);
        self.last_sent_seq = self.seq;
        self.last_raw_severity = Some(sample.severity);

        Some(CC_HEALTH_REPORT_DATA {
            companion_timestamp_us: (now_ns / 1000).max(0) as u64,
            sequence: self.seq,
            mission_id: self.mission_id,
            companion_boot_id: self.boot_id,
            health_flags: CcHealthFlags::from_bits_truncate(sample.flags),
            detail_code: 0,
            link_rtt_ms: tel.link_rtt_ms,
            telemetry_age_ms: tel.telemetry_age_ms,
            companion_loop_ms: tel.companion_loop_ms,
            dropped_rx_count: tel.dropped_rx_count,
            severity: to_cc_severity(sample.severity),
            recommended_action: to_cc_action(sample.action),
            confidence_percent: sample.confidence,
            schema_version: cc_protocol::identity::CC_SCHEMA_VERSION,
        })
    }

    /// Phase 7: drive the report from a live `cc-ai-health` [`Conclusion`]
    /// instead of a scripted scenario. Unlike v0, the **conclusion itself** is
    /// de-escalation-smoothed — a CRITICAL that flickers off for one 10 Hz tick
    /// does not instantly drop the report to OK (escalation stays immediate).
    /// Carries the conclusion's `detail_code`/`health_flags`/`action`/
    /// `confidence` (v0 hard-coded `detail_code: 0`).
    pub fn tick_with_conclusion(
        &mut self,
        c: Conclusion,
        now_ns: i64,
        last_acked_seq: u32,
        tel: SelfTelemetry,
    ) -> Option<CC_HEALTH_REPORT_DATA> {
        let smoothed = self.hyst.apply(c.severity, now_ns);
        // Keep the reported conclusion consistent with the smoothed severity:
        // adopt the incoming one whenever we are in sync (escalation or a
        // committed de-escalation); while latched at a higher severity, keep
        // the held conclusion so severity/action/detail all correspond.
        if smoothed == c.severity {
            self.held = c;
        }
        let out = self.held;

        let acked = self.last_sent_seq != 0 && last_acked_seq >= self.last_sent_seq;
        let interval = policy::interval_ns(smoothed, acked);
        let edge = self.last_raw_severity != Some(smoothed);
        let due = edge || self.last_sent_ns.is_none_or(|t| now_ns - t >= interval);
        if !due {
            return None;
        }

        self.seq = self.seq.wrapping_add(1);
        self.last_sent_ns = Some(now_ns);
        self.last_sent_seq = self.seq;
        self.last_raw_severity = Some(smoothed);

        Some(CC_HEALTH_REPORT_DATA {
            companion_timestamp_us: (now_ns / 1000).max(0) as u64,
            sequence: self.seq,
            mission_id: self.mission_id,
            companion_boot_id: self.boot_id,
            health_flags: CcHealthFlags::from_bits_truncate(out.health_flags),
            detail_code: out.detail_code,
            link_rtt_ms: tel.link_rtt_ms,
            telemetry_age_ms: tel.telemetry_age_ms,
            companion_loop_ms: tel.companion_loop_ms,
            dropped_rx_count: tel.dropped_rx_count,
            severity: to_cc_severity(smoothed),
            recommended_action: to_cc_action(out.action),
            confidence_percent: out.confidence,
            schema_version: cc_protocol::identity::CC_SCHEMA_VERSION,
        })
    }
}

/// Spawn the health-report transmitter. `ack_rx` carries the monitor's echoed
/// `last_report_sequence` (companiond feeds it from CC_SAFETY_STATUS);
/// `tel_fn` supplies live self-telemetry each tick.
pub fn spawn(
    mut source: ReportSource,
    tx: TxHandle,
    ack_rx: watch::Receiver<u32>,
    tel_fn: impl Fn() -> SelfTelemetry + Send + 'static,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let start = clock::now_ns();
        // 20 Hz tick — fine enough to realize the 5 Hz CRITICAL rate exactly.
        let mut ticker = tokio::time::interval(Duration::from_millis(50));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            let now = clock::now_ns();
            let acked = *ack_rx.borrow();
            if let Some(report) = source.tick(now - start, now, acked, tel_fn()) {
                tx.enqueue(Priority::P1, MavMessage::CC_HEALTH_REPORT(report));
            }
        }
    })
}

/// Phase 7: spawn the health-report transmitter driven by a live
/// `cc-ai-health` [`Conclusion`] (published on `conc_rx` by the Runner task)
/// rather than a scripted scenario. Same pacing/ACK contract as [`spawn`].
pub fn spawn_ai(
    mut source: ReportSource,
    tx: TxHandle,
    ack_rx: watch::Receiver<u32>,
    conc_rx: watch::Receiver<Conclusion>,
    tel_fn: impl Fn() -> SelfTelemetry + Send + 'static,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(50));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let now = clock::now_ns();
            let acked = *ack_rx.borrow();
            let conc = *conc_rx.borrow();
            if let Some(report) = source.tick_with_conclusion(conc, now, acked, tel_fn()) {
                tx.enqueue(Priority::P1, MavMessage::CC_HEALTH_REPORT(report));
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crit_scenario() -> Scenario {
        Scenario::from_toml_str(
            "[[event]]\nt_s=0\nseverity=\"critical\"\naction=\"land\"\nconfidence=90\n",
        )
        .unwrap()
    }

    #[test]
    fn first_report_is_immediate_and_carries_the_conclusion() {
        let mut s = ReportSource::new(crit_scenario(), 42, 7, 0);
        let r = s.tick(0, 0, 0, SelfTelemetry::default()).expect("first report immediate");
        assert_eq!(r.sequence, 1);
        assert_eq!(r.mission_id, 42);
        assert_eq!(r.companion_boot_id, 7);
        assert_eq!(r.severity, CcSeverity::CC_SEVERITY_CRITICAL);
        assert_eq!(r.recommended_action, CcRecommendedAction::CC_ACTION_LAND);
        assert_eq!(r.confidence_percent, 90);
    }

    #[test]
    fn conclusion_carries_detail_code_and_flags() {
        // v0 hard-codes detail_code:0; the AI path forwards the conclusion's.
        let mut s = ReportSource::new(crit_scenario(), 9, 3, 0);
        let c = Conclusion {
            severity: Severity::Warn,
            action: Action::WarnOnly,
            health_flags: 0b101,
            detail_code: 1005,
            confidence: 77,
        };
        let r = s.tick_with_conclusion(c, 0, 0, SelfTelemetry::default()).unwrap();
        assert_eq!(r.severity, CcSeverity::CC_SEVERITY_WARN);
        assert_eq!(r.detail_code, 1005);
        assert_eq!(r.confidence_percent, 77);
        assert_eq!(r.health_flags.bits() & 0b101, 0b101);
    }

    #[test]
    fn conclusion_deescalation_is_smoothed_but_escalation_is_immediate() {
        // 2 s de-escalation debounce
        let mut s = ReportSource::new(crit_scenario(), 1, 1, 2_000_000_000);
        let tel = SelfTelemetry::default();
        let crit = Conclusion {
            severity: Severity::Critical,
            action: Action::Land,
            detail_code: 1005,
            confidence: 95,
            ..Default::default()
        };
        let ok = Conclusion::default();

        // escalate to CRITICAL immediately
        let r = s.tick_with_conclusion(crit, 0, 0, tel).unwrap();
        assert_eq!(r.severity, CcSeverity::CC_SEVERITY_CRITICAL);
        assert_eq!(r.detail_code, 1005);
        // a single OK tick must NOT drop the report — the held CRITICAL persists
        // (edge stays CRITICAL → not due until the 5 Hz interval, so force time)
        let r = s.tick_with_conclusion(ok, 1_000_000_000, 0, tel).unwrap();
        assert_eq!(r.severity, CcSeverity::CC_SEVERITY_CRITICAL, "one OK tick must not drop");
        assert_eq!(r.detail_code, 1005, "held conclusion's detail persists");
        // after the debounce, OK is committed
        let r = s.tick_with_conclusion(ok, 3_500_000_000, 5, tel).unwrap();
        assert_eq!(r.severity, CcSeverity::CC_SEVERITY_OK);
        assert_eq!(r.detail_code, 0);
    }

    #[test]
    fn critical_repeats_at_5hz_until_acked_then_1hz() {
        let mut s = ReportSource::new(crit_scenario(), 1, 1, 0);
        let tel = SelfTelemetry::default();
        // t=0 first report (seq 1), unacked
        assert!(s.tick(0, 0, 0, tel).is_some());
        // 100 ms later: not due (5 Hz = 200 ms), unacked
        assert!(s.tick(100_000_000, 100_000_000, 0, tel).is_none());
        // 200 ms: due (seq 2)
        let r = s.tick(200_000_000, 200_000_000, 0, tel).unwrap();
        assert_eq!(r.sequence, 2);
        // monitor ACKs seq 2 -> drop to 1 Hz: 300 ms after is NOT due
        assert!(s.tick(500_000_000, 500_000_000, 2, tel).is_none());
        // 1 s after the last send (200 ms) -> due again at t=1200 ms
        assert!(s.tick(1_200_000_000, 1_200_000_000, 2, tel).is_some());
    }

    #[test]
    fn ok_scenario_paces_at_1hz() {
        let sc = Scenario::from_toml_str("[[event]]\nt_s=0\nseverity=\"ok\"\n").unwrap();
        let mut s = ReportSource::new(sc, 1, 1, 0);
        let tel = SelfTelemetry::default();
        assert!(s.tick(0, 0, 0, tel).is_some()); // first
        assert!(s.tick(500_000_000, 500_000_000, 1, tel).is_none()); // 0.5s < 1s
        assert!(s.tick(1_000_000_000, 1_000_000_000, 1, tel).is_some()); // 1s
    }

    #[test]
    fn severity_change_forces_an_immediate_report() {
        let sc = Scenario::from_toml_str(
            "[[event]]\nt_s=0\nseverity=\"ok\"\n[[event]]\nt_s=5\nseverity=\"critical\"\naction=\"land\"\n",
        )
        .unwrap();
        let mut s = ReportSource::new(sc, 1, 1, 0);
        let tel = SelfTelemetry::default();
        s.tick(0, 0, 0, tel); // OK seq1
        // 0.3s later while OK: not due (1 Hz)
        assert!(s.tick(300_000_000, 300_000_000, 1, tel).is_none());
        // scenario flips to CRITICAL at 5s: even 0.1s after the last OK send it
        // sends immediately on the edge
        let r = s.tick(5_000_000_000, 5_000_000_000, 1, tel).unwrap();
        assert_eq!(r.severity, CcSeverity::CC_SEVERITY_CRITICAL);
    }
}
