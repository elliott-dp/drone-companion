//! # cc-ingest — validate → normalize → fan-out (spec §5.5)
//!
//! Consumes decoded frames from cc-link and produces the single
//! [`TelemetryEvent`] stream every downstream task subscribes to
//! (mission log and health AI in later phases; the status display today).
//!
//! Per frame: schema validation (drop + count on mismatch, spec §4.4/§5.5)
//! → stream classification → **sequence continuity** (gaps counted once,
//! attached to the event's [`RxMeta`], never per-row spam) → **age**
//! against the timesync snapshot (flagged [`AgeInfo::UnknownOffset`] unless
//! LOCKED — missing data is missing, invariant 7) → broadcast.
//!
//! Stream **watchdogs**: a Class A–F stream silent for > 4× its nominal
//! period emits [`TelemetryEvent::StreamStale`] once on entry (spec §5.5);
//! resuming data clears the flag. `px4_boot_id` changes reset all sequence
//! trackers (sequences restart per FC boot, spec §4.2) and are published on
//! a watch channel (cc-timesync invalidates from it).

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cc_link::{clock, LinkFrame, LinkStatus};
use cc_protocol::cc_dialect::*;
use cc_protocol::validate;
use cc_timesync::{Quality, Snapshot};
use tokio::sync::{broadcast, mpsc, watch};

/// The FC→CC streams of the telemetry contract (spec §6 classes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamId {
    State = 0,
    Imu = 1,
    Power = 2,
    Gps = 3,
    Estimator = 4,
    Actuator = 5,
    Event = 6,
    SafetyStatus = 7,
}

impl StreamId {
    pub const ALL: [StreamId; 8] = [
        StreamId::State, StreamId::Imu, StreamId::Power, StreamId::Gps,
        StreamId::Estimator, StreamId::Actuator, StreamId::Event, StreamId::SafetyStatus,
    ];

    pub fn name(self) -> &'static str {
        match self {
            StreamId::State => "state",
            StreamId::Imu => "imu",
            StreamId::Power => "power",
            StreamId::Gps => "gps",
            StreamId::Estimator => "estimator",
            StreamId::Actuator => "actuator",
            StreamId::Event => "event",
            StreamId::SafetyStatus => "safety_status",
        }
    }

    /// Nominal AI_UART period (spec §6; actuator per the divider rule).
    /// Watchdog threshold = 4× this (spec §5.5). Event/SafetyStatus are
    /// event-driven — no watchdog.
    pub fn nominal_period_ns(self) -> Option<i64> {
        match self {
            StreamId::State => Some(40_000_000),
            StreamId::Imu => Some(20_000_000),
            StreamId::Power => Some(100_000_000),
            StreamId::Gps => Some(200_000_000),
            StreamId::Estimator => Some(100_000_000),
            StreamId::Actuator => Some(60_000_000),
            StreamId::Event | StreamId::SafetyStatus => None,
        }
    }
}

/// Age of the payload relative to the CC clock at classification time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AgeInfo {
    /// Timesync LOCKED: `age_ns = now_cc − to_cc(fc_timestamp)`.
    Locked { age_ns: i64 },
    /// Not locked — receive time only; never fabricated (spec §5.5/§11).
    UnknownOffset,
}

/// Receive-side identity envelope carried beside every payload
/// (spec §3.4: cc_receive_time_ns, per-stream sequence bookkeeping).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RxMeta {
    pub cc_receive_time_ns: i64,
    /// Sequence gap detected AT this message (0 = contiguous).
    pub seq_gap: u32,
    pub age: AgeInfo,
}

/// The single fan-out type (spec §5.5), payload variants carrying the
/// generated wire structs + [`RxMeta`] (deviation D17: the spec sketch
/// shows bare payloads; the identity envelope has to travel with them).
#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    State(CC_TELEMETRY_STATE_DATA, RxMeta),
    Imu(CC_TELEMETRY_IMU_DATA, RxMeta),
    Power(CC_TELEMETRY_POWER_DATA, RxMeta),
    Gps(CC_TELEMETRY_GPS_DATA, RxMeta),
    Actuator(CC_TELEMETRY_ACTUATOR_DATA, RxMeta),
    Estimator(CC_TELEMETRY_ESTIMATOR_DATA, RxMeta),
    SafetyStatus(CC_SAFETY_STATUS_DATA, RxMeta),
    Event(CC_EVENT_DATA, RxMeta),
    LinkStatus(LinkStatus),
    StreamStale(StreamId),
}

/// Cumulative per-stream + global counters (atomics; ingest writes, the
/// status task reads).
#[derive(Debug, Default)]
pub struct IngestStats {
    pub count: [AtomicU64; 8],
    pub gaps: [AtomicU64; 8],
    pub last_rx_ns: [AtomicI64; 8],
    pub stale: [AtomicBool; 8],
    pub bad_schema: AtomicU64,
    pub mission_id: AtomicU32,
}

