//! A segment: one on-disk directory holding every stream's part directory,
//! the operational event log, and the raw capture, for one
//! `(mission_id, cc_boot_id, px4_boot_id)` identity.
//!
//! Segments split on: companiond restart (new `cc_boot_id` — handled by the
//! mission opening a fresh segment), PX4 reboot (new `px4_boot_id`), or a
//! size/time rotation cap. Splitting bounds single-file loss and the replay
//! blast radius.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cc_config::Config;
use cc_ingest::{StreamId, TelemetryEvent};

use crate::batch::SegmentIdentity;
use crate::env::{Clock, Syncer};
use crate::events::{EventLog, EventRow};
use crate::health::LogHealth;
use crate::manifest::{RawEntry, SegmentEntry, StreamEntry};
use crate::raw::RawCapture;
use crate::shed::ShedStage;
use crate::writer::{ensure_stream_dir, StreamWriter};

pub struct Segment {
    index: u32,
    id: SegmentIdentity,
    writers: Vec<StreamWriter>, // one per StreamId, in StreamId order
    events: EventLog,
    raw: Option<RawCapture>,
    raw_shed_recorded: bool,
    opened_wall_ns: i64,
    clock: Arc<dyn Clock>,
    syncer: Arc<dyn Syncer>,
    health: Arc<LogHealth>,
}

impl Segment {
    /// Open (create) a segment directory and all its writers.
    pub fn open(
        dir: PathBuf,
        index: u32,
        id: SegmentIdentity,
        cfg: &Config,
        clock: Arc<dyn Clock>,
        syncer: Arc<dyn Syncer>,
        health: Arc<LogHealth>,
    ) -> crate::Result<Self> {
        std::fs::create_dir_all(&dir)?;

        let ml = &cfg.mission_log;
        let mut writers = Vec::with_capacity(8);
        for s in StreamId::ALL {
            let sdir = ensure_stream_dir(&dir, s)?;
            writers.push(StreamWriter::new(
                s, sdir, id, ml.flush_rows, ml.flush_secs, ml.compression,
                clock.clone(), syncer.clone(), health.clone(),
            ));
        }

        let events_dir = dir.join("events");
        std::fs::create_dir_all(&events_dir)?;
        let events = EventLog::new(
            events_dir, ml.compression, ml.flush_rows, ml.flush_secs,
            clock.clone(), syncer.clone(), health.clone(),
        );

        let raw = if ml.raw_capture {
            Some(RawCapture::create(&dir.join("raw_mavlink.bin"))?)
        } else {
            None
        };

        let opened_wall_ns = clock.wall_unix_ns();
        let mut seg = Self {
            index, id, writers, events, raw, raw_shed_recorded: false,
            opened_wall_ns, clock, syncer, health,
        };
        seg.events.record(EventRow {
            cc_receive_time_ns: seg.clock.wall_unix_ns(),
            kind: "open",
            stream_id: None,
            reason: Some(format!("segment_{:02}", seg.index)),
            shed_stage: 0,
            free_bytes: None,
            count: 1,
        });
        Ok(seg)
    }

    pub fn dir_name(&self) -> String {
        format!("segment_{:02}", self.index)
    }

    /// Route a telemetry event to its stream writer, or drop-and-account it if
    /// the shedding ladder has that class shed at the current stage.
    pub fn push_event(&mut self, ev: &TelemetryEvent, stage: ShedStage) {
        let stream = match ev {
            TelemetryEvent::State(..) => StreamId::State,
            TelemetryEvent::Imu(..) => StreamId::Imu,
            TelemetryEvent::Power(..) => StreamId::Power,
            TelemetryEvent::Gps(..) => StreamId::Gps,
            TelemetryEvent::Estimator(..) => StreamId::Estimator,
            TelemetryEvent::Actuator(..) => StreamId::Actuator,
            TelemetryEvent::Event(..) => StreamId::Event,
            TelemetryEvent::SafetyStatus(..) => StreamId::SafetyStatus,
            TelemetryEvent::LinkStatus(_) | TelemetryEvent::StreamStale(_) => return,
        };
        if crate::shed::stage_allows(stage, stream) {
            self.writers[stream as usize].push(ev);
        } else {
            // dropped by shedding — count + ledger it (coalesced counting is
            // fine because each drop increments the atomic; the events row is
            // per-drop here, low-volume at deep shed stages).
            self.health.add_dropped(stream, 1);
            self.events.record(EventRow {
                cc_receive_time_ns: self.clock.wall_unix_ns(),
                kind: "drop",
                stream_id: Some(stream as u8),
                reason: Some(format!("shed_{}", stage.name().to_ascii_lowercase())),
                shed_stage: stage.as_u8(),
                free_bytes: None,
                count: 1,
            });
        }
    }

