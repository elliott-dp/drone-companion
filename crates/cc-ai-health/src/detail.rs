//! Detail-code namespace — the `u16` that travels in `CC_HEALTH_REPORT.detail_code`
//! and `CC_AI_DIAGNOSTIC.detail_code`, joining a finding to `ai_health.parquet`
//! for post-hoc evidence.
//!
//! Layout: `subsystem_block(1000·k) + code`, decimal for readability (battery
//! 1xxx, motor 2xxx, vibration 3xxx, gps 4xxx, estimator 5xxx, thermal 6xxx,
//! link 7xxx, mission 8xxx). Centralized here so codes never collide across
//! algorithms.

// battery (1xxx)
pub const BATT_SAG_BEYOND_MODEL: u16 = 1001;
pub const BATT_R_INT_STEP: u16 = 1002;
pub const BATT_CONSUMED_REMAINING_DIVERGENCE: u16 = 1003;
pub const BATT_GAUGE_NONMONOTONIC: u16 = 1004;
pub const BATT_UNDERVOLTAGE_UNDER_LOAD: u16 = 1005;
pub const BATT_PX4_WARNING: u16 = 1010;

// motor (2xxx)
pub const MOTOR_OUTPUT_OFFSET: u16 = 2001;
pub const MOTOR_ACTUATOR_SATURATION: u16 = 2002;
pub const MOTOR_HEADING_STATIONARY_ASYM: u16 = 2003;
pub const MOTOR_MULTI_SIGNAL: u16 = 2005;

// vibration (3xxx)
pub const VIBE_ACCEL_STEP: u16 = 3001;
pub const VIBE_GYRO_STEP: u16 = 3002;
pub const VIBE_CONING_STEP: u16 = 3003;
pub const VIBE_MULTI_METRIC: u16 = 3004;
pub const VIBE_CLIPPING_RATE: u16 = 3005;
pub const VIBE_ACCEL_ABSOLUTE: u16 = 3006;

// gps (4xxx)
pub const GPS_FIX_DEGRADED: u16 = 4001;
pub const GPS_LOW_SATS: u16 = 4002;
pub const GPS_EPH_HIGH: u16 = 4003;
pub const GPS_EPV_HIGH: u16 = 4004;
pub const GPS_JAMMING: u16 = 4005;
pub const GPS_NOISE_STEP: u16 = 4006;
pub const GPS_SPEED_DIVERGENCE: u16 = 4007;
pub const GPS_COMPOSITE: u16 = 4008;

// estimator (5xxx)
pub const EST_VEL_BREACH: u16 = 5001;
pub const EST_POS_BREACH: u16 = 5002;
pub const EST_HEIGHT_BREACH: u16 = 5003;
pub const EST_MAG_BREACH: u16 = 5004;
pub const EST_INNOV_FLAG_STREAK: u16 = 5006;
pub const EST_SOLUTION_LOSS: u16 = 5007;
pub const EST_MULTI_INDEPENDENT: u16 = 5008;

// thermal (6xxx)
pub const THERM_BATT_HIGH: u16 = 6001;
pub const THERM_BATT_RATE: u16 = 6002;
pub const THERM_IMU_HIGH: u16 = 6003;
pub const THERM_IMU_RATE: u16 = 6004;

// link (7xxx)
pub const LINK_DROP_RATE_HIGH: u16 = 7001;
pub const LINK_RATE_BELOW_NOMINAL: u16 = 7002;
pub const LINK_JITTER_HIGH: u16 = 7006;

// mission (8xxx)
pub const MISSION_ENERGY_RESERVE_LOW: u16 = 8001;
pub const MISSION_POINT_OF_NO_RETURN: u16 = 8002;

// availability reasons (9xxx) — carried in AlgoOutput::{Degraded,Unavailable}
pub const AVAIL_WARMUP: u16 = 9001;
pub const AVAIL_STREAM_STALE: u16 = 9002;
pub const AVAIL_NAN_INPUT: u16 = 9003;
pub const AVAIL_LOW_EXCITATION: u16 = 9004;
pub const AVAIL_NO_DATA: u16 = 9005;
