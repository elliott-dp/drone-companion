//! End-to-end round-trip: build a benign multi-stream trace, persist it as real
//! `cc-mission-log` Parquet parts (via the same `RowBuf` the live writer uses) +
//! a manifest, then `read_mission` + replay and prove the finding timeline is
//! **byte-identical** to replaying the in-memory events directly. This closes
//! the loop the in-memory unit tests can't: the Parquet reconstruction is
//! lossless *for findings*, and the whole recorded-mission path is deterministic.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::path::Path;

use cc_ingest::{AgeInfo, RxMeta, StreamId, TelemetryEvent};
use cc_mission_log::batch::{RowBuf, SegmentIdentity};
use cc_mission_log::manifest::{Manifest, RawEntry, SegmentEntry};
use cc_protocol::cc_dialect::*;
use cc_replay::{read_mission, replay_events, run_mission};
use parquet::arrow::ArrowWriter;

const BOOT: u32 = 7;
const MISSION: u32 = 1;

fn rx(cc_ns: i64) -> RxMeta {
    RxMeta { cc_receive_time_ns: cc_ns, seq_gap: 0, age: AgeInfo::Locked { age_ns: 1 } }
}

/// A healthy, armed, hovering-near-home benign trace across the six periodic
/// streams at their nominal rates, time-ordered (State,Imu,Power,Gps,Est,Act).
fn benign_trace(dur_ns: i64) -> Vec<TelemetryEvent> {
    let mut out = Vec::new();
    let step = 10_000_000;
    let mut t = 0;
    let mut pi = 0u64;
    while t < dur_ns {
        if t % 40_000_000 == 0 {
            let d = CC_TELEMETRY_STATE_DATA {
                px4_boot_id: BOOT,
                mission_id: MISSION,
                angular_velocity: [0.02, 0.01, 0.0],
                velocity_ned: [0.1, 0.0, 0.0],
                position_ned: [1.0, 1.0, -10.0],
                heading: 0.3,
                arming_state: 2,
                estimator_valid: 1,
                schema_version: 1,
                ..Default::default()
            };
            out.push(TelemetryEvent::State(d, rx(t)));
        }
        if t % 20_000_000 == 0 {
            let d = CC_TELEMETRY_IMU_DATA {
                vibration_metric: [8.0, 0.05, 0.0005],
                accel: [0.0, 0.0, -9.8],
                temperature: 45.0,
                schema_version: 1,
                ..Default::default()
            };
            out.push(TelemetryEvent::Imu(d, rx(t)));
        }
        if t % 100_000_000 == 0 {
            let cur = if pi % 2 == 0 { 10.0 } else { 14.0 };
            pi += 1;
            let d = CC_TELEMETRY_POWER_DATA {
                voltage: 15.8,
                current: cur,
                power: 15.8 * cur,
                remaining: 0.72,
                temperature: 32.0,
                cell_count: 4,
                connected: 1,
                schema_version: 1,
                ..Default::default()
            };
            out.push(TelemetryEvent::Power(d, rx(t)));
        }
        if t % 200_000_000 == 0 {
            let d = CC_TELEMETRY_GPS_DATA {
                eph: 0.8,
                epv: 1.2,
                ground_speed: 0.1,
                noise_per_ms: 80,
                jamming_indicator: 5,
                fix_type: 4,
                satellites_used: 14,
                schema_version: 1,
                ..Default::default()
            };
            out.push(TelemetryEvent::Gps(d, rx(t)));
        }
        if t % 100_000_000 == 0 {
            let d = CC_TELEMETRY_ESTIMATOR_DATA {
                velocity_test_ratio: 0.4,
                position_test_ratio: 0.35,
                height_test_ratio: 0.3,
                mag_test_ratio: 0.25,
                airspeed_test_ratio: f32::NAN,
                schema_version: 1,
                ..Default::default()
            };
            out.push(TelemetryEvent::Estimator(d, rx(t)));
        }
        if t % 60_000_000 == 0 {
            let d = CC_TELEMETRY_ACTUATOR_DATA {
                actuator_output: [0.5, 0.5, 0.5, 0.5, 0.0, 0.0, 0.0, 0.0],
                motor_count: 4,
                schema_version: 1,
                ..Default::default()
            };
            out.push(TelemetryEvent::Actuator(d, rx(t)));
        }
        t += step;
    }
    out
}

