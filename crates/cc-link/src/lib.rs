//! # cc-link — FC↔CC link layer (spec §5.3)
//!
//! Owns exactly one transport at a time (UDP for SITL/Ethernet, serial for
//! the TELEM3 bench link in Phase 8 — same interface). Responsibilities:
//!
//! * **RX**: bytes → [`cc_protocol::CcFrameDecoder`] (the Phase 1
//!   fuzz-proven incremental parser: resync-on-0xFD, exact fault counters)
//!   → per-frame source check → bounded channel to the consumer
//!   (companiond's demux → cc-ingest).
//! * **TX**: four bounded queues drained in **strict priority** order,
//!   preemptive at frame boundary (P0 HEARTBEAT/TIMESYNC/health-reports,
//!   P1 session, P2 diagnostics, P3 bulk). A full P0 queue is a
//!   **link-down condition**, not a queueing condition (spec): it marks
//!   the link DOWN and counts `p0_stalls` — P0 is never silently dropped.
//! * **Companion HEARTBEAT** at 1 Hz (`MAV_TYPE_ONBOARD_CONTROLLER`,
//!   comp 191, spec §3.5).
//! * **Link state** UP/DEGRADED/DOWN from FC-heartbeat age and CRC-error
//!   deltas, published on a `watch` channel.
//!
//! The shared process-monotonic clock ([`clock`]) lives here because every
//! Phase 4 crate stamps against it (`cc_receive_time_ns` in the identity
//! envelope, spec §3.4).

pub mod clock;
pub mod transport;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cc_protocol::cc_dialect::{HEARTBEAT_DATA, MavAutopilot, MavModeFlag, MavState, MavType};
use cc_protocol::framing::DecodedFrame;
use cc_protocol::mavlink_core::{write_v2_msg, MavHeader};
use cc_protocol::{identity, validate, CcFrameDecoder, CcMavMessage};
use tokio::sync::{mpsc, watch};

use transport::{RxHalf, TxHalf};

/// TX priority classes (spec §5.3). Strict: a frame of priority N is only
/// sent when every queue < N is empty at that frame boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    /// HEARTBEAT, TIMESYNC, CC_HEALTH_REPORT — never dropped.
    P0,
    /// CC_MISSION_CONTEXT, acks/session.
    P1,
    /// CC_AI_DIAGNOSTIC.
    P2,
    /// Bulk/debug (development only).
    P3,
}

/// Link state derived from FC heartbeat age + CRC error rate (spec §5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    /// No FC heartbeat seen yet (or since going down).
    Down,
    /// Heartbeats fresh, error rate nominal.
    Up,
    /// Heartbeat aging (> 2.5 s) or CRC errors accumulating.
    Degraded,
}

/// Published on the link watch channel and forwarded into the
/// `TelemetryEvent` stream by cc-ingest (spec §5.5 `LinkStatus`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinkStatus {
    pub state: LinkState,
    /// Age of the newest FC HEARTBEAT, ns (None until the first one).
    pub fc_heartbeat_age_ns: Option<i64>,
    /// Cumulative CRC errors at the time of this status.
    pub crc_errors: u64,
}

/// Cumulative link counters (atomics: RX task writes, anyone reads).
/// Framing-level counts live in the decoder and are merged into
/// [`LinkStatsSnapshot`] — together they cover the spec §5.3 counter list
/// (`rx_frames`, `rx_crc_errors`, `rx_bad_source`, `rx_unknown_msg`).
#[derive(Debug, Default)]
pub struct LinkCounters {
    pub rx_bad_source: AtomicU64,
    pub tx_frames: AtomicU64,
    pub tx_errors: AtomicU64,
    pub p0_stalls: AtomicU64,
    pub rx_channel_drops: AtomicU64,
}

/// Point-in-time merged view of decoder + link counters.
#[derive(Debug, Clone, Default)]
pub struct LinkStatsSnapshot {
    pub frames_ok: u64,
    pub crc_errors: u64,
    pub garbage_bytes: u64,
    pub unknown_msg_ids: u64,
    pub bad_payloads: u64,
    pub rx_bad_source: u64,
    pub tx_frames: u64,
    pub tx_errors: u64,
    pub p0_stalls: u64,
    pub rx_channel_drops: u64,
}

/// One frame handed to the consumer (decoded message + wire header).
pub type LinkFrame = DecodedFrame<CcMavMessage>;

/// Handle for enqueueing outbound messages at a given priority.
#[derive(Clone)]
pub struct TxHandle {
    queues: [mpsc::Sender<CcMavMessage>; 4],
    counters: Arc<LinkCounters>,
    link_down_tx: watch::Sender<bool>,
}

