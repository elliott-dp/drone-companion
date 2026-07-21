//! The mission-dataset reader behind `log-inspect`.
//!
//! It **recomputes** authoritative per-stream counts, time ranges and gap
//! totals directly from the Parquet part footers and treats the manifest as
//! advisory (cross-checked, never trusted blindly). The verdict is
//! three-state:
//!
//! * **Clean** (exit 0): complete mission, dialect/schema match, every part
//!   footer valid, no leftover in-progress files, rollup reconciles, zero
//!   drops.
//! * **Dirty** (exit 1): recoverable — a killed-but-intact dataset (the
//!   headline crash-test success state), a stale manifest, disk-pressure
//!   drops, a torn raw tail. Usable with bounded, known loss.
//! * **Corrupt** (exit 2): unreadable or wrong-binary — missing/unparseable
//!   manifest, dialect-hash mismatch, or a sealed part with a broken footer.

use std::path::{Path, PathBuf};

use arrow::array::{Int64Array, StringArray, UInt32Array, UInt64Array, UInt8Array};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

use crate::manifest::Manifest;
use crate::part::{is_inprogress, is_sealed_part};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Clean,
    Dirty(Vec<String>),
    Corrupt(Vec<String>),
}

impl Verdict {
    pub fn exit_code(&self) -> i32 {
        match self {
            Verdict::Clean => 0,
            Verdict::Dirty(_) => 1,
            Verdict::Corrupt(_) => 2,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Verdict::Clean => "CLEAN",
            Verdict::Dirty(_) => "DIRTY",
            Verdict::Corrupt(_) => "CORRUPT",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct StreamReport {
    pub name: String,
    pub parts: u64,
    pub rows: u64,
    pub first_cc_ns: Option<i64>,
    pub last_cc_ns: Option<i64>,
    pub seq_gap_total: u64,
}

#[derive(Debug, Clone, Default)]
pub struct SegReport {
    pub dir: String,
    pub cc_boot_id: u32,
    pub px4_boot_id: u32,
    pub closed: bool,
    pub streams: Vec<StreamReport>,
    pub inprogress_parts: u64,
    pub raw_present: bool,
    pub raw_frames: u64,
    pub raw_torn: bool,
    pub drops: u64,
}

#[derive(Debug, Clone)]
pub struct Report {
    pub mission_dir: PathBuf,
    pub mission_id: u32,
    pub vehicle_id: u32,
    pub complete: bool,
    pub dialect_hash: String,
    pub dialect_hash_ok: bool,
    pub schema_version_ok: bool,
    pub segments: Vec<SegReport>,
    pub verdict: Verdict,
}

impl Report {
    pub fn exit_code(&self) -> i32 {
        self.verdict.exit_code()
    }
    pub fn total_rows(&self) -> u64 {
        self.segments.iter().flat_map(|s| s.streams.iter()).map(|st| st.rows).sum()
    }
    pub fn total_drops(&self) -> u64 {
        self.segments.iter().map(|s| s.drops).sum()
    }
}

/// Inspect a mission directory and produce a [`Report`].
pub fn inspect_mission(mission_dir: &Path) -> Report {
    let mut corrupt: Vec<String> = Vec::new();
    let mut dirty: Vec<String> = Vec::new();

    // Manifest is required to be present + parseable; otherwise Corrupt.
    let manifest = match Manifest::read(mission_dir) {
        Ok(m) => m,
        Err(e) => {
            return Report {
                mission_dir: mission_dir.to_path_buf(),
                mission_id: 0,
                vehicle_id: 0,
                complete: false,
                dialect_hash: String::new(),
                dialect_hash_ok: false,
                schema_version_ok: false,
                segments: vec![],
                verdict: Verdict::Corrupt(vec![format!("manifest unreadable: {e}")]),
            };
        }
    };

    let expected_hash = format!("0x{:08x}", cc_protocol::dialect_hash::CC_DIALECT_HASH);
    let dialect_hash_ok = manifest.dialect_hash == expected_hash
        && manifest.dialect_sha256 == cc_protocol::dialect_hash::CC_DIALECT_SHA256;
    let schema_version_ok = manifest.schema_version == cc_protocol::identity::CC_SCHEMA_VERSION;

    if !dialect_hash_ok {
        corrupt.push(format!(
            "dialect hash mismatch: dataset {} vs running {expected_hash}",
            manifest.dialect_hash
        ));
    }
    if !schema_version_ok {
        corrupt.push(format!("schema version mismatch: dataset {}", manifest.schema_version));
    }
    if !manifest.complete {
        dirty.push("mission not marked complete (killed mid-mission?)".into());
    }

    let mut segments = Vec::new();
    for seg_entry in &manifest.segments {
        let seg_dir = mission_dir.join(&seg_entry.dir);
        let mut seg = SegReport {
            dir: seg_entry.dir.clone(),
            cc_boot_id: seg_entry.cc_boot_id,
            px4_boot_id: seg_entry.px4_boot_id,
            closed: seg_entry.closed_wall_unix_ns.is_some() && seg_entry.close_reason.is_some(),
            ..Default::default()
        };
        if !seg.closed {
            dirty.push(format!("{}: segment not cleanly closed", seg_entry.dir));
        }

        // Recompute every stream from its part footers.
        for s in cc_ingest::StreamId::ALL {
            let sdir = seg_dir.join(s.name());
            match scan_stream(&sdir, s.name()) {
                Ok((report, inprogress)) => {
                    seg.inprogress_parts += inprogress;
                    // reconcile against the manifest rollup for closed segments
                    if seg.closed {
                        if let Some(me) = seg_entry.streams.get(s.name()) {
                            if me.rows != report.rows {
                                dirty.push(format!(
                                    "{}/{}: manifest rows {} != on-disk {}",
                                    seg_entry.dir, s.name(), me.rows, report.rows
                                ));
                            }
                        }
                    }
                    seg.streams.push(report);
                }
                Err(fault) => corrupt.push(format!("{}/{}: {fault}", seg_entry.dir, s.name())),
            }
        }
        if seg.inprogress_parts > 0 {
            dirty.push(format!("{}: {} in-progress part(s) (crash artifact)", seg_entry.dir, seg.inprogress_parts));
        }

        // Operational log → drop totals.
        match scan_events(&seg_dir.join("events")) {
            Ok((drops, inprog)) => {
                seg.drops = drops;
                if drops > 0 {
                    dirty.push(format!("{}: {} dropped row(s) (disk pressure)", seg_entry.dir, drops));
                }
                if inprog > 0 {
                    dirty.push(format!("{}/events: {inprog} in-progress part(s)", seg_entry.dir));
                }
            }
            Err(fault) => corrupt.push(format!("{}/events: {fault}", seg_entry.dir)),
        }

        // Raw capture (if present) → torn-tail detection.
        let raw_path = seg_dir.join("raw_mavlink.bin");
        if raw_path.exists() {
            seg.raw_present = true;
            let (frames, torn) = scan_raw(&raw_path);
            seg.raw_frames = frames;
            seg.raw_torn = torn;
            if torn {
                dirty.push(format!("{}: raw_mavlink.bin has a torn trailing frame", seg_entry.dir));
            }
        }

        segments.push(seg);
    }

    let verdict = if !corrupt.is_empty() {
        Verdict::Corrupt(corrupt)
    } else if !dirty.is_empty() {
        Verdict::Dirty(dirty)
    } else {
        Verdict::Clean
    };

    Report {
        mission_dir: mission_dir.to_path_buf(),
        mission_id: manifest.mission_id,
        vehicle_id: manifest.vehicle_id,
        complete: manifest.complete,
        dialect_hash: manifest.dialect_hash,
        dialect_hash_ok,
        schema_version_ok,
        segments,
        verdict,
    }
}

/// Scan one stream's part directory. Returns (report, in_progress_count) or a
/// fault string for a Corrupt verdict (a sealed part with a broken footer).
fn scan_stream(dir: &Path, name: &str) -> std::result::Result<(StreamReport, u64), String> {
    let mut report = StreamReport { name: name.to_string(), ..Default::default() };
    let mut inprogress = 0u64;
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok((report, 0)), // stream dir absent = zero rows (fine)
    };
    let mut parts: Vec<PathBuf> = Vec::new();
    for e in entries.flatten() {
        let fname = e.file_name().to_string_lossy().to_string();
        if is_inprogress(&fname) {
            inprogress += 1;
        } else if is_sealed_part(&fname) {
            parts.push(e.path());
        }
    }
    parts.sort();

    for p in parts {
        let f = std::fs::File::open(&p).map_err(|e| format!("open {}: {e}", p.display()))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(f)
            .map_err(|e| format!("bad footer in {}: {e}", p.display()))?;
        let reader = builder.build().map_err(|e| format!("{}: {e}", p.display()))?;
        report.parts += 1;
        for batch in reader {
            let batch = batch.map_err(|e| format!("{}: {e}", p.display()))?;
            report.rows += batch.num_rows() as u64;
            let rows = batch.num_rows();
            if let Some(cc) = batch
                .column_by_name("cc_receive_time_ns")
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>())
            {
                for i in 0..rows {
                    let v = cc.value(i);
                    report.first_cc_ns = Some(report.first_cc_ns.map_or(v, |m| m.min(v)));
                    report.last_cc_ns = Some(report.last_cc_ns.map_or(v, |m| m.max(v)));
                }
            }
            if let Some(g) = batch
                .column_by_name("seq_gap")
                .and_then(|c| c.as_any().downcast_ref::<UInt32Array>())
            {
                report.seq_gap_total += (0..rows).map(|i| u64::from(g.value(i))).sum::<u64>();
            }
        }
    }
    Ok((report, inprogress))
}

/// Sum `count` for `kind == "drop"` rows across the events parts.
fn scan_events(dir: &Path) -> std::result::Result<(u64, u64), String> {
    let mut drops = 0u64;
    let mut inprogress = 0u64;
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok((0, 0)),
    };
    let mut parts: Vec<PathBuf> = Vec::new();
    for e in entries.flatten() {
        let fname = e.file_name().to_string_lossy().to_string();
        if is_inprogress(&fname) {
            inprogress += 1;
        } else if is_sealed_part(&fname) {
            parts.push(e.path());
        }
    }
    for p in parts {
        let f = std::fs::File::open(&p).map_err(|e| format!("open {}: {e}", p.display()))?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(f)
            .map_err(|e| format!("bad footer in {}: {e}", p.display()))?
            .build()
            .map_err(|e| format!("{}: {e}", p.display()))?;
        for batch in reader {
            let batch = batch.map_err(|e| format!("{}: {e}", p.display()))?;
            let kind = batch.column_by_name("kind").and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let count = batch.column_by_name("count").and_then(|c| c.as_any().downcast_ref::<UInt64Array>());
            // touch stream_id/shed_stage columns to keep the schema honest
            let _ = batch.column_by_name("stream_id").and_then(|c| c.as_any().downcast_ref::<UInt8Array>());
            if let (Some(kind), Some(count)) = (kind, count) {
                for i in 0..batch.num_rows() {
                    if kind.value(i) == "drop" {
                        drops += count.value(i);
                    }
                }
            }
        }
    }
    Ok((drops, inprogress))
}

/// Public entry for `log-inspect --raw`: frame count + torn-tail flag for a
/// standalone `raw_mavlink.bin`.
pub fn raw_summary(path: &Path) -> (u64, bool) {
    scan_raw(path)
}

/// Count length-prefixed frames in raw_mavlink.bin; report a torn trailing
/// record (declared length runs past end-of-file).
fn scan_raw(path: &Path) -> (u64, bool) {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(_) => return (0, false),
    };
    let mut off = 0usize;
    let mut frames = 0u64;
    let mut torn = false;
    while off + 4 <= data.len() {
        let len = u32::from_le_bytes(data[off..off + 4].try_into().unwrap()) as usize;
        if off + 4 + len > data.len() {
            torn = true; // partial final frame
            break;
        }
        off += 4 + len;
        frames += 1;
    }
    if off < data.len() && !torn {
        torn = true; // trailing bytes that are not a full length prefix
    }
    (frames, torn)
}
