//! `link_quality` (health-flag bit 64).
//!
//! # Reconstructed only from replayable, in-stream fields (deviation D7)
//!
//! Link health is tempting to read from live plumbing, but three obvious sources
//! are **non-replayable** and were struck out by the judges:
//! * `link_rtt_ms` — not a `TelemetryEvent` field at all;
//! * the `IngestStats` atomics — a live side-channel the mission log never
//!   persists, so replay would see different numbers;
//! * `LinkStatus` / `StreamStale` events — dropped by `cc-mission-log`, so they
//!   are absent on replay.
//!
//! Using any of them would make a recorded mission's findings depend on
//! wall-clock timing and diverge on replay. So this lane derives everything from
//! the **two fields that travel inside every persisted message**:
//! `RxMeta.seq_gap` (per-stream sequence discontinuity) and
//! `cc_receive_time_ns` (integer receive time) compared against
//! `StreamId::nominal_period_ns()`.
//!
//! # Indicators (per monitored stream, worst reported)
//! * **drop rate** — EWMA of `seq_gap` (messages missing per received) + an
//!   upward CUSUM for a sudden onset;
//! * **effective-rate ratio** — `nominal_period / mean_inter_arrival`; well
//!   below 1 means messages are arriving slower than the stream's contract;
//! * **jitter** — inter-arrival standard deviation relative to the nominal
//!   period.
//!
//! # Scope
//! This lane reports the **degraded-but-alive** band as `WARN` (advisory). A
//! *fully* dead link is not this lane's job — the deterministic FC safety
//! monitor already owns the STALE/timeout domain. Phase-independent (loss is not
//! manoeuvre-induced); only streams with a nominal period are monitored.

use crate::finding::{flags, AlgoOutput, HealthFinding};
use crate::phase::FlightPhase;
use crate::stats::{clamp01, confidence_percent, logistic};
use crate::stats::{Cusum, CusumTrip, Ewma};
use crate::{detail, EvalCtx, HealthAlgorithm};
use cc_health_tx::{Action, Severity};
use cc_ingest::{RxMeta, StreamId, TelemetryEvent};
use cc_protocol::cc_dialect::CcSubsystem;

const DROP_WARN: f64 = 0.10; // avg messages missing per received (~9 % loss)
const RATE_WARN: f64 = 0.60; // effective rate below 60 % of nominal
const JITTER_WARN: f64 = 1.0; // inter-arrival σ ≈ one nominal period
const WARMUP_MSGS: u64 = 150;

/// Borrow the `RxMeta` + `StreamId` of a persisted telemetry event (the control
/// variants are intentionally ignored — see the module doc).
fn rx_of(ev: &TelemetryEvent) -> Option<(&RxMeta, StreamId)> {
    Some(match ev {
        TelemetryEvent::State(_, m) => (m, StreamId::State),
        TelemetryEvent::Imu(_, m) => (m, StreamId::Imu),
        TelemetryEvent::Power(_, m) => (m, StreamId::Power),
        TelemetryEvent::Gps(_, m) => (m, StreamId::Gps),
        TelemetryEvent::Estimator(_, m) => (m, StreamId::Estimator),
        TelemetryEvent::Actuator(_, m) => (m, StreamId::Actuator),
        TelemetryEvent::Event(_, m) => (m, StreamId::Event),
        TelemetryEvent::SafetyStatus(_, m) => (m, StreamId::SafetyStatus),
        TelemetryEvent::LinkStatus(_) | TelemetryEvent::StreamStale(_) => return None,
    })
}

#[derive(Clone)]
struct StreamLink {
    last_ns: Option<i64>,
    iat: Ewma,       // inter-arrival period (ns)
    gap: Ewma,       // seq_gap per received message
    gap_cusum: Cusum, // upward onset of loss
    msgs: u64,
}
impl StreamLink {
    fn new() -> Self {
        Self {
            last_ns: None,
            iat: Ewma::new(0.05),
            gap: Ewma::new(0.05),
            gap_cusum: Cusum::new(0.5, 4.0),
            msgs: 0,
        }
    }
}

