//! Row accumulation → Arrow `RecordBatch`.
//!
//! Rows land in plain typed `Vec`s (not live Arrow builders) so the hot path
//! is a trivial push and memory is bounded by `flush_rows`; the Arrow arrays
//! are materialised once, at [`RowBuf::finish`]. Column order matches
//! [`crate::schema`] exactly, and `RecordBatch::try_new` re-checks that at
//! build time.

use std::sync::Arc;

use arrow::array::{
    ArrayRef, BooleanArray, Float32Array, Int32Array, Int64Array, UInt16Array, UInt32Array,
    UInt64Array, UInt8Array, FixedSizeListArray,
};
use arrow::datatypes::{DataType, Field};
use arrow::record_batch::RecordBatch;
use cc_ingest::{AgeInfo, RxMeta, StreamId, TelemetryEvent};

use crate::schema::stream_schema;

/// The four segment-constant identity columns (§3.4). Every row in a segment
/// carries the same values; Parquet RLE/dictionary collapses them so each
/// lone part file stays independently join-able on the dedup key.
#[derive(Debug, Clone, Copy)]
pub struct SegmentIdentity {
    pub vehicle_id: u32,
    pub mission_id: u32,
    pub cc_boot_id: u32,
    pub px4_boot_id: u32,
}

/// Shared identity-envelope accumulators.
#[derive(Default)]
struct EnvCols {
    sequence: Vec<Option<u32>>,
    fc_timestamp_us: Vec<u64>,
    cc_receive_time_ns: Vec<i64>,
    seq_gap: Vec<u32>,
    age_ns: Vec<Option<i64>>,
    age_locked: Vec<bool>,
    schema_version: Vec<u8>,
}

impl EnvCols {
    fn push(&mut self, seq: Option<u32>, fc_us: u64, meta: &RxMeta, schema_version: u8) {
        self.sequence.push(seq);
        self.fc_timestamp_us.push(fc_us);
        self.cc_receive_time_ns.push(meta.cc_receive_time_ns);
        self.seq_gap.push(meta.seq_gap);
        match meta.age {
            AgeInfo::Locked { age_ns } => {
                self.age_ns.push(Some(age_ns));
                self.age_locked.push(true);
            }
            AgeInfo::UnknownOffset => {
                self.age_ns.push(None);
                self.age_locked.push(false);
            }
        }
        self.schema_version.push(schema_version);
    }
}

/// Accumulates rows for exactly one stream in one part.
pub struct RowBuf {
    stream: StreamId,
    id: SegmentIdentity,
    env: EnvCols,
    payload: Payload,
}

impl RowBuf {
    pub fn new(stream: StreamId, id: SegmentIdentity) -> Self {
        Self { stream, id, env: EnvCols::default(), payload: Payload::new(stream) }
    }

    pub fn stream(&self) -> StreamId {
        self.stream
    }

