//! cc-ingest behavior tests (dev plan Phase 4.3): sequence continuity,
//! boot-id reset, schema gate, staleness watchdogs. Deterministic —
//! fabricated frames, no sockets; the watchdog test runs under tokio's
//! paused clock.

use cc_ingest::{spawn, AgeInfo, StreamId, TelemetryEvent};
use cc_link::{LinkState, LinkStatus};
use cc_protocol::cc_dialect::*;
use cc_protocol::framing::DecodedFrame;
use cc_protocol::identity;
use cc_protocol::mavlink_core::MavHeader;
use cc_timesync::{Quality, Snapshot};
use tokio::sync::{mpsc, watch};
use tokio::time::{advance, timeout, Duration};

fn state_frame(seq: u32, boot: u32) -> DecodedFrame<MavMessage> {
    let d = CC_TELEMETRY_STATE_DATA {
        sequence: seq,
        px4_boot_id: boot,
        mission_id: 777,
        fc_timestamp_us: 1000 + u64::from(seq),
        schema_version: identity::CC_SCHEMA_VERSION,
        ..Default::default()
    };
    frame(MavMessage::CC_TELEMETRY_STATE(d), 54000)
}

fn imu_frame(seq: u32, schema: u8) -> DecodedFrame<MavMessage> {
    let d = CC_TELEMETRY_IMU_DATA {
        sequence: seq,
        fc_timestamp_us: 2000 + u64::from(seq),
        schema_version: schema,
        ..Default::default()
    };
    frame(MavMessage::CC_TELEMETRY_IMU(d), 54001)
}

fn frame(message: MavMessage, msg_id: u32) -> DecodedFrame<MavMessage> {
    DecodedFrame {
        header: MavHeader { system_id: 1, component_id: identity::COMPID_FC, sequence: 0 },
        message,
        msg_id,
        payload_len: 0,
        signed: false,
        frame_len: 0,
    }
}

struct Rig {
    frames: mpsc::Sender<DecodedFrame<MavMessage>>,
    events: tokio::sync::broadcast::Receiver<TelemetryEvent>,
    stats: std::sync::Arc<cc_ingest::IngestStats>,
    boot_rx: watch::Receiver<u32>,
    ts_tx: watch::Sender<Snapshot>,
    _link_tx: watch::Sender<LinkStatus>,
}

fn rig() -> Rig {
    let (frame_tx, frame_rx) = mpsc::channel(64);
    let (ts_tx, ts_rx) = watch::channel(Snapshot::UNLOCKED);
    let (link_tx, link_rx) = watch::channel(LinkStatus {
        state: LinkState::Down,
        fc_heartbeat_age_ns: None,
        crc_errors: 0,
    });
    let (boot_tx, boot_rx) = watch::channel(0u32);
    let ingest = spawn(frame_rx, ts_rx, link_rx, boot_tx);
    Rig {
        frames: frame_tx,
        events: ingest.events.subscribe(),
        stats: ingest.stats,
        boot_rx,
        ts_tx,
        _link_tx: link_tx,
    }
}

async fn next_event(rx: &mut tokio::sync::broadcast::Receiver<TelemetryEvent>) -> TelemetryEvent {
    timeout(Duration::from_secs(5), rx.recv()).await.expect("event timeout").expect("closed")
}

