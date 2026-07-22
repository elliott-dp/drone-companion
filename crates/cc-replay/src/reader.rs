//! Mission-dataset → time-ordered `TelemetryEvent` stream.
//!
//! Reads a `cc-mission-log` mission directory (manifest → `segment_NN/` →
//! per-stream `NNNNNN.parquet` parts), reconstructs each row into its wire
//! struct + [`RxMeta`] identity envelope, and **k-way merge-sorts** every row
//! across all streams and segments by `(cc_receive_time_ns, stream_id,
//! sequence)` into one ordered stream — exactly the order the live ingest
//! produced, so the downstream Runner reproduces the live findings.
//!
//! Only the **six periodic streams** the health algorithms consume are decoded
//! (State, Imu, Power, Gps, Estimator, Actuator). `Event` and `SafetyStatus`
//! are intentionally skipped: no `HealthAlgorithm` reads them, and the Runner
//! only uses them to bump `last_seen[Event|SafetyStatus]`, which no algorithm's
//! freshness check consults — so omitting them cannot change any finding, and
//! decoding them would be dead work. (Documented replay scope, not a silent
//! gap.)

use std::path::Path;

use arrow::array::{
    Array, BooleanArray, FixedSizeListArray, Float32Array, Int32Array, Int64Array, UInt16Array,
    UInt32Array, UInt64Array, UInt8Array,
};
use arrow::record_batch::RecordBatch;
use cc_ingest::{AgeInfo, RxMeta, StreamId, TelemetryEvent};
use cc_mission_log::manifest::Manifest;
use cc_mission_log::part::is_sealed_part;
use cc_protocol::cc_dialect::*;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

/// Streams whose rows drive the health algorithms (see module doc).
const PERIODIC: [StreamId; 6] = [
    StreamId::State,
    StreamId::Imu,
    StreamId::Power,
    StreamId::Gps,
    StreamId::Estimator,
    StreamId::Actuator,
];

#[derive(Debug)]
pub enum ReplayError {
    Manifest(String),
    Io(String),
    Parquet(String),
    Schema(String),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayError::Manifest(s) => write!(f, "manifest: {s}"),
            ReplayError::Io(s) => write!(f, "io: {s}"),
            ReplayError::Parquet(s) => write!(f, "parquet: {s}"),
            ReplayError::Schema(s) => write!(f, "schema: {s}"),
        }
    }
}
impl std::error::Error for ReplayError {}

/// Merge key: the live receive order — receive time, then stream, then per-
/// stream sequence, so ties are resolved identically on every replay.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Key(i64, u8, u32);

/// Read a mission directory into one time-ordered `TelemetryEvent` stream.
pub fn read_mission(mission_dir: &Path) -> Result<Vec<TelemetryEvent>, ReplayError> {
    let manifest =
        Manifest::read(mission_dir).map_err(|e| ReplayError::Manifest(format!("{e:?}")))?;

    let mut rows: Vec<(Key, TelemetryEvent)> = Vec::new();
    for seg in &manifest.segments {
        let seg_dir = mission_dir.join(&seg.dir);
        for stream in PERIODIC {
            let sdir = seg_dir.join(stream.name());
            if !sdir.is_dir() {
                continue;
            }
            let mut parts: Vec<_> = std::fs::read_dir(&sdir)
                .map_err(|e| ReplayError::Io(format!("{}: {e}", sdir.display())))?
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| is_sealed_part(n))
                .collect();
            parts.sort(); // NNNNNN ordering (cosmetic; we sort by key at the end)
            for name in parts {
                decode_part(&sdir.join(name), stream, &mut rows)?;
            }
        }
    }

    // stable k-way merge by (cc_ns, stream, sequence)
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(rows.into_iter().map(|(_, ev)| ev).collect())
}

fn decode_part(
    path: &Path,
    stream: StreamId,
    out: &mut Vec<(Key, TelemetryEvent)>,
) -> Result<(), ReplayError> {
    let file =
        std::fs::File::open(path).map_err(|e| ReplayError::Io(format!("{}: {e}", path.display())))?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| ReplayError::Parquet(format!("{}: {e}", path.display())))?
        .build()
        .map_err(|e| ReplayError::Parquet(format!("{}: {e}", path.display())))?;
    for batch in reader {
        let batch = batch.map_err(|e| ReplayError::Parquet(format!("{}: {e}", path.display())))?;
        decode_batch(stream, &batch, out)?;
    }
    Ok(())
}

// ---- typed column accessors -------------------------------------------------

macro_rules! col {
    ($b:expr, $name:literal, $ty:ty) => {{
        let c = $b
            .column_by_name($name)
            .ok_or_else(|| ReplayError::Schema(format!("missing column {}", $name)))?;
        c.as_any()
            .downcast_ref::<$ty>()
            .ok_or_else(|| ReplayError::Schema(format!("column {} wrong type", $name)))?
    }};
}