    pub fn len(&self) -> usize {
        self.env.fc_timestamp_us.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Append one telemetry event. Returns `false` (and pushes nothing) if the
    /// event does not belong to this buffer's stream — the caller routes by
    /// stream, so this is a guard, not a hot path.
    pub fn push(&mut self, ev: &TelemetryEvent) -> bool {
        macro_rules! env {
            ($seq:expr, $m:expr, $d:expr) => {
                self.env.push($seq, $d.fc_timestamp_us, $m, $d.schema_version)
            };
        }
        match (self.stream, ev) {
            (StreamId::State, TelemetryEvent::State(d, m)) => {
                env!(Some(d.sequence), m, d);
                self.payload.push_state(d);
            }
            (StreamId::Imu, TelemetryEvent::Imu(d, m)) => {
                env!(Some(d.sequence), m, d);
                self.payload.push_imu(d);
            }
            (StreamId::Power, TelemetryEvent::Power(d, m)) => {
                env!(Some(d.sequence), m, d);
                self.payload.push_power(d);
            }
            (StreamId::Gps, TelemetryEvent::Gps(d, m)) => {
                env!(Some(d.sequence), m, d);
                self.payload.push_gps(d);
            }
            (StreamId::Estimator, TelemetryEvent::Estimator(d, m)) => {
                env!(Some(d.sequence), m, d);
                self.payload.push_estimator(d);
            }
            (StreamId::Actuator, TelemetryEvent::Actuator(d, m)) => {
                env!(Some(d.sequence), m, d);
                self.payload.push_actuator(d);
            }
            (StreamId::Event, TelemetryEvent::Event(d, m)) => {
                env!(Some(d.sequence), m, d);
                self.payload.push_event(d);
            }
            (StreamId::SafetyStatus, TelemetryEvent::SafetyStatus(d, m)) => {
                // SafetyStatus carries no per-message sequence (envelope null).
                env!(None, m, d);
                self.payload.push_safety(d);
            }
            _ => return false,
        }
        true
    }

    /// Materialise the accumulated rows into one `RecordBatch`.
    pub fn finish(self) -> Result<RecordBatch, arrow::error::ArrowError> {
        let n = self.len();
        let id = self.id;

        // envelope (segment-constant identity first), in schema column order
        let mut cols: Vec<ArrayRef> = vec![
            Arc::new(UInt32Array::from(vec![id.vehicle_id; n])),
            Arc::new(UInt32Array::from(vec![id.mission_id; n])),
            Arc::new(UInt32Array::from(vec![id.px4_boot_id; n])),
            Arc::new(UInt32Array::from(vec![id.cc_boot_id; n])),
            Arc::new(UInt8Array::from(vec![self.stream as u8; n])),
            Arc::new(UInt32Array::from(self.env.sequence)),
            Arc::new(UInt64Array::from(self.env.fc_timestamp_us)),
            Arc::new(Int64Array::from(self.env.cc_receive_time_ns)),
            Arc::new(UInt32Array::from(self.env.seq_gap)),
            Arc::new(Int64Array::from(self.env.age_ns)),
            Arc::new(BooleanArray::from(self.env.age_locked)),
            Arc::new(UInt8Array::from(self.env.schema_version)),
        ];

        self.payload.extend_columns(&mut cols);

        RecordBatch::try_new(stream_schema(self.stream), cols)
    }
}

/// Build a `FixedSizeList<Float32, N>` column from row-major fixed arrays.
fn fsl_col<const N: usize>(rows: &[[f32; N]]) -> ArrayRef {
    let flat: Vec<f32> = rows.iter().flat_map(|a| a.iter().copied()).collect();
    let values = Arc::new(Float32Array::from(flat)) as ArrayRef;
    let field = Arc::new(Field::new("item", DataType::Float32, false));
    Arc::new(FixedSizeListArray::new(field, N as i32, values, None))
}

/// Per-stream payload column accumulators.
enum Payload {
    State {
        failsafe_flags: Vec<u32>,
        q: Vec<[f32; 4]>,
        angular_velocity: Vec<[f32; 3]>,
        position_ned: Vec<[f32; 3]>,
        velocity_ned: Vec<[f32; 3]>,
        heading: Vec<f32>,
        nav_state: Vec<u8>,
        arming_state: Vec<u8>,
        vehicle_type: Vec<u8>,
        estimator_valid: Vec<u8>,
        control_mode_flags: Vec<u8>,
    },
    Imu {
        clipping_count: Vec<u32>,
        accel: Vec<[f32; 3]>,
        gyro: Vec<[f32; 3]>,
        delta_angle: Vec<[f32; 3]>,
        delta_velocity: Vec<[f32; 3]>,
        vibration_metric: Vec<[f32; 3]>,
        temperature: Vec<f32>,
    },
    Power {
        voltage: Vec<f32>,
        current: Vec<f32>,
        power: Vec<f32>,
        consumed_mah: Vec<f32>,
        remaining: Vec<f32>,
        temperature: Vec<f32>,
        cell_count: Vec<u8>,
        warning: Vec<u8>,
        connected: Vec<u8>,
    },
    Gps {
        lat: Vec<i32>,
        lon: Vec<i32>,
        alt: Vec<i32>,
        eph: Vec<f32>,
        epv: Vec<f32>,
        ground_speed: Vec<f32>,
        heading: Vec<f32>,
        noise_per_ms: Vec<u16>,
        jamming_indicator: Vec<u16>,
        fix_type: Vec<u8>,
        satellites_used: Vec<u8>,
    },
    Estimator {
        status_flags: Vec<u32>,
        velocity_test_ratio: Vec<f32>,
        position_test_ratio: Vec<f32>,
        height_test_ratio: Vec<f32>,
        mag_test_ratio: Vec<f32>,
        airspeed_test_ratio: Vec<f32>,
        innovation_check_flags: Vec<u16>,
        solution_status_flags: Vec<u16>,
    },
    Actuator {
        actuator_output: Vec<[f32; 8]>,
        motor_count: Vec<u8>,
    },
    Event {
        event_id: Vec<u32>,
        argument0: Vec<u32>,
        argument1: Vec<u32>,
        severity: Vec<u8>,
        subsystem: Vec<u8>,
    },
    Safety {
        last_report_sequence: Vec<u32>,
        active_health_flags: Vec<u32>,
        report_age_ms: Vec<u32>,
        missed_reports: Vec<u32>,
        companion_state: Vec<u8>,
        action_taken: Vec<u8>,
        reject_reason: Vec<u8>,
    },
}

impl Payload {
    fn new(stream: StreamId) -> Self {
        match stream {
            StreamId::State => Payload::State {
                failsafe_flags: vec![], q: vec![], angular_velocity: vec![], position_ned: vec![],
                velocity_ned: vec![], heading: vec![], nav_state: vec![], arming_state: vec![],
                vehicle_type: vec![], estimator_valid: vec![], control_mode_flags: vec![],
            },
            StreamId::Imu => Payload::Imu {
                clipping_count: vec![], accel: vec![], gyro: vec![], delta_angle: vec![],
                delta_velocity: vec![], vibration_metric: vec![], temperature: vec![],
            },
            StreamId::Power => Payload::Power {
                voltage: vec![], current: vec![], power: vec![], consumed_mah: vec![],
                remaining: vec![], temperature: vec![], cell_count: vec![], warning: vec![],
                connected: vec![],
            },
            StreamId::Gps => Payload::Gps {
                lat: vec![], lon: vec![], alt: vec![], eph: vec![], epv: vec![],
                ground_speed: vec![], heading: vec![], noise_per_ms: vec![],
                jamming_indicator: vec![], fix_type: vec![], satellites_used: vec![],
            },
            StreamId::Estimator => Payload::Estimator {
                status_flags: vec![], velocity_test_ratio: vec![], position_test_ratio: vec![],
                height_test_ratio: vec![], mag_test_ratio: vec![], airspeed_test_ratio: vec![],
                innovation_check_flags: vec![], solution_status_flags: vec![],
            },
            StreamId::Actuator => Payload::Actuator { actuator_output: vec![], motor_count: vec![] },
            StreamId::Event => Payload::Event {
                event_id: vec![], argument0: vec![], argument1: vec![], severity: vec![],
                subsystem: vec![],
            },
            StreamId::SafetyStatus => Payload::Safety {
                last_report_sequence: vec![], active_health_flags: vec![], report_age_ms: vec![],
                missed_reports: vec![], companion_state: vec![], action_taken: vec![],
                reject_reason: vec![],
            },
        }
    }