#[tokio::test]
async fn continuity_gaps_counted_once_and_attached() {
    let mut r = rig();

    r.frames.send(state_frame(10, 42)).await.unwrap();
    match next_event(&mut r.events).await {
        TelemetryEvent::State(d, meta) => {
            assert_eq!(d.sequence, 10);
            assert_eq!(meta.seq_gap, 0, "first message of a boot never counts a gap");
        }
        other => panic!("unexpected {other:?}"),
    }

    r.frames.send(state_frame(11, 42)).await.unwrap();
    match next_event(&mut r.events).await {
        TelemetryEvent::State(_, meta) => assert_eq!(meta.seq_gap, 0),
        other => panic!("unexpected {other:?}"),
    }

    // jump 11 -> 15: gap of 3, counted once, attached to this event
    r.frames.send(state_frame(15, 42)).await.unwrap();
    match next_event(&mut r.events).await {
        TelemetryEvent::State(_, meta) => assert_eq!(meta.seq_gap, 3),
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(r.stats.stream_gaps(StreamId::State), 3);

    // duplicate + regression add nothing
    r.frames.send(state_frame(15, 42)).await.unwrap();
    next_event(&mut r.events).await;
    r.frames.send(state_frame(14, 42)).await.unwrap();
    next_event(&mut r.events).await;
    assert_eq!(r.stats.stream_gaps(StreamId::State), 3);
}

#[tokio::test]
async fn boot_change_resets_sequences_and_publishes_watch() {
    let mut r = rig();

    r.frames.send(state_frame(5000, 1111)).await.unwrap();
    next_event(&mut r.events).await;
    assert_eq!(*r.boot_rx.borrow(), 1111);

    // FC reboot: sequences restart low — must NOT count a giant gap
    r.frames.send(state_frame(0, 2222)).await.unwrap();
    match next_event(&mut r.events).await {
        TelemetryEvent::State(d, meta) => {
            assert_eq!(d.px4_boot_id, 2222);
            assert_eq!(meta.seq_gap, 0, "boot change must reset continuity");
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(*r.boot_rx.borrow(), 2222);
    assert_eq!(r.stats.stream_gaps(StreamId::State), 0);
    assert_eq!(
        r.stats.mission_id.load(std::sync::atomic::Ordering::Relaxed),
        777
    );
}

#[tokio::test]
async fn schema_mismatch_dropped_and_counted() {
    let mut r = rig();

    r.frames.send(imu_frame(1, 99)).await.unwrap(); // wrong schema
    r.frames.send(imu_frame(2, identity::CC_SCHEMA_VERSION)).await.unwrap();

    // only the valid one arrives
    match next_event(&mut r.events).await {
        TelemetryEvent::Imu(d, _) => assert_eq!(d.sequence, 2),
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(r.stats.bad_schema.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert_eq!(r.stats.stream_count(StreamId::Imu), 1);
}

#[tokio::test]
async fn age_flags_follow_timesync_quality() {
    let mut r = rig();

    // UNLOCKED -> UnknownOffset (never fabricated, invariant 7)
    r.frames.send(state_frame(1, 7)).await.unwrap();
    match next_event(&mut r.events).await {
        TelemetryEvent::State(_, meta) => assert_eq!(meta.age, AgeInfo::UnknownOffset),
        other => panic!("unexpected {other:?}"),
    }

    // LOCKED -> a computed age
    r.ts_tx
        .send(Snapshot { offset_ns: 0, rtt_ns: 1000, quality: Quality::Locked, window_len: 32, rejected: 0 })
        .unwrap();
    r.frames.send(state_frame(2, 7)).await.unwrap();
    match next_event(&mut r.events).await {
        TelemetryEvent::State(_, meta) => {
            assert!(matches!(meta.age, AgeInfo::Locked { .. }));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test(start_paused = true)]
async fn watchdog_fires_once_after_4x_nominal_silence() {
    let mut r = rig();

    r.frames.send(state_frame(1, 9)).await.unwrap();
    match next_event(&mut r.events).await {
        TelemetryEvent::State(..) => {}
        other => panic!("unexpected {other:?}"),
    }
    assert!(!r.stats.stream_stale(StreamId::State));

    // state nominal period 40 ms -> stale after 160 ms of silence
    advance(Duration::from_millis(500)).await;

    match next_event(&mut r.events).await {
        TelemetryEvent::StreamStale(s) => assert_eq!(s, StreamId::State),
        other => panic!("unexpected {other:?}"),
    }
    assert!(r.stats.stream_stale(StreamId::State));

    // stays stale silently (no event spam) — advance more, expect nothing
    advance(Duration::from_millis(500)).await;
    // then data resumes -> flag clears
    r.frames.send(state_frame(2, 9)).await.unwrap();
    loop {
        match next_event(&mut r.events).await {
            TelemetryEvent::State(d, _) if d.sequence == 2 => break,
            TelemetryEvent::StreamStale(_) => panic!("stale event spammed"),
            _ => {}
        }
    }
    advance(Duration::from_millis(150)).await; // < 4x nominal since resume... within threshold
    assert!(!r.stats.stream_stale(StreamId::State), "resumed data clears staleness");
}