fn write_stream_part(dir: &Path, stream: StreamId, events: &[&TelemetryEvent]) {
    if events.is_empty() {
        return;
    }
    let sdir = dir.join(stream.name());
    fs::create_dir_all(&sdir).unwrap();
    let id = SegmentIdentity { vehicle_id: 1, mission_id: MISSION, cc_boot_id: 1, px4_boot_id: BOOT };
    let mut buf = RowBuf::new(stream, id);
    for ev in events {
        assert!(buf.push(ev), "row rejected for {stream:?}");
    }
    let batch = buf.finish().unwrap();
    let file = File::create(sdir.join("000000.parquet")).unwrap();
    let mut w = ArrowWriter::try_new(file, batch.schema(), None).unwrap();
    w.write(&batch).unwrap();
    w.close().unwrap();
}

fn write_mission(root: &Path, events: &[TelemetryEvent]) {
    let mission_dir = root.join("mission_000001");
    let seg_dir = mission_dir.join("segment_00");
    fs::create_dir_all(&seg_dir).unwrap();

    for stream in
        [StreamId::State, StreamId::Imu, StreamId::Power, StreamId::Gps, StreamId::Estimator, StreamId::Actuator]
    {
        let rows: Vec<&TelemetryEvent> = events
            .iter()
            .filter(|e| matches!(
                (stream, e),
                (StreamId::State, TelemetryEvent::State(..))
                    | (StreamId::Imu, TelemetryEvent::Imu(..))
                    | (StreamId::Power, TelemetryEvent::Power(..))
                    | (StreamId::Gps, TelemetryEvent::Gps(..))
                    | (StreamId::Estimator, TelemetryEvent::Estimator(..))
                    | (StreamId::Actuator, TelemetryEvent::Actuator(..))
            ))
            .collect();
        write_stream_part(&seg_dir, stream, &rows);
    }

    let mut manifest = Manifest::new(1, MISSION, "test".into(), 0);
    manifest.complete = true;
    manifest.segments.push(SegmentEntry {
        index: 0,
        dir: "segment_00".into(),
        cc_boot_id: 1,
        px4_boot_id: BOOT,
        opened_wall_unix_ns: 0,
        closed_wall_unix_ns: Some(1),
        close_reason: Some("clean".into()),
        streams: BTreeMap::new(),
        raw_mavlink: RawEntry { present: false, bytes: 0, frames: 0, shed: false },
        drop_totals: BTreeMap::new(),
    });
    let json = serde_json::to_string_pretty(&manifest).unwrap();
    fs::write(Manifest::path(&mission_dir), json).unwrap();
}

fn scratch_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("cc-replay-rt-{}-{tag}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn parquet_roundtrip_reproduces_in_memory_findings() {
    let events = benign_trace(30_000_000_000); // 30 s
    let root = scratch_dir("findings");
    write_mission(&root, &events);
    let mission_dir = root.join("mission_000001");

    // 1) the reader restores exactly the events we wrote
    let read_back = read_mission(&mission_dir).expect("read_mission");
    assert_eq!(read_back.len(), events.len(), "row count must round-trip");

    // 2) replaying the recorded mission == replaying the in-memory events
    let from_disk = run_mission(&mission_dir).expect("run_mission");
    let from_mem = replay_events(&events);
    assert!(!from_disk.rows.is_empty(), "expected a non-trivial timeline");
    assert_eq!(
        from_disk.hash(),
        from_mem.hash(),
        "Parquet round-trip must be lossless for findings"
    );

    // 3) benign mission → no WARN/CRITICAL
    assert_eq!(from_disk.findings().count(), 0, "benign mission produced findings");

    fs::remove_dir_all(&root).ok();
}

#[test]
fn recorded_mission_replays_identically_twice() {
    let events = benign_trace(20_000_000_000);
    let root = scratch_dir("determinism");
    write_mission(&root, &events);
    let mission_dir = root.join("mission_000001");

    let a = run_mission(&mission_dir).expect("run a");
    let b = run_mission(&mission_dir).expect("run b");
    assert_eq!(a.hash(), b.hash(), "same dataset → identical findings on re-run");

    fs::remove_dir_all(&root).ok();
}