    fn push_state(&mut self, d: &cc_protocol::cc_dialect::CC_TELEMETRY_STATE_DATA) {
        if let Payload::State { failsafe_flags, q, angular_velocity, position_ned, velocity_ned,
            heading, nav_state, arming_state, vehicle_type, estimator_valid, control_mode_flags } = self
        {
            failsafe_flags.push(d.failsafe_flags);
            q.push(d.q);
            angular_velocity.push(d.angular_velocity);
            position_ned.push(d.position_ned);
            velocity_ned.push(d.velocity_ned);
            heading.push(d.heading);
            nav_state.push(d.nav_state);
            arming_state.push(d.arming_state);
            vehicle_type.push(d.vehicle_type);
            estimator_valid.push(d.estimator_valid);
            control_mode_flags.push(d.control_mode_flags);
        }
    }

    fn push_imu(&mut self, d: &cc_protocol::cc_dialect::CC_TELEMETRY_IMU_DATA) {
        if let Payload::Imu { clipping_count, accel, gyro, delta_angle, delta_velocity,
            vibration_metric, temperature } = self
        {
            clipping_count.push(d.clipping_count);
            accel.push(d.accel);
            gyro.push(d.gyro);
            delta_angle.push(d.delta_angle);
            delta_velocity.push(d.delta_velocity);
            vibration_metric.push(d.vibration_metric);
            temperature.push(d.temperature);
        }
    }