impl IngestStats {
    pub fn stream_count(&self, s: StreamId) -> u64 {
        self.count[s as usize].load(Ordering::Relaxed)
    }
    pub fn stream_gaps(&self, s: StreamId) -> u64 {
        self.gaps[s as usize].load(Ordering::Relaxed)
    }
    pub fn stream_stale(&self, s: StreamId) -> bool {
        self.stale[s as usize].load(Ordering::Relaxed)
    }
    pub fn total_gaps(&self) -> u64 {
        StreamId::ALL.iter().map(|s| self.stream_gaps(*s)).sum()
    }
}

pub struct Ingest {
    pub events: broadcast::Sender<TelemetryEvent>,
    pub stats: Arc<IngestStats>,
}

/// Broadcast capacity: slow subscribers lag (drop oldest) rather than
/// blocking the pipeline — spec §5.2's "no task may block the RX path".
const BROADCAST_DEPTH: usize = 1024;
/// Watchdog sweep period.
const WATCHDOG_TICK: Duration = Duration::from_millis(100);
/// Stale threshold multiplier (spec §5.5: "> 4 × its nominal period").
const STALE_FACTOR: i64 = 4;

/// `boot_tx` is created by the caller (companiond) so its receiver can be
/// handed to cc-timesync before ingest exists — the two crates otherwise
/// depend on each other's watches.
pub fn spawn(
    mut frames: mpsc::Receiver<LinkFrame>,
    ts_snapshot: watch::Receiver<Snapshot>,
    mut link_status: watch::Receiver<LinkStatus>,
    boot_tx: watch::Sender<u32>,
) -> Ingest {
    let (event_tx, _keepalive_rx) = broadcast::channel(BROADCAST_DEPTH);
    let stats = Arc::new(IngestStats::default());

    let tx = event_tx.clone();
    let st = stats.clone();

    tokio::spawn(async move {
        // per-stream sequence trackers (None = no message this boot yet)
        let mut last_seq: [Option<u32>; 8] = [None; 8];
        // staleness bookkeeping on the RUNTIME clock (tokio Instants):
        // virtual-clock-correct under test, identical to wall time in prod
        let mut last_seen: [Option<tokio::time::Instant>; 8] = [None; 8];
        let mut watchdog = tokio::time::interval(WATCHDOG_TICK);

        loop {
            tokio::select! {
                maybe = frames.recv() => {
                    let Some(frame) = maybe else { return; };
                    if let Some(stream) = handle_frame(frame, &tx, &st, &mut last_seq, &ts_snapshot, &boot_tx) {
                        last_seen[stream as usize] = Some(tokio::time::Instant::now());
                    }
                }

                _ = watchdog.tick() => {
                    sweep_watchdogs(&tx, &st, &last_seen);
                }

                res = link_status.changed() => {
                    if res.is_err() { return; }
                    let status = *link_status.borrow();
                    let _ = tx.send(TelemetryEvent::LinkStatus(status));
                }
            }
        }
    });

    Ingest { events: event_tx, stats }
}

