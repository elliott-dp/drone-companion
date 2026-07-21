//! The concrete, fully-resolved [`Config`] and its sections.
//!
//! Every field has a built-in default (the lowest precedence layer). The
//! layered loader in [`crate::layer`] overlays file → env → CLI on top of
//! `Config::default()`, and [`crate::validate`] runs the cross-field
//! invariants once at the end.

use std::path::PathBuf;

/// Transport selection (mirrors `cc-link`'s transport enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    Udp,
    Serial,
}

/// Parquet payload compression for the mission log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Snappy,
    Zstd,
}

/// PX4 parameter snapshot capture mode at handshake (dev-plan 5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamSnapshotMode {
    /// Real, bounded, non-blocking `PARAM_REQUEST_LIST` read.
    Real,
    /// Deterministic placeholder — the default for tests and the harness.
    Stub,
    /// Skip the snapshot entirely.
    Off,
}

/// `[general]` — identity and top-level paths.
#[derive(Debug, Clone, PartialEq)]
pub struct General {
    /// Vehicle identity; MUST match the PX4 param (spec §7). `0` is rejected.
    pub vehicle_id: u32,
    /// Root under which mission directories are minted.
    pub mission_root: PathBuf,
    /// Emit the machine-readable status line (companiond `--status-json`).
    pub status_json: bool,
}

/// `[transport]` — the CC-FC link (spec §2, §5.2).
#[derive(Debug, Clone, PartialEq)]
pub struct Transport {
    pub kind: TransportKind,
    /// UDP bind address (companiond listens here for the FC).
    pub udp_bind: String,
    /// Optional fixed remote; when absent the peer is learned on first
    /// validly-decoded frame (cc-link anti-hijack gate).
    pub remote: Option<String>,
    pub serial_path: Option<String>,
    pub baud: u32,
    pub sysid: u8,
}

/// `[mission_log]` — the Parquet dataset writer (spec §5.6).
#[derive(Debug, Clone, PartialEq)]
pub struct MissionLog {
    /// Seal a part after this many rows (also the parquet row-group size).
    pub flush_rows: u32,
    /// Seal a part after this many seconds even if `flush_rows` is not hit
    /// (evaluated on an independent ticker so silent streams still seal).
    pub flush_secs: u64,
    /// Roll to a new segment when the current one reaches this size…
    pub seg_cap_bytes: u64,
    /// …or this age.
    pub seg_cap_secs: u64,
    /// Append the length-prefixed `raw_mavlink.bin` ground-truth capture.
    pub raw_capture: bool,
    pub compression: Compression,
}

/// `[disk]` — the startup floor and the shedding ladder (spec §5.6).
#[derive(Debug, Clone, PartialEq)]
pub struct Disk {
    /// Refuse to *start* a mission below this free-space floor.
    pub floor_bytes: u64,
    /// Below this: stop appending raw_mavlink.bin (shed raw first).
    pub raw_shed_low_bytes: u64,
    /// Resume raw at or above this (hysteresis).
    pub raw_resume_bytes: u64,
    /// Below this: also drop Class B (imu) + Class F (actuator) rows.
    pub bf_shed_low_bytes: u64,
    pub bf_resume_bytes: u64,
    /// Below this: keep only the never-shed classes (state/power/gps/
    /// estimator/event/safety) + events + manifest.
    pub crit_low_bytes: u64,
    pub crit_resume_bytes: u64,
}

/// `[handshake]` — CC_MISSION_CONTEXT + param snapshot (dev-plan 5.3).
#[derive(Debug, Clone, PartialEq)]
pub struct Handshake {
    pub context_hz: f64,
    /// Phase 5: accept on first FC heartbeat. Phase 6 flips this to false
    /// and waits for CC_SAFETY_STATUS leaving UNKNOWN.
    pub stub_ack_on_heartbeat: bool,
    pub param_snapshot: ParamSnapshotMode,
    pub param_timeout_secs: u64,
}

/// The fully-resolved configuration handed to companiond.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    pub general: General,
    pub transport: Transport,
    pub mission_log: MissionLog,
    pub disk: Disk,
    pub handshake: Handshake,
}

impl Default for General {
    fn default() -> Self {
        Self {
            vehicle_id: 1,
            mission_root: PathBuf::from("/var/lib/companiond/missions"),
            status_json: false,
        }
    }
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            kind: TransportKind::Udp,
            udp_bind: "0.0.0.0:24040".to_string(),
            remote: None,
            serial_path: None,
            baud: 921_600,
            sysid: 1,
        }
    }
}

impl Default for MissionLog {
    fn default() -> Self {
        Self {
            flush_rows: 5_000,
            flush_secs: 10,
            seg_cap_bytes: 2 * 1024 * 1024 * 1024, // 2 GiB
            seg_cap_secs: 1_800,                    // 30 min
            raw_capture: true,
            compression: Compression::Zstd,
        }
    }
}

impl Default for Disk {
    fn default() -> Self {
        Self {
            floor_bytes: 5 * 1024 * 1024 * 1024,          // 5 GiB
            raw_shed_low_bytes: 2 * 1024 * 1024 * 1024,   // 2 GiB
            raw_resume_bytes: 4 * 1024 * 1024 * 1024,     // 4 GiB
            bf_shed_low_bytes: 1024 * 1024 * 1024,        // 1 GiB
            bf_resume_bytes: 1536 * 1024 * 1024,          // 1.5 GiB
            crit_low_bytes: 512 * 1024 * 1024,            // 512 MiB
            crit_resume_bytes: 768 * 1024 * 1024,         // 768 MiB
        }
    }
}

impl Default for Handshake {
    fn default() -> Self {
        Self {
            context_hz: 1.0,
            stub_ack_on_heartbeat: true,
            param_snapshot: ParamSnapshotMode::Stub,
            param_timeout_secs: 20,
        }
    }
}