    fn push_power(&mut self, d: &cc_protocol::cc_dialect::CC_TELEMETRY_POWER_DATA) {
        if let Payload::Power { voltage, current, power, consumed_mah, remaining, temperature,
            cell_count, warning, connected } = self
        {
            voltage.push(d.voltage);
            current.push(d.current);
            power.push(d.power);
            consumed_mah.push(d.consumed_mah);
            remaining.push(d.remaining);
            temperature.push(d.temperature);
            cell_count.push(d.cell_count);
            warning.push(d.warning);
            connected.push(d.connected);
        }
    }

    fn push_gps(&mut self, d: &cc_protocol::cc_dialect::CC_TELEMETRY_GPS_DATA) {
        if let Payload::Gps { lat, lon, alt, eph, epv, ground_speed, heading, noise_per_ms,
            jamming_indicator, fix_type, satellites_used } = self
        {
            lat.push(d.lat);
            lon.push(d.lon);
            alt.push(d.alt);
            eph.push(d.eph);
            epv.push(d.epv);
            ground_speed.push(d.ground_speed);
            heading.push(d.heading);
            noise_per_ms.push(d.noise_per_ms);
            jamming_indicator.push(d.jamming_indicator);
            fix_type.push(d.fix_type);
            satellites_used.push(d.satellites_used);
        }
    }

    fn push_estimator(&mut self, d: &cc_protocol::cc_dialect::CC_TELEMETRY_ESTIMATOR_DATA) {
        if let Payload::Estimator { status_flags, velocity_test_ratio, position_test_ratio,
            height_test_ratio, mag_test_ratio, airspeed_test_ratio, innovation_check_flags,
            solution_status_flags } = self
        {
            status_flags.push(d.status_flags);
            velocity_test_ratio.push(d.velocity_test_ratio);
            position_test_ratio.push(d.position_test_ratio);
            height_test_ratio.push(d.height_test_ratio);
            mag_test_ratio.push(d.mag_test_ratio);
            airspeed_test_ratio.push(d.airspeed_test_ratio);
            innovation_check_flags.push(d.innovation_check_flags);
            solution_status_flags.push(d.solution_status_flags);
        }
    }

    fn push_actuator(&mut self, d: &cc_protocol::cc_dialect::CC_TELEMETRY_ACTUATOR_DATA) {
        if let Payload::Actuator { actuator_output, motor_count } = self {
            actuator_output.push(d.actuator_output);
            motor_count.push(d.motor_count);
        }
    }

    fn push_event(&mut self, d: &cc_protocol::cc_dialect::CC_EVENT_DATA) {
        if let Payload::Event { event_id, argument0, argument1, severity, subsystem } = self {
            event_id.push(d.event_id);
            argument0.push(d.argument0);
            argument1.push(d.argument1);
            severity.push(d.severity as u8);
            subsystem.push(d.subsystem as u8);
        }
    }

    fn push_safety(&mut self, d: &cc_protocol::cc_dialect::CC_SAFETY_STATUS_DATA) {
        if let Payload::Safety { last_report_sequence, active_health_flags, report_age_ms,
            missed_reports, companion_state, action_taken, reject_reason } = self
        {
            last_report_sequence.push(d.last_report_sequence);
            active_health_flags.push(d.active_health_flags.bits());
            report_age_ms.push(d.report_age_ms);
            missed_reports.push(d.missed_reports);
            companion_state.push(d.companion_state as u8);
            action_taken.push(d.action_taken as u8);
            reject_reason.push(d.reject_reason as u8);
        }
    }