fn handle_frame(
    frame: LinkFrame,
    tx: &broadcast::Sender<TelemetryEvent>,
    stats: &IngestStats,
    last_seq: &mut [Option<u32>; 8],
    ts: &watch::Receiver<Snapshot>,
    boot_tx: &watch::Sender<u32>,
) -> Option<StreamId> {
    let now = clock::now_ns();

    // schema gate (spec §4.4 order as it applies CC-side; source was
    // checked in cc-link). Standard messages (no schema field) pass.
    if validate::validate_schema(frame.msg_id, &frame.message).is_err() {
        stats.bad_schema.fetch_add(1, Ordering::Relaxed);
        return None;
    }

    use MavMessage as M;
    match frame.message {
        M::CC_TELEMETRY_STATE(m) => {
            // boot identity: a changed px4_boot_id means the FC rebooted —
            // per-stream sequences restart (spec §4.2), timesync must
            // invalidate (watch consumers), segments split in Phase 5
            if *boot_tx.borrow() != m.px4_boot_id {
                *last_seq = [None; 8];
                let _ = boot_tx.send(m.px4_boot_id);
            }
            stats.mission_id.store(m.mission_id, Ordering::Relaxed);
            emit(StreamId::State, Some(m.sequence), m.fc_timestamp_us, now,
                 |meta| TelemetryEvent::State(m, meta), tx, stats, last_seq, ts);
            Some(StreamId::State)
        }
        M::CC_TELEMETRY_IMU(m) => {
            emit(StreamId::Imu, Some(m.sequence), m.fc_timestamp_us, now,
                 |meta| TelemetryEvent::Imu(m, meta), tx, stats, last_seq, ts);
            Some(StreamId::Imu)
        }
        M::CC_TELEMETRY_POWER(m) => {
            emit(StreamId::Power, Some(m.sequence), m.fc_timestamp_us, now,
                 |meta| TelemetryEvent::Power(m, meta), tx, stats, last_seq, ts);
            Some(StreamId::Power)
        }
        M::CC_TELEMETRY_GPS(m) => {
            emit(StreamId::Gps, Some(m.sequence), m.fc_timestamp_us, now,
                 |meta| TelemetryEvent::Gps(m, meta), tx, stats, last_seq, ts);
            Some(StreamId::Gps)
        }
        M::CC_TELEMETRY_ESTIMATOR(m) => {
            emit(StreamId::Estimator, Some(m.sequence), m.fc_timestamp_us, now,
                 |meta| TelemetryEvent::Estimator(m, meta), tx, stats, last_seq, ts);
            Some(StreamId::Estimator)
        }
        M::CC_TELEMETRY_ACTUATOR(m) => {
            emit(StreamId::Actuator, Some(m.sequence), m.fc_timestamp_us, now,
                 |meta| TelemetryEvent::Actuator(m, meta), tx, stats, last_seq, ts);
            Some(StreamId::Actuator)
        }
        M::CC_EVENT(m) => {
            // sequence sourced from PX4's u16 event counter (doc D12):
            // wraps early; the wrapping-diff gap logic treats the wrap as
            // a regression and simply doesn't count a gap there
            emit(StreamId::Event, Some(m.sequence), m.fc_timestamp_us, now,
                 |meta| TelemetryEvent::Event(m, meta), tx, stats, last_seq, ts);
            Some(StreamId::Event)
        }
        M::CC_SAFETY_STATUS(m) => {
            // event-driven monitor echo: carries no per-stream sequence
            // (its last_report_sequence is the report ACK, spec §4.5)
            emit(StreamId::SafetyStatus, None, m.fc_timestamp_us, now,
                 |meta| TelemetryEvent::SafetyStatus(m, meta), tx, stats, last_seq, ts);
            Some(StreamId::SafetyStatus)
        }
        // HEARTBEAT/TIMESYNC and anything else standard: consumed upstream
        // (link heartbeat clock, companiond demux) — nothing to fan out
        _ => None,
    }
}

/// Continuity + age + broadcast for one classified payload.
#[allow(clippy::too_many_arguments)]
fn emit<F: FnOnce(RxMeta) -> TelemetryEvent>(
    stream: StreamId,
    seq: Option<u32>,
    fc_timestamp_us: u64,
    now: i64,
    make: F,
    tx: &broadcast::Sender<TelemetryEvent>,
    stats: &IngestStats,
    last_seq: &mut [Option<u32>; 8],
    ts: &watch::Receiver<Snapshot>,
) {
    let idx = stream as usize;

    // sequence continuity (spec §5.5): wrapping diff; gap counted once and
    // attached to THIS event's meta; duplicates/regressions add no gap
    let gap = match (seq, last_seq[idx]) {
        (Some(s), Some(prev)) => {
            let d = s.wrapping_sub(prev);
            if d == 0 || d > u32::MAX / 2 { 0 } else { d - 1 }
        }
        _ => 0,
    };
    if let Some(s) = seq {
        last_seq[idx] = Some(s);
    }
    if gap > 0 {
        stats.gaps[idx].fetch_add(u64::from(gap), Ordering::Relaxed);
    }
    stats.count[idx].fetch_add(1, Ordering::Relaxed);
    stats.last_rx_ns[idx].store(now, Ordering::Relaxed);

    // age (spec §5.5): only when timesync is LOCKED; otherwise flagged —
    // never fabricated (invariant 7)
    let snap = *ts.borrow();
    let age = if snap.quality == Quality::Locked {
        AgeInfo::Locked { age_ns: now - snap.fc_us_to_cc_ns(fc_timestamp_us) }
    } else {
        AgeInfo::UnknownOffset
    };

    let _ = tx.send(make(RxMeta { cc_receive_time_ns: now, seq_gap: gap, age }));
}

fn sweep_watchdogs(
    tx: &broadcast::Sender<TelemetryEvent>,
    stats: &IngestStats,
    last_seen: &[Option<tokio::time::Instant>; 8],
) {
    let now = tokio::time::Instant::now();
    for s in StreamId::ALL {
        let Some(period) = s.nominal_period_ns() else { continue };
        let Some(last) = last_seen[s as usize] else {
            continue; // never seen — "absent", not "stale"
        };
        let is_stale = now.duration_since(last).as_nanos() as i64 > STALE_FACTOR * period;
        let was_stale = stats.stale[s as usize].swap(is_stale, Ordering::Relaxed);
        if is_stale && !was_stale {
            let _ = tx.send(TelemetryEvent::StreamStale(s));
        }
    }
}