pub struct LinkQuality {
    streams: [StreamLink; 8],
    total_msgs: u64,
}

impl Default for LinkQuality {
    fn default() -> Self {
        Self::new()
    }
}

impl LinkQuality {
    pub fn new() -> Self {
        Self { streams: std::array::from_fn(|_| StreamLink::new()), total_msgs: 0 }
    }

    /// Worst drop fraction over monitored streams and the stream it came from.
    fn worst_drop(&self) -> (f64, StreamId) {
        let mut worst = 0.0;
        let mut who = StreamId::State;
        for s in StreamId::ALL {
            if s.nominal_period_ns().is_none() {
                continue;
            }
            let st = &self.streams[s as usize];
            if st.msgs < 20 {
                continue;
            }
            let g = st.gap.mean().max(0.0);
            let frac = g / (1.0 + g);
            let bumped = if st.gap_cusum.trip() == CusumTrip::Up { frac.max(DROP_WARN) } else { frac };
            if bumped > worst {
                worst = bumped;
                who = s;
            }
        }
        (worst, who)
    }

    /// Worst (lowest) effective-rate ratio over monitored streams.
    fn worst_rate(&self) -> (f64, StreamId) {
        let mut worst = 1.0;
        let mut who = StreamId::State;
        for s in StreamId::ALL {
            let Some(nominal) = s.nominal_period_ns() else { continue };
            let st = &self.streams[s as usize];
            if st.msgs < 20 {
                continue;
            }
            let iat = st.iat.mean();
            if iat > 0.0 {
                let ratio = nominal as f64 / iat;
                if ratio < worst {
                    worst = ratio;
                    who = s;
                }
            }
        }
        (worst, who)
    }

    /// Worst jitter ratio (σ_iat / nominal) over monitored streams.
    fn worst_jitter(&self) -> f64 {
        let mut worst = 0.0;
        for s in StreamId::ALL {
            let Some(nominal) = s.nominal_period_ns() else { continue };
            let st = &self.streams[s as usize];
            if st.msgs < 20 {
                continue;
            }
            let jr = st.iat.std() / nominal as f64;
            if jr > worst {
                worst = jr;
            }
        }
        worst
    }
}

impl HealthAlgorithm for LinkQuality {
    fn subsystem(&self) -> CcSubsystem {
        CcSubsystem::CC_SUBSYS_LINK
    }

    fn on_event(&mut self, ev: &TelemetryEvent, _phase: FlightPhase) {
        let Some((m, sid)) = rx_of(ev) else {
            return;
        };
        if sid.nominal_period_ns().is_none() {
            return; // event-driven streams have no rate contract
        }
        let st = &mut self.streams[sid as usize];
        st.msgs = st.msgs.saturating_add(1);
        self.total_msgs = self.total_msgs.saturating_add(1);

        let gap = m.seq_gap as f64;
        st.gap.update(gap);
        st.gap_cusum.update(gap, 0.0);

        if let Some(prev) = st.last_ns {
            let dt = (m.cc_receive_time_ns - prev) as f64;
            if dt > 0.0 {
                st.iat.update(dt);
            }
        }
        st.last_ns = Some(m.cc_receive_time_ns);
    }

