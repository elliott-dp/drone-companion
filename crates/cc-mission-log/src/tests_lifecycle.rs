//! Deterministic end-to-end lifecycle tests: a whole mission written through
//! [`Mission`], then read back by [`inspect_mission`] — with the "crash"
//! modelled as **dropping the mission without finalising** (byte-identical to
//! a real `kill -9` between seals, but host-runnable in milliseconds and 100%
//! reproducible).

#![cfg(test)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use cc_config::Config;
use cc_ingest::{AgeInfo, RxMeta, StreamId, TelemetryEvent};
use cc_protocol::cc_dialect::{
    CC_TELEMETRY_ACTUATOR_DATA, CC_TELEMETRY_IMU_DATA, CC_TELEMETRY_POWER_DATA,
    CC_TELEMETRY_STATE_DATA,
};

use crate::env::{Clock, FakeClock, FakeSpace, NoopSyncer, SpaceProbe, Syncer};
use crate::inspect::{inspect_mission, Verdict};
use crate::part::part_name;
use crate::shed::ShedStage;
use crate::{LogHealth, Mission};

fn tmp_root(tag: &str) -> std::path::PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    let p = std::env::temp_dir().join(format!("ccml-life-{}-{tag}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// Config with compact, readable thresholds and a small flush cap.
fn cfg(root: std::path::PathBuf) -> Config {
    let mut c = Config::default();
    c.general.mission_root = root;
    c.general.vehicle_id = 1;
    c.mission_log.flush_rows = 4;
    c.mission_log.flush_secs = 10;
    c.mission_log.seg_cap_bytes = 1 << 40; // effectively no size rotation
    c.mission_log.seg_cap_secs = 1 << 30; // no time rotation
    c.disk.floor_bytes = 100;
    c.disk.raw_shed_low_bytes = 40;
    c.disk.raw_resume_bytes = 50;
    c.disk.bf_shed_low_bytes = 30;
    c.disk.bf_resume_bytes = 35;
    c.disk.crit_low_bytes = 20;
    c.disk.crit_resume_bytes = 25;
    c
}

fn meta(seq_cc: i64) -> RxMeta {
    RxMeta { cc_receive_time_ns: seq_cc, seq_gap: 0, age: AgeInfo::Locked { age_ns: 1 } }
}

fn power(seq: u32) -> TelemetryEvent {
    TelemetryEvent::Power(
        CC_TELEMETRY_POWER_DATA {
            fc_timestamp_us: 1000 + u64::from(seq),
            sequence: seq,
            voltage: 16.8,
            current: 1.0,
            power: 16.8,
            consumed_mah: 2.0,
            remaining: 0.9,
            temperature: 25.0,
            cell_count: 6,
            warning: 0,
            connected: 1,
            schema_version: 1,
        },
        meta(100 + i64::from(seq)),
    )
}

fn state(seq: u32) -> TelemetryEvent {
    TelemetryEvent::State(
        CC_TELEMETRY_STATE_DATA {
            fc_timestamp_us: 2000 + u64::from(seq),
            sequence: seq,
            px4_boot_id: 3,
            mission_id: 0,
            failsafe_flags: 0,
            q: [1.0, 0.0, 0.0, 0.0],
            angular_velocity: [0.0; 3],
            position_ned: [0.0; 3],
            velocity_ned: [0.0; 3],
            heading: 0.0,
            nav_state: 3,
            arming_state: 2,
            vehicle_type: 2,
            estimator_valid: 1,
            control_mode_flags: 0,
            schema_version: 1,
        },
        meta(200 + i64::from(seq)),
    )
}

fn imu(seq: u32) -> TelemetryEvent {
    TelemetryEvent::Imu(
        CC_TELEMETRY_IMU_DATA {
            fc_timestamp_us: 3000 + u64::from(seq),
            sequence: seq,
            clipping_count: 0,
            accel: [0.0, 0.0, 9.8],
            gyro: [0.0; 3],
            delta_angle: [0.0; 3],
            delta_velocity: [0.0; 3],
            vibration_metric: [0.0; 3],
            temperature: 30.0,
            schema_version: 1,
        },
        meta(300 + i64::from(seq)),
    )
}

fn actuator(seq: u32) -> TelemetryEvent {
    TelemetryEvent::Actuator(
        CC_TELEMETRY_ACTUATOR_DATA {
            fc_timestamp_us: 4000 + u64::from(seq),
            sequence: seq,
            actuator_output: [0.5; 8],
            motor_count: 4,
            schema_version: 1,
        },
        meta(400 + i64::from(seq)),
    )
}

fn open(cfg: Config, space: Arc<dyn SpaceProbe>, health: Arc<LogHealth>) -> Mission {
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(0, 1_721_000_000_000_000_000));
    let syncer: Arc<dyn Syncer> = Arc::new(NoopSyncer);
    Mission::open(cfg, clock, space, syncer, health, "phase5-test".into(), 3, |_| {}).unwrap()
}

#[test]
fn clean_mission_is_inspect_clean() {
    let root = tmp_root("clean");
    let health = Arc::new(LogHealth::default());
    let space: Arc<dyn SpaceProbe> = Arc::new(FakeSpace::fixed(1000)); // well above every threshold
    let mut m = open(cfg(root.clone()), space, health);
    let dir = m.mission_dir().to_path_buf();

    for i in 0..10 {
        m.on_event(&power(i));
        m.on_event(&state(i));
    }
    m.finalize().unwrap();

    let report = inspect_mission(&dir);
    assert_eq!(report.verdict, Verdict::Clean, "verdict: {:?}", report.verdict);
    assert!(report.complete);
    assert_eq!(report.total_rows(), 20, "10 power + 10 state rows, all sealed at finalize");
    assert_eq!(report.total_drops(), 0);
    assert_eq!(report.exit_code(), 0);
}

#[test]
fn crash_drop_leaves_sealed_parts_and_dirty_verdict() {
    let root = tmp_root("crash");
    let health = Arc::new(LogHealth::default());
    let space: Arc<dyn SpaceProbe> = Arc::new(FakeSpace::fixed(1000));
    let mut m = open(cfg(root.clone()), space, health);
    let dir = m.mission_dir().to_path_buf();

    // 10 power rows at flush_rows=4 → 2 sealed parts (8 rows) + 2 buffered.
    for i in 0..10 {
        m.on_event(&power(i));
    }
    // "kill -9": drop the mission WITHOUT finalizing (buffered rows are lost;
    // sealed parts and the complete=false manifest remain on disk).
    drop(m);

    // The two sealed parts are readable by a stock reader (proved via inspect).
    let report = inspect_mission(&dir);
    assert!(matches!(report.verdict, Verdict::Dirty(_)), "verdict: {:?}", report.verdict);
    assert!(!report.complete, "mission left incomplete by the crash");
    assert_eq!(report.exit_code(), 1);
    // sealed part 000000 exists and is readable
    let seg0_power = dir.join("segment_00").join("power");
    assert!(seg0_power.join(part_name(0)).exists(), "first sealed part present");
    // the 8 sealed rows survived; the 2 buffered rows are the bounded loss
    let power_rows: u64 = report.segments[0]
        .streams
        .iter()
        .find(|s| s.name == "power")
        .map(|s| s.rows)
        .unwrap_or(0);
    assert_eq!(power_rows, 8, "exactly the sealed rows survive kill -9");
}

#[test]
fn restart_resumes_same_mission_id_new_segment() {
    let root = tmp_root("resume");
    // first process: write + crash
    let h1 = Arc::new(LogHealth::default());
    let s1: Arc<dyn SpaceProbe> = Arc::new(FakeSpace::fixed(1000));
    let mut m1 = open(cfg(root.clone()), s1, h1);
    let mission_id_1 = m1.mission_id();
    let dir1 = m1.mission_dir().to_path_buf();
    for i in 0..6 {
        m1.on_event(&power(i));
    }
    drop(m1); // crash

    // second process: resume — SAME mission_id, NEW segment (spec §7)
    let h2 = Arc::new(LogHealth::default());
    let s2: Arc<dyn SpaceProbe> = Arc::new(FakeSpace::fixed(1000));
    let mut m2 = open(cfg(root.clone()), s2, h2);
    assert_eq!(m2.mission_id(), mission_id_1, "restart continues the same mission");
    assert_eq!(m2.mission_dir(), dir1, "same mission directory");
    for i in 6..12 {
        m2.on_event(&power(i));
    }
    m2.finalize().unwrap();

    // manifest now links segment_00 (crashed) + segment_01 (resumed, clean)
    let report = inspect_mission(&dir1);
    assert!(report.segments.len() >= 2, "two segments linked in one manifest");
    assert_eq!(report.segments[1].dir, "segment_01");
    assert!(report.complete, "second run finalized the mission");
}

#[test]
fn disk_full_sheds_in_order_and_never_state() {
    let root = tmp_root("diskfull");
    let health = Arc::new(LogHealth::default());
    // free readings: [open floor-gate, tick#1, tick#2, …]. Open consumes the
    // first (1000, above the floor); tick#1 stays NORMAL (1000); tick#2 craters
    // to SHED_CRIT (15) and sticks.
    let space: Arc<dyn SpaceProbe> = Arc::new(FakeSpace::new(vec![1000, 1000, 15, 15]));
    let mut m = open(cfg(root.clone()), space, health.clone());
    let dir = m.mission_dir().to_path_buf();

    // tick #1 reads 1000 (NORMAL): everything written
    m.tick().unwrap();
    m.on_event(&state(0));
    m.on_event(&imu(0));
    m.on_event(&actuator(0));

    // tick #2 reads 15 (< crit_low 20 → SHED_CRIT): imu + actuator dropped,
    // state always written.
    m.tick().unwrap();
    assert_eq!(health.snapshot().shed_stage, ShedStage::ShedCrit.as_u8());
    for i in 1..5 {
        m.on_event(&state(i)); // never shed
        m.on_event(&imu(i)); // shed
        m.on_event(&actuator(i)); // shed
    }
    m.finalize().unwrap();

    let snap = health.snapshot();
    assert!(snap.warn, "shedding raises the WARN flag");
    assert_eq!(snap.dropped[StreamId::Imu as usize], 4, "4 imu rows shed at CRIT");
    assert_eq!(snap.dropped[StreamId::Actuator as usize], 4, "4 actuator rows shed at CRIT");
    assert_eq!(snap.dropped[StreamId::State as usize], 0, "state NEVER shed");

    // inspect: state rows landed throughout; drop ledger is non-zero (Dirty).
    let report = inspect_mission(&dir);
    let state_rows = report.segments[0].streams.iter()
        .find(|s| s.name == "state").map(|s| s.rows).unwrap_or(0);
    assert_eq!(state_rows, 5, "all 5 state rows written despite disk pressure");
    assert!(report.total_drops() > 0, "drops recorded in events.parquet");
    assert!(matches!(report.verdict, Verdict::Dirty(_)));
}

#[test]
fn stray_inprogress_file_reads_dirty() {
    let root = tmp_root("inprogress");
    let health = Arc::new(LogHealth::default());
    let space: Arc<dyn SpaceProbe> = Arc::new(FakeSpace::fixed(1000));
    let mut m = open(cfg(root.clone()), space, health);
    let dir = m.mission_dir().to_path_buf();
    for i in 0..8 {
        m.on_event(&power(i));
    }
    m.finalize().unwrap();
    // simulate a crash artifact: a leftover in-progress part
    let stray = dir.join("segment_00").join("power").join("000009.parquet.inprogress");
    std::fs::write(&stray, b"partial").unwrap();

    let report = inspect_mission(&dir);
    assert!(matches!(report.verdict, Verdict::Dirty(_)), "in-progress artifact → Dirty");
}

#[test]
fn boot_change_rotates_into_new_segment() {
    let root = tmp_root("boot");
    let health = Arc::new(LogHealth::default());
    let space: Arc<dyn SpaceProbe> = Arc::new(FakeSpace::fixed(1000));
    let mut m = open(cfg(root.clone()), space, health);
    let dir = m.mission_dir().to_path_buf();
    for i in 0..4 {
        m.on_event(&power(i));
    }
    m.on_boot_change(9).unwrap(); // PX4 rebooted → new segment
    for i in 4..8 {
        m.on_event(&power(i));
    }
    m.finalize().unwrap();

    let report = inspect_mission(&dir);
    assert_eq!(report.segments.len(), 2, "boot change split the mission into two segments");
    assert_eq!(report.segments[0].px4_boot_id, 3);
    assert_eq!(report.segments[1].px4_boot_id, 9);
    assert_eq!(report.segments[1].dir, "segment_01");
    assert_eq!(report.verdict, Verdict::Clean);
}
