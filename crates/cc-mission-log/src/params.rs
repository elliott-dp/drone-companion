//! The PX4 parameter snapshot (`px4_params_snapshot.json`).
//!
//! Completeness is first-class: `expected_count` / `received_count` /
//! `timed_out` make a partial read a *deterministic representation*
//! ("received 812/840") rather than a silent stub — tests assert on the
//! representation, not on wall-clock timing. The `Stub` mode writes a fixed
//! placeholder so the crash / disk harness never depends on FC param timing.

use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::env::Syncer;
use crate::ids::write_atomic;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParamEntry {
    pub id: String,
    pub index: u32,
    /// `MAV_PARAM_TYPE`.
    #[serde(rename = "type")]
    pub type_: u8,
    /// IEEE-754 param value (PX4/QGC convention).
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParamSnapshot {
    pub captured_wall_unix_ns: i64,
    pub px4_boot_id: u32,
    pub mission_id: u32,
    pub mode: String,
    pub complete: bool,
    pub expected_count: u32,
    pub received_count: u32,
    pub timed_out: bool,
    pub params: Vec<ParamEntry>,
}

impl ParamSnapshot {
    /// A deterministic placeholder for tests / the harness.
    pub fn stub(px4_boot_id: u32, mission_id: u32, wall_ns: i64) -> Self {
        let params = vec![
            ParamEntry { id: "SYS_AUTOSTART".into(), index: 0, type_: 6, value: 10016.0 },
            ParamEntry { id: "MAV_PROTO_VER".into(), index: 1, type_: 6, value: 2.0 },
            ParamEntry { id: "CBRK_SUPPLY_CHK".into(), index: 2, type_: 6, value: 0.0 },
        ];
        Self {
            captured_wall_unix_ns: wall_ns,
            px4_boot_id,
            mission_id,
            mode: "stub".into(),
            complete: true,
            expected_count: params.len() as u32,
            received_count: params.len() as u32,
            timed_out: false,
            params,
        }
    }

    pub fn path(mission_dir: &Path) -> std::path::PathBuf {
        mission_dir.join("px4_params_snapshot.json")
    }

    /// Serialize (params sorted by index for a stable diff) and write durably.
    pub fn write_atomic(&self, mission_dir: &Path, syncer: &Arc<dyn Syncer>) -> Result<()> {
        let mut snap = self.clone();
        snap.params.sort_by_key(|p| p.index);
        let bytes = serde_json::to_vec_pretty(&snap)?;
        write_atomic(&Self::path(mission_dir), &bytes, syncer).map_err(Error::Io)
    }

    /// The compact summary embedded in the manifest.
    pub fn summary(&self) -> crate::manifest::ParamsSummary {
        crate::manifest::ParamsSummary {
            mode: self.mode.clone(),
            complete: self.complete,
            received: self.received_count,
            expected: self.expected_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::NoopSyncer;

    #[test]
    fn stub_snapshot_round_trips_and_is_complete() {
        let dir = std::env::temp_dir().join(format!("ccml-param-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let s: Arc<dyn Syncer> = Arc::new(NoopSyncer);
        let snap = ParamSnapshot::stub(3, 42, 1_721_000_000_000_000_000);
        snap.write_atomic(&dir, &s).unwrap();

        let text = std::fs::read_to_string(ParamSnapshot::path(&dir)).unwrap();
        let read: ParamSnapshot = serde_json::from_str(&text).unwrap();
        assert_eq!(read, snap);
        assert!(read.complete);
        assert_eq!(read.mode, "stub");
        assert!(text.contains("\"type\""), "MAV_PARAM_TYPE serialized as `type`");
    }
}
