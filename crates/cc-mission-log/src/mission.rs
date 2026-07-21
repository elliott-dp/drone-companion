//! Mission lifecycle: directory minting, **resume-on-restart** (spec §7: a
//! companiond restart continues the *same* mission_id with a new segment,
//! never a silent new mission), segment rotation, disk polling, and the
//! atomic manifest bookkeeping.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use cc_config::Config;
use cc_ingest::TelemetryEvent;

use crate::batch::SegmentIdentity;
use crate::env::{Clock, SpaceProbe, Syncer};
use crate::health::LogHealth;
use crate::ids::mint_counter;
use crate::manifest::{
    Manifest, SegmentEntry, CLOSE_CLEAN, CLOSE_PX4_REBOOT, CLOSE_ROTATION_CAP,
};
use crate::segment::{segment_dir, Segment};
use crate::shed::ShedLadder;
use crate::{Error, Result};

/// Reason a mission failed to open.
#[derive(Debug)]
pub enum OpenError {
    /// Free space is below the configured startup floor (spec §5.6).
    BelowFloor { free: u64, floor: u64 },
    Io(Error),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::BelowFloor { free, floor } => {
                write!(f, "free space {free} below mission floor {floor}")
            }
            OpenError::Io(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for OpenError {}

pub struct Mission {
    mission_root: PathBuf,
    mission_dir: PathBuf,
    cfg: Config,
    clock: Arc<dyn Clock>,
    space: Arc<dyn SpaceProbe>,
    syncer: Arc<dyn Syncer>,
    health: Arc<LogHealth>,
    manifest: Manifest,
    id_base: SegmentIdentity,
    px4_boot_id: u32,
    seg: Option<Segment>,
    seg_index: u32,
    ladder: ShedLadder,
    seg_opened_mono: i64,
}

impl Mission {
    /// Open (or resume) a mission. Mints `cc_boot_id` fresh; reuses an
    /// incomplete mission's `mission_id` if one exists for this vehicle,
    /// otherwise mints a new `mission_id`.
    #[allow(clippy::too_many_arguments)]
    pub fn open(
        cfg: Config,
        clock: Arc<dyn Clock>,
        space: Arc<dyn SpaceProbe>,
        syncer: Arc<dyn Syncer>,
        health: Arc<LogHealth>,
        sw_version: String,
        px4_boot_id: u32,
        mut warn: impl FnMut(&str),
    ) -> std::result::Result<Mission, OpenError> {
        let mission_root = cfg.general.mission_root.clone();
        std::fs::create_dir_all(&mission_root).map_err(|e| OpenError::Io(Error::Io(e)))?;

        // Startup floor gate: a mission storage precondition, checked once.
        let free = space.free_bytes(&mission_root).map_err(|e| OpenError::Io(Error::Io(e)))?;
        if free < cfg.disk.floor_bytes {
            return Err(OpenError::BelowFloor { free, floor: cfg.disk.floor_bytes });
        }

        let cc_boot_id = mint_counter(&mission_root.join("cc_boot_seq"), &syncer, &mut warn) as u32;
        let vehicle_id = cfg.general.vehicle_id;

        // Resume an incomplete mission for this vehicle, else mint a new one.
        let (mission_id, mission_dir, mut manifest, seg_index) =
            match find_incomplete(&mission_root, vehicle_id) {
                Some((dir, mut m)) => {
                    let idx = m.segments.len() as u32;
                    warn(&format!(
                        "resuming incomplete mission {} at segment_{:02} (spec §7)",
                        m.mission_id, idx
                    ));
                    // Retroactively finalize the segment the crashed process
                    // left open: recompute its stats from the sealed parts on
                    // disk and stamp close_reason="cc_restart", so the manifest
                    // reconciles and a cleanly-resumed mission reads Clean.
                    if let Some(last) = m.segments.last_mut() {
                        if last.closed_wall_unix_ns.is_none() {
                            let seg_dir = dir.join(&last.dir);
                            last.streams = crate::inspect::recompute_segment_streams(&seg_dir);
                            last.closed_wall_unix_ns = Some(clock.wall_unix_ns());
                            last.close_reason = Some(crate::manifest::CLOSE_CC_RESTART.to_string());
                        }
                    }
                    (m.mission_id, dir, m, idx)
                }
                None => {
                    let mission_id =
                        mint_counter(&mission_root.join("mission_seq"), &syncer, &mut warn) as u32;
                    let dir = mission_root.join(format!("mission_{mission_id:06}"));
                    std::fs::create_dir_all(&dir).map_err(|e| OpenError::Io(Error::Io(e)))?;
                    let m = Manifest::new(vehicle_id, mission_id, sw_version, clock.wall_unix_ns());
                    (mission_id, dir, m, 0)
                }
            };

        let id_base = SegmentIdentity { vehicle_id, mission_id, cc_boot_id, px4_boot_id };
        let ladder = ShedLadder::new(cfg.disk.clone());
        health.set_free_bytes(free);

        let mut mission = Mission {
            mission_root,
            mission_dir,
            cfg,
            clock: clock.clone(),
            space,
            syncer,
            health,
            manifest: std::mem::replace(&mut manifest, Manifest::new(0, 0, String::new(), 0)),
            id_base,
            px4_boot_id,
            seg: None,
            seg_index,
            ladder,
            seg_opened_mono: clock.mono_ns(),
        };
        mission.open_segment().map_err(OpenError::Io)?;
        Ok(mission)
    }

    pub fn mission_dir(&self) -> &Path {
        &self.mission_dir
    }
    pub fn mission_id(&self) -> u32 {
        self.id_base.mission_id
    }
    pub fn cc_boot_id(&self) -> u32 {
        self.id_base.cc_boot_id
    }

    /// Open a new segment at `self.seg_index`, append an "open" placeholder to
    /// the manifest, and persist it (so a crash still leaves a manifest that
    /// references the segment directory).
    fn open_segment(&mut self) -> Result<()> {
        let id = SegmentIdentity { px4_boot_id: self.px4_boot_id, ..self.id_base };
        let dir = segment_dir(&self.mission_dir, self.seg_index);
        let seg = Segment::open(
            dir, self.seg_index, id, &self.cfg,
            self.clock.clone(), self.syncer.clone(), self.health.clone(),
        )?;
        // placeholder entry (closed=null); replaced with real stats on close.
        self.manifest.segments.push(SegmentEntry {
            index: self.seg_index,
            dir: seg.dir_name(),
            cc_boot_id: id.cc_boot_id,
            px4_boot_id: id.px4_boot_id,
            opened_wall_unix_ns: seg.opened_wall_ns(),
            closed_wall_unix_ns: None,
            close_reason: None,
            streams: Default::default(),
            raw_mavlink: Default::default(),
            drop_totals: Default::default(),
        });
        self.seg = Some(seg);
        self.seg_opened_mono = self.clock.mono_ns();
        self.manifest.write_atomic(&self.mission_dir, &self.syncer)
    }

    /// Finalize the current segment, replacing its placeholder manifest entry
    /// with the sealed rollup, and persist.
    fn close_segment(&mut self, reason: &str) -> Result<()> {
        if let Some(seg) = self.seg.take() {
            let entry = seg.finalize(reason);
            if let Some(last) = self.manifest.segments.last_mut() {
                *last = entry;
            }
            self.manifest.write_atomic(&self.mission_dir, &self.syncer)?;
        }
        Ok(())
    }

    fn rotate(&mut self, reason: &str) -> Result<()> {
        self.close_segment(reason)?;
        self.seg_index += 1;
        self.open_segment()
    }

    /// Route a telemetry event into the current segment.
    pub fn on_event(&mut self, ev: &TelemetryEvent) {
        if let Some(seg) = self.seg.as_mut() {
            seg.push_event(ev, self.ladder.stage());
        }
    }

    /// Feed a raw wire frame (tapped pre-decode) into the raw capture.
    pub fn on_raw(&mut self, frame: &[u8]) {
        let allowed = self.ladder.raw_allowed();
        if let Some(seg) = self.seg.as_mut() {
            seg.append_raw(frame, allowed);
        }
    }

    /// Record telemetry events lost to broadcast lag (slow disk); the logger
    /// is a lossy subscriber, so this never back-pressures RX.
    pub fn note_lag(&self, n: u64) {
        self.health.add_lagged(n);
    }

    /// React to a PX4 boot-id change: rotate into a fresh segment.
    pub fn on_boot_change(&mut self, new_px4_boot_id: u32) -> Result<()> {
        if new_px4_boot_id != self.px4_boot_id {
            self.px4_boot_id = new_px4_boot_id;
            self.rotate(CLOSE_PX4_REBOOT)?;
        }
        Ok(())
    }

    /// Periodic tick: refresh the shed ladder from free space, seal silent
    /// streams / flush raw, and rotate on the size/time cap.
    pub fn tick(&mut self) -> Result<()> {
        if let Ok(free) = self.space.free_bytes(&self.mission_root) {
            self.ladder.update(free);
            self.health.set_stage(self.ladder.stage());
            self.health.set_free_bytes(free);
        }
        if let Some(seg) = self.seg.as_mut() {
            seg.tick();
        }
        let size = self.seg.as_ref().map(|s| s.size_bytes()).unwrap_or(0);
        let age_ns = self.clock.mono_ns() - self.seg_opened_mono;
        let cap_hit = size >= self.cfg.mission_log.seg_cap_bytes
            || age_ns >= (self.cfg.mission_log.seg_cap_secs as i64) * 1_000_000_000;
        if cap_hit {
            self.rotate(CLOSE_ROTATION_CAP)?;
        }
        Ok(())
    }

    /// Clean shutdown: seal the final segment and mark the mission complete.
    pub fn finalize(mut self) -> Result<()> {
        self.close_segment(CLOSE_CLEAN)?;
        self.manifest.complete = true;
        self.manifest.write_atomic(&self.mission_dir, &self.syncer)
    }
}

/// Scan `mission_root` for an incomplete mission (`complete=false`) belonging
/// to `vehicle_id`; return the highest-numbered such mission's dir + manifest.
fn find_incomplete(mission_root: &Path, vehicle_id: u32) -> Option<(PathBuf, Manifest)> {
    let mut best: Option<(u32, PathBuf, Manifest)> = None;
    for entry in std::fs::read_dir(mission_root).ok()?.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let name = entry.file_name();
        if !name.to_string_lossy().starts_with("mission_") {
            continue;
        }
        if let Ok(m) = Manifest::read(&dir) {
            if !m.complete && m.vehicle_id == vehicle_id {
                let better = best.as_ref().map(|(id, ..)| m.mission_id > *id).unwrap_or(true);
                if better {
                    best = Some((m.mission_id, dir, m));
                }
            }
        }
    }
    best.map(|(_, dir, m)| (dir, m))
}
