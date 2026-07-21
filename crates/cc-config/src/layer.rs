//! Precedence merge: **defaults → file → env → CLI** (highest wins,
//! per-field, not per-struct).
//!
//! Each layer is parsed into an all-`Option` [`PartialConfig`] mirror; the
//! layers are overlaid (a higher layer's `Some` replaces a lower one), and
//! the result is applied on top of [`Config::default`]. Enum-valued fields
//! travel as strings through every layer and are parsed in exactly one place
//! ([`Config::apply`]) so a bad value produces the same error whether it came
//! from the file, the environment, or the command line.

use serde::Deserialize;

use crate::model::{
    Compression, Config, ParamSnapshotMode, TransportKind,
};
use crate::ConfigError;

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct PGeneral {
    pub vehicle_id: Option<u32>,
    pub mission_root: Option<String>,
    pub status_json: Option<bool>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct PTransport {
    pub kind: Option<String>,
    pub udp_bind: Option<String>,
    pub remote: Option<String>,
    pub serial_path: Option<String>,
    pub baud: Option<u32>,
    pub sysid: Option<u8>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct PMissionLog {
    pub flush_rows: Option<u32>,
    pub flush_secs: Option<u64>,
    pub seg_cap_bytes: Option<u64>,
    pub seg_cap_secs: Option<u64>,
    pub raw_capture: Option<bool>,
    pub compression: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct PDisk {
    pub floor_bytes: Option<u64>,
    pub raw_shed_low_bytes: Option<u64>,
    pub raw_resume_bytes: Option<u64>,
    pub bf_shed_low_bytes: Option<u64>,
    pub bf_resume_bytes: Option<u64>,
    pub crit_low_bytes: Option<u64>,
    pub crit_resume_bytes: Option<u64>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct PHandshake {
    pub context_hz: Option<f64>,
    pub stub_ack_on_heartbeat: Option<bool>,
    pub param_snapshot: Option<String>,
    pub param_timeout_secs: Option<u64>,
}

/// All-`Option` mirror of [`Config`]; one instance per precedence layer.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PartialConfig {
    pub(crate) general: PGeneral,
    pub(crate) transport: PTransport,
    pub(crate) mission_log: PMissionLog,
    pub(crate) disk: PDisk,
    pub(crate) handshake: PHandshake,
}

/// A flat, public set of command-line overrides. The binary fills the fields
/// it parsed; [`Overrides::into_partial`] maps them onto the highest-precedence
/// [`PartialConfig`] layer (keeping the per-section partial types private).
#[derive(Debug, Default, Clone)]
pub struct Overrides {
    pub vehicle_id: Option<u32>,
    pub mission_root: Option<String>,
    pub status_json: Option<bool>,
    pub transport_kind: Option<String>,
    pub udp_bind: Option<String>,
    pub remote: Option<String>,
    pub serial_path: Option<String>,
    pub baud: Option<u32>,
    pub sysid: Option<u8>,
    pub disk_floor_bytes: Option<u64>,
    pub param_snapshot: Option<String>,
}

impl Overrides {
    pub fn into_partial(self) -> PartialConfig {
        PartialConfig {
            general: PGeneral {
                vehicle_id: self.vehicle_id,
                mission_root: self.mission_root,
                status_json: self.status_json,
            },
            transport: PTransport {
                kind: self.transport_kind,
                udp_bind: self.udp_bind,
                remote: self.remote,
                serial_path: self.serial_path,
                baud: self.baud,
                sysid: self.sysid,
            },
            mission_log: PMissionLog::default(),
            disk: PDisk { floor_bytes: self.disk_floor_bytes, ..PDisk::default() },
            handshake: PHandshake { param_snapshot: self.param_snapshot, ..PHandshake::default() },
        }
    }
}

/// `hi.or(lo)` per field — a higher-precedence `Some` wins.
macro_rules! overlay_fields {
    ($lo:expr, $hi:expr, $($f:ident),+ $(,)?) => {
        $( $lo.$f = $hi.$f.take().or($lo.$f.take()); )+
    };
}

impl PartialConfig {
    /// Parse a TOML document. A malformed document is a hard error; an empty
    /// one yields an all-`None` partial.
    pub fn from_toml_str(s: &str) -> Result<Self, ConfigError> {
        toml::from_str(s).map_err(|e| ConfigError::File(e.to_string()))
    }

    /// Read `CC_<SECTION>__<FIELD>` variables via a getter (injected so the
    /// merge is testable without touching the process environment).
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let mut p = PartialConfig::default();
        p.general.vehicle_id = env_parse(&get, "CC_GENERAL__VEHICLE_ID")?;
        p.general.mission_root = get("CC_GENERAL__MISSION_ROOT");
        p.general.status_json = env_parse(&get, "CC_GENERAL__STATUS_JSON")?;
        p.transport.kind = get("CC_TRANSPORT__KIND");
        p.transport.udp_bind = get("CC_TRANSPORT__UDP_BIND");
        p.transport.remote = get("CC_TRANSPORT__REMOTE");
        p.transport.serial_path = get("CC_TRANSPORT__SERIAL_PATH");
        p.transport.baud = env_parse(&get, "CC_TRANSPORT__BAUD")?;
        p.transport.sysid = env_parse(&get, "CC_TRANSPORT__SYSID")?;
        p.mission_log.flush_rows = env_parse(&get, "CC_MISSION_LOG__FLUSH_ROWS")?;
        p.mission_log.flush_secs = env_parse(&get, "CC_MISSION_LOG__FLUSH_SECS")?;
        p.mission_log.seg_cap_bytes = env_parse(&get, "CC_MISSION_LOG__SEG_CAP_BYTES")?;
        p.mission_log.seg_cap_secs = env_parse(&get, "CC_MISSION_LOG__SEG_CAP_SECS")?;
        p.mission_log.raw_capture = env_parse(&get, "CC_MISSION_LOG__RAW_CAPTURE")?;
        p.mission_log.compression = get("CC_MISSION_LOG__COMPRESSION");
        p.disk.floor_bytes = env_parse(&get, "CC_DISK__FLOOR_BYTES")?;
        p.disk.raw_shed_low_bytes = env_parse(&get, "CC_DISK__RAW_SHED_LOW_BYTES")?;
        p.disk.raw_resume_bytes = env_parse(&get, "CC_DISK__RAW_RESUME_BYTES")?;
        p.disk.bf_shed_low_bytes = env_parse(&get, "CC_DISK__BF_SHED_LOW_BYTES")?;
        p.disk.bf_resume_bytes = env_parse(&get, "CC_DISK__BF_RESUME_BYTES")?;
        p.disk.crit_low_bytes = env_parse(&get, "CC_DISK__CRIT_LOW_BYTES")?;
        p.disk.crit_resume_bytes = env_parse(&get, "CC_DISK__CRIT_RESUME_BYTES")?;
        p.handshake.context_hz = env_parse(&get, "CC_HANDSHAKE__CONTEXT_HZ")?;
        p.handshake.stub_ack_on_heartbeat = env_parse(&get, "CC_HANDSHAKE__STUB_ACK_ON_HEARTBEAT")?;
        p.handshake.param_snapshot = get("CC_HANDSHAKE__PARAM_SNAPSHOT");
        p.handshake.param_timeout_secs = env_parse(&get, "CC_HANDSHAKE__PARAM_TIMEOUT_SECS")?;
        Ok(p)
    }

    /// Overlay a higher-precedence layer on top of `self` (in place).
    pub fn overlay(&mut self, mut hi: PartialConfig) {
        overlay_fields!(self.general, hi.general, vehicle_id, mission_root, status_json);
        overlay_fields!(
            self.transport, hi.transport,
            kind, udp_bind, remote, serial_path, baud, sysid
        );
        overlay_fields!(
            self.mission_log, hi.mission_log,
            flush_rows, flush_secs, seg_cap_bytes, seg_cap_secs, raw_capture, compression
        );
        overlay_fields!(
            self.disk, hi.disk,
            floor_bytes, raw_shed_low_bytes, raw_resume_bytes,
            bf_shed_low_bytes, bf_resume_bytes, crit_low_bytes, crit_resume_bytes
        );
        overlay_fields!(
            self.handshake, hi.handshake,
            context_hz, stub_ack_on_heartbeat, param_snapshot, param_timeout_secs
        );
    }
}

fn env_parse<T: std::str::FromStr>(
    get: &impl Fn(&str) -> Option<String>,
    key: &str,
) -> Result<Option<T>, ConfigError> {
    match get(key) {
        None => Ok(None),
        Some(v) => v
            .parse::<T>()
            .map(Some)
            .map_err(|_| ConfigError::Env(format!("{key}: cannot parse {v:?}"))),
    }
}

fn parse_transport_kind(s: &str) -> Result<TransportKind, ConfigError> {
    match s.to_ascii_lowercase().as_str() {
        "udp" => Ok(TransportKind::Udp),
        "serial" => Ok(TransportKind::Serial),
        other => Err(ConfigError::Value(format!("transport.kind: {other:?} (want udp|serial)"))),
    }
}

fn parse_compression(s: &str) -> Result<Compression, ConfigError> {
    match s.to_ascii_lowercase().as_str() {
        "none" => Ok(Compression::None),
        "snappy" | "snap" => Ok(Compression::Snappy),
        "zstd" => Ok(Compression::Zstd),
        other => Err(ConfigError::Value(format!(
            "mission_log.compression: {other:?} (want none|snappy|zstd)"
        ))),
    }
}

fn parse_param_mode(s: &str) -> Result<ParamSnapshotMode, ConfigError> {
    match s.to_ascii_lowercase().as_str() {
        "real" => Ok(ParamSnapshotMode::Real),
        "stub" => Ok(ParamSnapshotMode::Stub),
        "off" => Ok(ParamSnapshotMode::Off),
        other => Err(ConfigError::Value(format!(
            "handshake.param_snapshot: {other:?} (want real|stub|off)"
        ))),
    }
}

impl Config {
    /// Apply an overlaid partial on top of `self` (which begins as
    /// [`Config::default`]). Enum strings are parsed here — the single place.
    pub(crate) fn apply(&mut self, p: PartialConfig) -> Result<(), ConfigError> {
        let PartialConfig { general, transport, mission_log, disk, handshake } = p;

        if let Some(v) = general.vehicle_id { self.general.vehicle_id = v; }
        if let Some(v) = general.mission_root { self.general.mission_root = v.into(); }
        if let Some(v) = general.status_json { self.general.status_json = v; }

        if let Some(v) = transport.kind { self.transport.kind = parse_transport_kind(&v)?; }
        if let Some(v) = transport.udp_bind { self.transport.udp_bind = v; }
        if let Some(v) = transport.remote { self.transport.remote = Some(v); }
        if let Some(v) = transport.serial_path { self.transport.serial_path = Some(v); }
        if let Some(v) = transport.baud { self.transport.baud = v; }
        if let Some(v) = transport.sysid { self.transport.sysid = v; }

        if let Some(v) = mission_log.flush_rows { self.mission_log.flush_rows = v; }
        if let Some(v) = mission_log.flush_secs { self.mission_log.flush_secs = v; }
        if let Some(v) = mission_log.seg_cap_bytes { self.mission_log.seg_cap_bytes = v; }
        if let Some(v) = mission_log.seg_cap_secs { self.mission_log.seg_cap_secs = v; }
        if let Some(v) = mission_log.raw_capture { self.mission_log.raw_capture = v; }
        if let Some(v) = mission_log.compression { self.mission_log.compression = parse_compression(&v)?; }

        if let Some(v) = disk.floor_bytes { self.disk.floor_bytes = v; }
        if let Some(v) = disk.raw_shed_low_bytes { self.disk.raw_shed_low_bytes = v; }
        if let Some(v) = disk.raw_resume_bytes { self.disk.raw_resume_bytes = v; }
        if let Some(v) = disk.bf_shed_low_bytes { self.disk.bf_shed_low_bytes = v; }
        if let Some(v) = disk.bf_resume_bytes { self.disk.bf_resume_bytes = v; }
        if let Some(v) = disk.crit_low_bytes { self.disk.crit_low_bytes = v; }
        if let Some(v) = disk.crit_resume_bytes { self.disk.crit_resume_bytes = v; }

        if let Some(v) = handshake.context_hz { self.handshake.context_hz = v; }
        if let Some(v) = handshake.stub_ack_on_heartbeat { self.handshake.stub_ack_on_heartbeat = v; }
        if let Some(v) = handshake.param_snapshot { self.handshake.param_snapshot = parse_param_mode(&v)?; }
        if let Some(v) = handshake.param_timeout_secs { self.handshake.param_timeout_secs = v; }

        Ok(())
    }
}
