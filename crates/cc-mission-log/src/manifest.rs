//! The mission `manifest.json`: written **first** at mission open (with
//! `complete=false`) and rewritten atomically at each segment close and at
//! clean mission end. It records provenance (dialect hash, schema version,
//! sw version) and a per-segment / per-stream rollup.
//!
//! `log-inspect` treats the manifest as *advisory*: it recomputes the
//! authoritative counts from the part footers and reports any divergence as
//! recoverable (DIRTY), so a stale post-crash manifest never blocks reading
//! valid data.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::env::Syncer;
use crate::ids::write_atomic;
use crate::writer::StreamStats;
use crate::{Error, Result};

/// Current manifest schema version (the file format, not the wire schema).
pub const MANIFEST_VERSION: u32 = 1;

/// Why a segment was closed.
pub const CLOSE_CLEAN: &str = "clean";
pub const CLOSE_CC_RESTART: &str = "cc_restart";
pub const CLOSE_PX4_REBOOT: &str = "px4_reboot";
pub const CLOSE_ROTATION_CAP: &str = "rotation_cap";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StreamEntry {
    pub sealed_parts: u64,
    pub rows: u64,
    pub dropped: u64,
    pub first_cc_ns: Option<i64>,
    pub last_cc_ns: Option<i64>,
    pub seq_gap_total: u64,
    pub bytes: u64,
}

impl From<&StreamStats> for StreamEntry {
    fn from(s: &StreamStats) -> Self {
        Self {
            sealed_parts: s.sealed_parts,
            rows: s.rows,
            dropped: s.dropped,
            first_cc_ns: s.first_cc_ns,
            last_cc_ns: s.last_cc_ns,
            seq_gap_total: s.seq_gap_total,
            bytes: s.bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RawEntry {
    pub present: bool,
    pub bytes: u64,
    pub frames: u64,
    /// True if raw capture was ever shed during the segment (a torn/partial
    /// raw file is then expected, not an anomaly).
    pub shed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SegmentEntry {
    pub index: u32,
    pub dir: String,
    pub cc_boot_id: u32,
    pub px4_boot_id: u32,
    pub opened_wall_unix_ns: i64,
    pub closed_wall_unix_ns: Option<i64>,
    pub close_reason: Option<String>,
    pub streams: BTreeMap<String, StreamEntry>,
    pub raw_mavlink: RawEntry,
    pub drop_totals: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub manifest_version: u32,
    /// Flips to `true` only at a clean mission end.
    pub complete: bool,
    pub vehicle_id: u32,
    pub mission_id: u32,
    pub cc_sw_version: String,
    /// e.g. `"0xdc5b8e9f"`.
    pub dialect_hash: String,
    pub dialect_sha256: String,
    pub schema_version: u8,
    pub created_wall_unix_ns: i64,
    pub params: Option<ParamsSummary>,
    pub segments: Vec<SegmentEntry>,
}

/// Compact param-snapshot completeness recorded in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParamsSummary {
    pub mode: String,
    pub complete: bool,
    pub received: u32,
    pub expected: u32,
}

impl Manifest {
    /// A fresh manifest at mission open, provenance stamped from the running
    /// binary (so a dataset can be matched to the code that wrote it).
    pub fn new(vehicle_id: u32, mission_id: u32, cc_sw_version: String, created_wall_unix_ns: i64) -> Self {
        Self {
            manifest_version: MANIFEST_VERSION,
            complete: false,
            vehicle_id,
            mission_id,
            cc_sw_version,
            dialect_hash: format!("0x{:08x}", cc_protocol::dialect_hash::CC_DIALECT_HASH),
            dialect_sha256: cc_protocol::dialect_hash::CC_DIALECT_SHA256.to_string(),
            schema_version: cc_protocol::identity::CC_SCHEMA_VERSION,
            created_wall_unix_ns,
            params: None,
            segments: Vec::new(),
        }
    }

    pub fn path(mission_dir: &Path) -> std::path::PathBuf {
        mission_dir.join("manifest.json")
    }

    /// Serialize and write atomically (temp → fsync → rename → dir-fsync).
    pub fn write_atomic(&self, mission_dir: &Path, syncer: &Arc<dyn Syncer>) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(self)?;
        write_atomic(&Self::path(mission_dir), &bytes, syncer).map_err(Error::Io)
    }

    /// Read a manifest from a mission directory.
    pub fn read(mission_dir: &Path) -> Result<Manifest> {
        let text = std::fs::read_to_string(Self::path(mission_dir)).map_err(Error::Io)?;
        serde_json::from_str(&text).map_err(Error::Json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::NoopSyncer;

    fn tmpdir(tag: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("ccml-man-{}-{tag}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn round_trips_and_stamps_provenance() {
        let dir = tmpdir("rt");
        let s: Arc<dyn Syncer> = Arc::new(NoopSyncer);
        let m = Manifest::new(1, 42, "phase5-gabc".into(), 1_721_000_000_000_000_000);
        m.write_atomic(&dir, &s).unwrap();

        let read = Manifest::read(&dir).unwrap();
        assert_eq!(read, m);
        assert_eq!(read.dialect_hash, "0xdc5b8e9f");
        assert_eq!(read.schema_version, 1);
        assert!(!read.complete);
        // no .tmp left behind
        let tmps = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp")).count();
        assert_eq!(tmps, 0);
    }

    #[test]
    fn rewrite_preserves_old_on_reparse() {
        // A second atomic write replaces the file wholesale and stays parseable.
        let dir = tmpdir("rewrite");
        let s: Arc<dyn Syncer> = Arc::new(NoopSyncer);
        let mut m = Manifest::new(1, 7, "sw".into(), 100);
        m.write_atomic(&dir, &s).unwrap();
        m.complete = true;
        m.write_atomic(&dir, &s).unwrap();
        assert!(Manifest::read(&dir).unwrap().complete);
    }
}