    fn extend_columns(self, cols: &mut Vec<ArrayRef>) {
        match self {
            Payload::State { failsafe_flags, q, angular_velocity, position_ned, velocity_ned,
                heading, nav_state, arming_state, vehicle_type, estimator_valid, control_mode_flags } =>
            {
                cols.push(Arc::new(UInt32Array::from(failsafe_flags)));
                cols.push(fsl_col(&q));
                cols.push(fsl_col(&angular_velocity));
                cols.push(fsl_col(&position_ned));
                cols.push(fsl_col(&velocity_ned));
                cols.push(Arc::new(Float32Array::from(heading)));
                cols.push(Arc::new(UInt8Array::from(nav_state)));
                cols.push(Arc::new(UInt8Array::from(arming_state)));
                cols.push(Arc::new(UInt8Array::from(vehicle_type)));
                cols.push(Arc::new(UInt8Array::from(estimator_valid)));
                cols.push(Arc::new(UInt8Array::from(control_mode_flags)));
            }
            Payload::Imu { clipping_count, accel, gyro, delta_angle, delta_velocity,
                vibration_metric, temperature } =>
            {
                cols.push(Arc::new(UInt32Array::from(clipping_count)));
                cols.push(fsl_col(&accel));
                cols.push(fsl_col(&gyro));
                cols.push(fsl_col(&delta_angle));
                cols.push(fsl_col(&delta_velocity));
                cols.push(fsl_col(&vibration_metric));
                cols.push(Arc::new(Float32Array::from(temperature)));
            }
            Payload::Power { voltage, current, power, consumed_mah, remaining, temperature,
                cell_count, warning, connected } =>
            {
                cols.push(Arc::new(Float32Array::from(voltage)));
                cols.push(Arc::new(Float32Array::from(current)));
                cols.push(Arc::new(Float32Array::from(power)));
                cols.push(Arc::new(Float32Array::from(consumed_mah)));
                cols.push(Arc::new(Float32Array::from(remaining)));
                cols.push(Arc::new(Float32Array::from(temperature)));
                cols.push(Arc::new(UInt8Array::from(cell_count)));
                cols.push(Arc::new(UInt8Array::from(warning)));
                cols.push(Arc::new(UInt8Array::from(connected)));
            }
            Payload::Gps { lat, lon, alt, eph, epv, ground_speed, heading, noise_per_ms,
                jamming_indicator, fix_type, satellites_used } =>
            {
                cols.push(Arc::new(Int32Array::from(lat)));
                cols.push(Arc::new(Int32Array::from(lon)));
                cols.push(Arc::new(Int32Array::from(alt)));
                cols.push(Arc::new(Float32Array::from(eph)));
                cols.push(Arc::new(Float32Array::from(epv)));
                cols.push(Arc::new(Float32Array::from(ground_speed)));
                cols.push(Arc::new(Float32Array::from(heading)));
                cols.push(Arc::new(UInt16Array::from(noise_per_ms)));
                cols.push(Arc::new(UInt16Array::from(jamming_indicator)));
                cols.push(Arc::new(UInt8Array::from(fix_type)));
                cols.push(Arc::new(UInt8Array::from(satellites_used)));
            }
            Payload::Estimator { status_flags, velocity_test_ratio, position_test_ratio,
                height_test_ratio, mag_test_ratio, airspeed_test_ratio, innovation_check_flags,
                solution_status_flags } =>
            {
                cols.push(Arc::new(UInt32Array::from(status_flags)));
                cols.push(Arc::new(Float32Array::from(velocity_test_ratio)));
                cols.push(Arc::new(Float32Array::from(position_test_ratio)));
                cols.push(Arc::new(Float32Array::from(height_test_ratio)));
                cols.push(Arc::new(Float32Array::from(mag_test_ratio)));
                cols.push(Arc::new(Float32Array::from(airspeed_test_ratio)));
                cols.push(Arc::new(UInt16Array::from(innovation_check_flags)));
                cols.push(Arc::new(UInt16Array::from(solution_status_flags)));
            }
            Payload::Actuator { actuator_output, motor_count } => {
                cols.push(fsl_col(&actuator_output));
                cols.push(Arc::new(UInt8Array::from(motor_count)));
            }
            Payload::Event { event_id, argument0, argument1, severity, subsystem } => {
                cols.push(Arc::new(UInt32Array::from(event_id)));
                cols.push(Arc::new(UInt32Array::from(argument0)));
                cols.push(Arc::new(UInt32Array::from(argument1)));
                cols.push(Arc::new(UInt8Array::from(severity)));
                cols.push(Arc::new(UInt8Array::from(subsystem)));
            }
            Payload::Safety { last_report_sequence, active_health_flags, report_age_ms,
                missed_reports, companion_state, action_taken, reject_reason } =>
            {
                cols.push(Arc::new(UInt32Array::from(last_report_sequence)));
                cols.push(Arc::new(UInt32Array::from(active_health_flags)));
                cols.push(Arc::new(UInt32Array::from(report_age_ms)));
                cols.push(Arc::new(UInt32Array::from(missed_reports)));
                cols.push(Arc::new(UInt8Array::from(companion_state)));
                cols.push(Arc::new(UInt8Array::from(action_taken)));
                cols.push(Arc::new(UInt8Array::from(reject_reason)));
            }
        }
    }
}