    fn evaluate(&self, _ctx: &EvalCtx) -> AlgoOutput {
        if self.total_msgs == 0 {
            return AlgoOutput::Unavailable(detail::AVAIL_NO_DATA);
        }
        if self.total_msgs < WARMUP_MSGS {
            return AlgoOutput::Degraded(detail::AVAIL_WARMUP);
        }

        let (drop, _dwho) = self.worst_drop();
        let (rate, _rwho) = self.worst_rate();
        let jitter = self.worst_jitter();

        // report the most severe of the three indicators
        if drop >= DROP_WARN {
            let conf = clamp01(0.6 + 0.4 * logistic((drop - DROP_WARN) * 20.0));
            return warn(detail::LINK_DROP_RATE_HIGH, drop, DROP_WARN, conf);
        }
        if rate <= RATE_WARN {
            let conf = clamp01(0.6 + 0.4 * logistic((RATE_WARN - rate) * 5.0));
            return warn(detail::LINK_RATE_BELOW_NOMINAL, rate, RATE_WARN, conf);
        }
        if jitter >= JITTER_WARN {
            return warn(detail::LINK_JITTER_HIGH, jitter, JITTER_WARN, 0.7);
        }
        AlgoOutput::Available(ok())
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

fn ok() -> HealthFinding {
    HealthFinding {
        subsystem: CcSubsystem::CC_SUBSYS_LINK,
        flag_bit: flags::LINK,
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
        subsystem: CcSubsystem::CC_SUBSYS_LINK,
        flag_bit: flags::LINK,
        severity: Severity::Warn,
        action: Action::WarnOnly,
        detail_code: code,
        value: value as f32,
        limit: limit as f32,
        confidence: confidence_percent(conf01),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_ingest::AgeInfo;
    use cc_protocol::cc_dialect::CC_TELEMETRY_STATE_DATA;

    fn ctx(now_ns: i64) -> EvalCtx {
        EvalCtx {
            now_ns,
            phase: FlightPhase::Steady,
            last_seen_ns: [now_ns; 8],
            timesync_locked: true,
        }
    }

    fn state(cc_ns: i64, seq_gap: u32) -> TelemetryEvent {
        let d = CC_TELEMETRY_STATE_DATA { arming_state: 2, schema_version: 1, ..Default::default() };
        TelemetryEvent::State(
            d,
            RxMeta { cc_receive_time_ns: cc_ns, seq_gap, age: AgeInfo::Locked { age_ns: 1 } },
        )
    }

    // State nominal period = 40 ms. Healthy cadence, no gaps.
    fn feed_healthy(l: &mut LinkQuality, n: usize, t0: i64) -> i64 {
        let mut t = t0;
        for _ in 0..n {
            l.on_event(&state(t, 0), FlightPhase::Steady);
            t += 40_000_000;
        }
        t
    }

    #[test]
    fn healthy_link_no_finding() {
        let mut l = LinkQuality::new();
        let t = feed_healthy(&mut l, 300, 0);
        match l.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => assert_eq!(f.severity, Severity::Ok),
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn drop_rate_warns() {
        let mut l = LinkQuality::new();
        let mut t = feed_healthy(&mut l, 200, 0);
        // sustained losses: every message reports a gap of 2
        for _ in 0..120 {
            l.on_event(&state(t, 2), FlightPhase::Steady);
            t += 120_000_000; // 3 missing → 3× the period
        }
        match l.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Warn);
                assert_eq!(f.detail_code, detail::LINK_DROP_RATE_HIGH);
            }
            other => panic!("expected drop warn, got {other:?}"),
        }
    }

    #[test]
    fn rate_below_nominal_warns() {
        let mut l = LinkQuality::new();
        let mut t = feed_healthy(&mut l, 200, 0);
        // no seq gaps, but arrivals slow to ~4× the nominal period
        for _ in 0..120 {
            l.on_event(&state(t, 0), FlightPhase::Steady);
            t += 160_000_000;
        }
        match l.evaluate(&ctx(t)) {
            AlgoOutput::Available(f) => {
                assert_eq!(f.severity, Severity::Warn);
                assert_eq!(f.detail_code, detail::LINK_RATE_BELOW_NOMINAL);
            }
            other => panic!("expected rate warn, got {other:?}"),
        }
    }

    #[test]
    fn no_data_is_unavailable() {
        let l = LinkQuality::new();
        assert!(matches!(
            l.evaluate(&ctx(0)),
            AlgoOutput::Unavailable(detail::AVAIL_NO_DATA)
        ));
    }

    #[test]
    fn deterministic_repeat() {
        let run = || {
            let mut l = LinkQuality::new();
            let mut t = feed_healthy(&mut l, 200, 0);
            for _ in 0..120 {
                l.on_event(&state(t, 2), FlightPhase::Steady);
                t += 120_000_000;
            }
            match l.evaluate(&ctx(t)) {
                AlgoOutput::Available(f) => (f.severity as u8, f.detail_code, f.value.to_bits()),
                _ => (255, 0, 0),
            }
        };
        assert_eq!(run(), run());
    }
}