impl TxHandle {
    /// Enqueue a message. P0 uses `try_send` and treats a full queue as a
    /// link-down condition (counted, surfaced); P1–P3 drop-and-count when
    /// full (bounded channels, spec §5.2 overflow policy).
    pub fn enqueue(&self, prio: Priority, msg: CcMavMessage) {
        let idx = prio as usize;
        match self.queues[idx].try_send(msg) {
            Ok(()) => {}
            Err(_) if prio == Priority::P0 => {
                self.counters.p0_stalls.fetch_add(1, Ordering::Relaxed);
                let _ = self.link_down_tx.send(true);
            }
            Err(_) => {
                self.counters.tx_errors.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

/// Everything a consumer needs from a spawned link.
pub struct Link {
    frames: Option<mpsc::Receiver<LinkFrame>>,
    pub tx: TxHandle,
    pub status: watch::Receiver<LinkStatus>,
    pub counters: Arc<LinkCounters>,
    stats_decoder: Arc<std::sync::Mutex<cc_protocol::framing::DecodeCounters>>,
}

impl Link {
    /// Take ownership of the inbound frame channel (once).
    pub fn take_frames(&mut self) -> mpsc::Receiver<LinkFrame> {
        self.frames.take().expect("frames already taken")
    }

    pub fn stats(&self) -> LinkStatsSnapshot {
        let d = self.stats_decoder.lock().unwrap().clone();
        LinkStatsSnapshot {
            frames_ok: d.frames_ok,
            crc_errors: d.crc_errors,
            garbage_bytes: d.garbage_bytes,
            unknown_msg_ids: d.unknown_msg_ids,
            bad_payloads: d.bad_payloads,
            rx_bad_source: self.counters.rx_bad_source.load(Ordering::Relaxed),
            tx_frames: self.counters.tx_frames.load(Ordering::Relaxed),
            tx_errors: self.counters.tx_errors.load(Ordering::Relaxed),
            p0_stalls: self.counters.p0_stalls.load(Ordering::Relaxed),
            rx_channel_drops: self.counters.rx_channel_drops.load(Ordering::Relaxed),
        }
    }
}

/// Queue depths: P0 small by design (it must always drain fast; a deep P0
/// only hides a dead link), the rest sized for bursts.
const P0_DEPTH: usize = 16;
const P_DEPTH: usize = 128;
/// Consumer channel depth (RX → ingest).
const FRAME_DEPTH: usize = 512;

/// Spawn the link tasks (RX, TX, heartbeat, state) on the current runtime.
/// `peer_tx` comes from the transport constructor; the RX task learns the
/// UDP peer only from datagrams that decode to at least one valid frame.
pub fn spawn(
    rx_half: RxHalf,
    tx_half: TxHalf,
    peer_tx: tokio::sync::watch::Sender<Option<std::net::SocketAddr>>,
    sysid: u8,
) -> Link {
    let counters = Arc::new(LinkCounters::default());
    let stats_decoder = Arc::new(std::sync::Mutex::new(
        cc_protocol::framing::DecodeCounters::default(),
    ));
    let (frame_tx, frame_rx) = mpsc::channel::<LinkFrame>(FRAME_DEPTH);
    let (status_tx, status_rx) = watch::channel(LinkStatus {
        state: LinkState::Down,
        fc_heartbeat_age_ns: None,
        crc_errors: 0,
    });
    let (link_down_tx, link_down_rx) = watch::channel(false);

    let q: Vec<(mpsc::Sender<CcMavMessage>, mpsc::Receiver<CcMavMessage>)> =
        (0..4).map(|i| mpsc::channel(if i == 0 { P0_DEPTH } else { P_DEPTH })).collect();
    let mut q = q.into_iter();
    let (p0s, p0r) = q.next().unwrap();
    let (p1s, p1r) = q.next().unwrap();
    let (p2s, p2r) = q.next().unwrap();
    let (p3s, p3r) = q.next().unwrap();

    let tx_handle = TxHandle {
        queues: [p0s, p1s, p2s, p3s],
        counters: counters.clone(),
        link_down_tx,
    };

    // shared "ns timestamp of last FC heartbeat" (0 = never)
    let last_fc_hb = Arc::new(AtomicU64::new(0));

    tokio::spawn(rx_task(
        rx_half,
        frame_tx,
        counters.clone(),
        stats_decoder.clone(),
        last_fc_hb.clone(),
        peer_tx,
        sysid,
    ));
    tokio::spawn(tx_task(tx_half, [p0r, p1r, p2r, p3r], counters.clone(), sysid));
    tokio::spawn(heartbeat_task(tx_handle.clone()));
    tokio::spawn(state_task(status_tx, last_fc_hb, stats_decoder.clone(), link_down_rx));

    Link {
        frames: Some(frame_rx),
        tx: tx_handle,
        status: status_rx,
        counters,
        stats_decoder,
    }
}

async fn rx_task(
    mut rx: RxHalf,
    frame_tx: mpsc::Sender<LinkFrame>,
    counters: Arc<LinkCounters>,
    stats_decoder: Arc<std::sync::Mutex<cc_protocol::framing::DecodeCounters>>,
    last_fc_hb: Arc<AtomicU64>,
    peer_tx: watch::Sender<Option<std::net::SocketAddr>>,
    sysid: u8,
) {
    let mut decoder = CcFrameDecoder::new();
    let mut buf = vec![0u8; 65536];

    loop {
        let (n, from) = match rx.recv(&mut buf).await {
            Ok((0, _)) => continue,
            Ok(x) => x,
            Err(_) => {
                // transport error: back off briefly and retry (serial
                // reconnect with backoff is Phase 8 bench work; UDP recv
                // errors are transient)
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };

        let mut decoded_any = false;

        for frame in decoder.push(&buf[..n]) {
            decoded_any = true;
            // spec §3.3 source gate: FC-originated dialect messages must
            // come from (sysid, comp 1); our own message class looping
            // back is a config error. Standard msgs checked for sysid.
            if validate::validate_source_on_cc(&frame.header, sysid, frame.msg_id).is_err() {
                counters.rx_bad_source.fetch_add(1, Ordering::Relaxed);
                continue;
            }

            if matches!(frame.message, CcMavMessage::HEARTBEAT(_)) {
                last_fc_hb.store(clock::now_ns() as u64, Ordering::Relaxed);
            }

            if frame_tx.try_send(frame).is_err() {
                counters.rx_channel_drops.fetch_add(1, Ordering::Relaxed);
            }
        }

        // peer learning gated on a valid decode: garbage from a stranger
        // cannot redirect our TX away from PX4
        if decoded_any {
            if let Some(addr) = from {
                if peer_tx.borrow().as_ref() != Some(&addr) {
                    let _ = peer_tx.send(Some(addr));
                }
            }
        }

        *stats_decoder.lock().unwrap() = decoder.counters().clone();
    }
}

async fn tx_task(
    mut tx: TxHalf,
    queues: [mpsc::Receiver<CcMavMessage>; 4],
    counters: Arc<LinkCounters>,
    sysid: u8,
) {
    let [mut p0, mut p1, mut p2, mut p3] = queues;
    let mut seq: u8 = 0;
    let mut out = Vec::with_capacity(300);

    loop {
        // Strict priority, preemptive at frame boundary (spec §5.3):
        // a biased select polls the arms top-down on every wakeup, so
        // whenever P0 has a frame ready it always wins; lower classes only
        // transmit when everything above them is empty at this boundary.
        // (Channels never close — TxHandle clones hold all senders.)
        let msg = tokio::select! {
            biased;
            Some(m) = p0.recv() => m,
            Some(m) = p1.recv() => m,
            Some(m) = p2.recv() => m,
            Some(m) = p3.recv() => m,
        };

        out.clear();
        let header = MavHeader {
            system_id: sysid,
            component_id: identity::COMPID_CC,
            sequence: seq,
        };
        seq = seq.wrapping_add(1);

        if write_v2_msg(&mut out, header, &msg).is_ok() && tx.send(&out).await.is_ok() {
            counters.tx_frames.fetch_add(1, Ordering::Relaxed);
        } else {
            counters.tx_errors.fetch_add(1, Ordering::Relaxed);
        }
    }
}

async fn heartbeat_task(tx: TxHandle) {
    // spec §3.5: CC → PX4 standard HEARTBEAT at 1 Hz,
    // type ONBOARD_CONTROLLER, autopilot INVALID
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    loop {
        tick.tick().await;
        tx.enqueue(
            Priority::P0,
            CcMavMessage::HEARTBEAT(HEARTBEAT_DATA {
                custom_mode: 0,
                mavtype: MavType::MAV_TYPE_ONBOARD_CONTROLLER,
                autopilot: MavAutopilot::MAV_AUTOPILOT_INVALID,
                base_mode: MavModeFlag::empty(),
                system_status: MavState::MAV_STATE_ACTIVE,
                mavlink_version: 3,
            }),
        );
    }
}

/// Heartbeat-age thresholds (spec §5.3 link state).
const HB_DEGRADED_NS: i64 = 2_500_000_000;
const HB_DOWN_NS: i64 = 5_000_000_000;

async fn state_task(
    status_tx: watch::Sender<LinkStatus>,
    last_fc_hb: Arc<AtomicU64>,
    stats_decoder: Arc<std::sync::Mutex<cc_protocol::framing::DecodeCounters>>,
    mut link_down_rx: watch::Receiver<bool>,
) {
    let mut tick = tokio::time::interval(Duration::from_millis(500));
    loop {
        tokio::select! {
            _ = tick.tick() => {}
            _ = link_down_rx.changed() => {}
        }

        let hb = last_fc_hb.load(Ordering::Relaxed);
        let crc = stats_decoder.lock().unwrap().crc_errors;
        let p0_stalled = *link_down_rx.borrow();

        let (state, age) = if hb == 0 {
            (LinkState::Down, None)
        } else {
            let age = clock::now_ns() - hb as i64;
            let s = if p0_stalled || age > HB_DOWN_NS {
                LinkState::Down
            } else if age > HB_DEGRADED_NS {
                LinkState::Degraded
            } else {
                LinkState::Up
            };
            (s, Some(age))
        };

        // send_replace never notifies-on-equal subtleties: watch notifies
        // on every send; consumers deduplicate on the state field
        let _ = status_tx.send_replace(LinkStatus { state, fc_heartbeat_age_ns: age, crc_errors: crc });
    }
}