fn fsl_row<const N: usize>(a: &FixedSizeListArray, row: usize) -> Result<[f32; N], ReplayError> {
    let vals = a.value(row);
    let f = vals
        .as_any()
        .downcast_ref::<Float32Array>()
        .ok_or_else(|| ReplayError::Schema("fixed-size-list item not f32".into()))?;
    Ok(std::array::from_fn(|i| f.value(i)))
}

/// The identity envelope fields common to every stream row.
struct Env {
    mission_id: u32,
    px4_boot_id: u32,
    sequence: u32,
    fc_timestamp_us: u64,
    rx: RxMeta,
}

fn decode_batch(
    stream: StreamId,
    b: &RecordBatch,
    out: &mut Vec<(Key, TelemetryEvent)>,
) -> Result<(), ReplayError> {
    let mission_id = col!(b, "mission_id", UInt32Array);
    let px4_boot_id = col!(b, "px4_boot_id", UInt32Array);
    let sequence = col!(b, "sequence", UInt32Array); // nullable
    let fc_ts = col!(b, "fc_timestamp_us", UInt64Array);
    let cc_ns = col!(b, "cc_receive_time_ns", Int64Array);
    let seq_gap = col!(b, "seq_gap", UInt32Array);
    let age_ns = col!(b, "age_ns", Int64Array); // nullable
    let age_locked = col!(b, "age_locked", BooleanArray);
    let schema_version = col!(b, "schema_version", UInt8Array);

    let n = b.num_rows();
    let env_of = |i: usize| -> Env {
        let age = if age_locked.value(i) && !age_ns.is_null(i) {
            AgeInfo::Locked { age_ns: age_ns.value(i) }
        } else {
            AgeInfo::UnknownOffset
        };
        Env {
            mission_id: mission_id.value(i),
            px4_boot_id: px4_boot_id.value(i),
            sequence: if sequence.is_null(i) { 0 } else { sequence.value(i) },
            fc_timestamp_us: fc_ts.value(i),
            rx: RxMeta { cc_receive_time_ns: cc_ns.value(i), seq_gap: seq_gap.value(i), age },
        }
    };
    let sv = |i: usize| schema_version.value(i);

    match stream {
        StreamId::State => {
            let failsafe = col!(b, "failsafe_flags", UInt32Array);
            let q = col!(b, "q", FixedSizeListArray);
            let av = col!(b, "angular_velocity", FixedSizeListArray);
            let pos = col!(b, "position_ned", FixedSizeListArray);
            let vel = col!(b, "velocity_ned", FixedSizeListArray);
            let heading = col!(b, "heading", Float32Array);
            let nav = col!(b, "nav_state", UInt8Array);
            let arming = col!(b, "arming_state", UInt8Array);
            let vt = col!(b, "vehicle_type", UInt8Array);
            let ev = col!(b, "estimator_valid", UInt8Array);
            let cmf = col!(b, "control_mode_flags", UInt8Array);
            for i in 0..n {
                let e = env_of(i);
                let d = CC_TELEMETRY_STATE_DATA {
                    fc_timestamp_us: e.fc_timestamp_us,
                    sequence: e.sequence,
                    px4_boot_id: e.px4_boot_id,
                    mission_id: e.mission_id,
                    failsafe_flags: failsafe.value(i),
                    q: fsl_row(q, i)?,
                    angular_velocity: fsl_row(av, i)?,
                    position_ned: fsl_row(pos, i)?,
                    velocity_ned: fsl_row(vel, i)?,
                    heading: heading.value(i),
                    nav_state: nav.value(i),
                    arming_state: arming.value(i),
                    vehicle_type: vt.value(i),
                    estimator_valid: ev.value(i),
                    control_mode_flags: cmf.value(i),
                    schema_version: sv(i),
                };
                push(out, stream, &e, TelemetryEvent::State(d, e.rx));
            }
        }
        StreamId::Imu => {
            let clip = col!(b, "clipping_count", UInt32Array);
            let accel = col!(b, "accel", FixedSizeListArray);
            let gyro = col!(b, "gyro", FixedSizeListArray);
            let da = col!(b, "delta_angle", FixedSizeListArray);
            let dv = col!(b, "delta_velocity", FixedSizeListArray);
            let vib = col!(b, "vibration_metric", FixedSizeListArray);
            let temp = col!(b, "temperature", Float32Array);
            for i in 0..n {
                let e = env_of(i);
                let d = CC_TELEMETRY_IMU_DATA {
                    fc_timestamp_us: e.fc_timestamp_us,
                    sequence: e.sequence,
                    clipping_count: clip.value(i),
                    accel: fsl_row(accel, i)?,
                    gyro: fsl_row(gyro, i)?,
                    delta_angle: fsl_row(da, i)?,
                    delta_velocity: fsl_row(dv, i)?,
                    vibration_metric: fsl_row(vib, i)?,
                    temperature: temp.value(i),
                    schema_version: sv(i),
                };
                push(out, stream, &e, TelemetryEvent::Imu(d, e.rx));
            }
        }
        StreamId::Power => {
            let voltage = col!(b, "voltage", Float32Array);
            let current = col!(b, "current", Float32Array);
            let power = col!(b, "power", Float32Array);
            let consumed = col!(b, "consumed_mah", Float32Array);
            let remaining = col!(b, "remaining", Float32Array);
            let temp = col!(b, "temperature", Float32Array);
            let cells = col!(b, "cell_count", UInt8Array);
            let warning = col!(b, "warning", UInt8Array);
            let connected = col!(b, "connected", UInt8Array);
            for i in 0..n {
                let e = env_of(i);
                let d = CC_TELEMETRY_POWER_DATA {
                    fc_timestamp_us: e.fc_timestamp_us,
                    sequence: e.sequence,
                    voltage: voltage.value(i),
                    current: current.value(i),
                    power: power.value(i),
                    consumed_mah: consumed.value(i),
                    remaining: remaining.value(i),
                    temperature: temp.value(i),
                    cell_count: cells.value(i),
                    warning: warning.value(i),
                    connected: connected.value(i),
                    schema_version: sv(i),
                };
                push(out, stream, &e, TelemetryEvent::Power(d, e.rx));
            }
        }
        StreamId::Gps => {
            let lat = col!(b, "lat", Int32Array);
            let lon = col!(b, "lon", Int32Array);
            let alt = col!(b, "alt", Int32Array);
            let eph = col!(b, "eph", Float32Array);
            let epv = col!(b, "epv", Float32Array);
            let gs = col!(b, "ground_speed", Float32Array);
            let heading = col!(b, "heading", Float32Array);
            let noise = col!(b, "noise_per_ms", UInt16Array);
            let jam = col!(b, "jamming_indicator", UInt16Array);
            let fix = col!(b, "fix_type", UInt8Array);
            let sats = col!(b, "satellites_used", UInt8Array);
            for i in 0..n {
                let e = env_of(i);
                let d = CC_TELEMETRY_GPS_DATA {
                    fc_timestamp_us: e.fc_timestamp_us,
                    sequence: e.sequence,
                    lat: lat.value(i),
                    lon: lon.value(i),
                    alt: alt.value(i),
                    eph: eph.value(i),
                    epv: epv.value(i),
                    ground_speed: gs.value(i),
                    heading: heading.value(i),
                    noise_per_ms: noise.value(i),
                    jamming_indicator: jam.value(i),
                    fix_type: fix.value(i),
                    satellites_used: sats.value(i),
                    schema_version: sv(i),
                };
                push(out, stream, &e, TelemetryEvent::Gps(d, e.rx));
            }
        }
        StreamId::Estimator => {
            let status = col!(b, "status_flags", UInt32Array);
            let vel = col!(b, "velocity_test_ratio", Float32Array);
            let pos = col!(b, "position_test_ratio", Float32Array);
            let hgt = col!(b, "height_test_ratio", Float32Array);
            let mag = col!(b, "mag_test_ratio", Float32Array);
            let air = col!(b, "airspeed_test_ratio", Float32Array);
            let innov = col!(b, "innovation_check_flags", UInt16Array);
            let sol = col!(b, "solution_status_flags", UInt16Array);
            for i in 0..n {
                let e = env_of(i);
                let d = CC_TELEMETRY_ESTIMATOR_DATA {
                    fc_timestamp_us: e.fc_timestamp_us,
                    sequence: e.sequence,
                    status_flags: status.value(i),
                    velocity_test_ratio: vel.value(i),
                    position_test_ratio: pos.value(i),
                    height_test_ratio: hgt.value(i),
                    mag_test_ratio: mag.value(i),
                    airspeed_test_ratio: air.value(i),
                    innovation_check_flags: innov.value(i),
                    solution_status_flags: sol.value(i),
                    schema_version: sv(i),
                };
                push(out, stream, &e, TelemetryEvent::Estimator(d, e.rx));
            }
        }
        StreamId::Actuator => {
            let outs = col!(b, "actuator_output", FixedSizeListArray);
            let mc = col!(b, "motor_count", UInt8Array);
            for i in 0..n {
                let e = env_of(i);
                let d = CC_TELEMETRY_ACTUATOR_DATA {
                    fc_timestamp_us: e.fc_timestamp_us,
                    sequence: e.sequence,
                    actuator_output: fsl_row(outs, i)?,
                    motor_count: mc.value(i),
                    schema_version: sv(i),
                };
                push(out, stream, &e, TelemetryEvent::Actuator(d, e.rx));
            }
        }
        StreamId::Event | StreamId::SafetyStatus => {}
    }
    Ok(())
}

fn push(out: &mut Vec<(Key, TelemetryEvent)>, stream: StreamId, e: &Env, ev: TelemetryEvent) {
    out.push((Key(e.rx.cc_receive_time_ns, stream as u8, e.sequence), ev));
}