    /// Append a raw wire frame if raw capture is enabled and not shed.
    pub fn append_raw(&mut self, frame: &[u8], raw_allowed: bool) {
        let Some(raw) = self.raw.as_mut() else { return };
        if raw_allowed {
            let _ = raw.append(frame);
        } else {
            if !self.raw_shed_recorded {
                raw.mark_shed();
                self.raw_shed_recorded = true;
                self.events.record(EventRow {
                    cc_receive_time_ns: self.clock.wall_unix_ns(),
                    kind: "shed",
                    stream_id: None,
                    reason: Some("raw".into()),
                    shed_stage: ShedStage::ShedRaw.as_u8(),
                    free_bytes: None,
                    count: 1,
                });
            }
            self.health.add_raw_dropped(1);
        }
    }

    /// Time-cap tick: seal silent streams, flush raw durably.
    pub fn tick(&mut self) {
        for w in &mut self.writers {
            w.tick();
        }
        self.events.tick();
        if let Some(raw) = self.raw.as_mut() {
            let _ = raw.flush(&self.syncer);
        }
    }

    /// Approximate on-disk size, for the rotation cap.
    pub fn size_bytes(&self) -> u64 {
        let stream_bytes: u64 = self.writers.iter().map(|w| w.stats().bytes).sum();
        let raw_bytes = self.raw.as_ref().map(|r| r.bytes()).unwrap_or(0);
        stream_bytes + raw_bytes
    }

    pub fn opened_wall_ns(&self) -> i64 {
        self.opened_wall_ns
    }

    /// Seal everything and produce the manifest entry for this segment.
    pub fn finalize(mut self, close_reason: &str) -> SegmentEntry {
        for w in &mut self.writers {
            w.finalize();
        }
        self.events.finalize();
        if let Some(raw) = self.raw.as_mut() {
            let _ = raw.flush(&self.syncer);
        }

        let mut streams = BTreeMap::new();
        let mut drop_totals = BTreeMap::new();
        for w in &self.writers {
            let st = w.stats();
            if st.dropped > 0 {
                drop_totals.insert(w.stream().name().to_string(), st.dropped);
            }
            streams.insert(w.stream().name().to_string(), StreamEntry::from(st));
        }

        let raw_mavlink = match &self.raw {
            Some(r) => {
                if r.frames() == 0 && r.shed() {
                    drop_totals.insert("raw".into(), self.health.snapshot().raw_dropped);
                }
                RawEntry { present: true, bytes: r.bytes(), frames: r.frames(), shed: r.shed() }
            }
            None => RawEntry::default(),
        };

        SegmentEntry {
            index: self.index,
            dir: self.dir_name(),
            cc_boot_id: self.id.cc_boot_id,
            px4_boot_id: self.id.px4_boot_id,
            opened_wall_unix_ns: self.opened_wall_ns,
            closed_wall_unix_ns: Some(self.clock.wall_unix_ns()),
            close_reason: Some(close_reason.to_string()),
            streams,
            raw_mavlink,
            drop_totals,
        }
    }
}

/// The directory name for segment `index` under a mission directory.
pub fn segment_dir(mission_dir: &Path, index: u32) -> PathBuf {
    mission_dir.join(format!("segment_{index:02}"))
}
