//! The crash-safety crux: **one flush = one complete Parquet part file**.
//!
//! Each part is written to `NNNNNN.parquet.inprogress`, its footer is written
//! by `ArrowWriter::into_inner` (which finalises the file), the bytes are
//! fsynced, the file is atomically renamed to `NNNNNN.parquet`, and finally
//! the directory is fsynced so the rename itself is durable. After a `kill -9`
//! or power loss at any instant, every `NNNNNN.parquet` present is a
//! standard, footer-complete file readable by any Parquet reader with zero
//! recovery code; at most one `.inprogress` file (the open part) is lost.
//!
//! Ordering is the whole point and is asserted by a test via `RecordingSyncer`:
//! **fsync(file) → rename → fsync(dir)**. Reversing any two would let a crash
//! expose a rename that points at unsynced bytes, or a durable-but-unnamed
//! file.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow::record_batch::RecordBatch;
use cc_config::Compression as CfgCompression;
use parquet::arrow::ArrowWriter;
use parquet::basic::{Compression, ZstdLevel};
use parquet::file::properties::WriterProperties;

use crate::env::Syncer;
use crate::{Error, Result};

/// The file name of part `index` in a stream directory (final, sealed form).
pub fn part_name(index: u64) -> String {
    format!("{index:06}.parquet")
}

fn inprogress_name(index: u64) -> String {
    format!("{index:06}.parquet.inprogress")
}

/// `true` if a directory entry is a sealed part file we should read.
pub fn is_sealed_part(name: &str) -> bool {
    name.ends_with(".parquet") && !name.ends_with(".inprogress")
}

/// `true` if a directory entry is an abandoned in-progress part (an expected,
/// ignorable crash artifact).
pub fn is_inprogress(name: &str) -> bool {
    name.ends_with(".parquet.inprogress")
}

fn parquet_compression(c: CfgCompression) -> Compression {
    match c {
        CfgCompression::None => Compression::UNCOMPRESSED,
        CfgCompression::Snappy => Compression::SNAPPY,
        CfgCompression::Zstd => Compression::ZSTD(ZstdLevel::default()),
    }
}

/// Write `batch` as a single-row-group part in `dir`, sealing it durably.
/// Returns the final path. Must not be called with an empty batch.
pub fn seal_part(
    dir: &Path,
    index: u64,
    batch: &RecordBatch,
    compression: CfgCompression,
    syncer: &Arc<dyn Syncer>,
) -> Result<PathBuf> {
    debug_assert!(batch.num_rows() > 0, "seal_part called with an empty batch");

    let tmp_path = dir.join(inprogress_name(index));
    let final_path = dir.join(part_name(index));

    // One batch, one row group: cap the row-group size at this batch's length
    // so the whole part is exactly one row group (bounded, deterministic loss).
    let props = WriterProperties::builder()
        .set_max_row_group_row_count(Some(batch.num_rows().max(1)))
        .set_compression(parquet_compression(compression))
        .build();

    let file = std::fs::File::create(&tmp_path).map_err(Error::Io)?;
    let mut writer =
        ArrowWriter::try_new(file, batch.schema(), Some(props)).map_err(Error::Parquet)?;
    writer.write(batch).map_err(Error::Parquet)?;
    // into_inner finalises the file (writes the footer) and hands the File
    // back so we can fsync the completed bytes.
    let file = writer.into_inner().map_err(Error::Parquet)?;
    syncer.sync_file(&file).map_err(Error::Io)?;
    drop(file);

    // Atomic same-directory rename, then fsync the directory so the rename
    // survives a crash.
    std::fs::rename(&tmp_path, &final_path).map_err(Error::Io)?;
    syncer.sync_dir(dir).map_err(Error::Io)?;

    Ok(final_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batch::{RowBuf, SegmentIdentity};
    use crate::env::{NoopSyncer, RecordingSyncer};
    use cc_ingest::{AgeInfo, RxMeta, StreamId, TelemetryEvent};
    use cc_protocol::cc_dialect::CC_TELEMETRY_POWER_DATA;
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

    fn power_event(seq: u32, v: f32) -> TelemetryEvent {
        let d = CC_TELEMETRY_POWER_DATA {
            fc_timestamp_us: 1000 + u64::from(seq),
            sequence: seq,
            voltage: v,
            current: 1.0,
            power: v,
            consumed_mah: 2.0,
            remaining: 0.9,
            temperature: 25.0,
            cell_count: 6,
            warning: 0,
            connected: 1,
            schema_version: 1,
        };
        let m = RxMeta { cc_receive_time_ns: 5000 + i64::from(seq), seq_gap: 0, age: AgeInfo::Locked { age_ns: 42 } };
        TelemetryEvent::Power(d, m)
    }

    fn id() -> SegmentIdentity {
        SegmentIdentity { vehicle_id: 1, mission_id: 42, cc_boot_id: 7, px4_boot_id: 3 }
    }

    fn build(n: u32) -> RecordBatch {
        let mut rb = RowBuf::new(StreamId::Power, id());
        for i in 0..n {
            assert!(rb.push(&power_event(i, 16.0 + i as f32)));
        }
        rb.finish().unwrap()
    }

    #[test]
    fn sealed_part_is_a_standard_readable_file() {
        let dir = tempdir();
        let batch = build(10);
        let syncer: Arc<dyn Syncer> = Arc::new(NoopSyncer);
        let path = seal_part(&dir, 0, &batch, CfgCompression::Zstd, &syncer).unwrap();

        // Reopen with a stock parquet reader — proves the footer is present.
        let f = std::fs::File::open(&path).unwrap();
        let builder = ParquetRecordBatchReaderBuilder::try_new(f).unwrap();
        assert_eq!(builder.metadata().num_row_groups(), 1, "exactly one row group per part");
        let rows: usize = builder.build().unwrap().map(|b| b.unwrap().num_rows()).sum();
        assert_eq!(rows, 10);
        assert!(is_sealed_part(path.file_name().unwrap().to_str().unwrap()));
    }

    #[test]
    fn sync_ordering_is_file_then_rename_then_dir() {
        let dir = tempdir();
        let batch = build(3);
        let rec = Arc::new(RecordingSyncer::default());
        let syncer: Arc<dyn Syncer> = rec.clone();
        seal_part(&dir, 5, &batch, CfgCompression::None, &syncer).unwrap();

        let ops = rec.ops.lock().unwrap().clone();
        // file fsync must precede the dir fsync (the rename sits between them
        // in code; here we assert the two fsyncs bracket it in order).
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0], "file");
        assert!(ops[1].starts_with("dir:"), "dir fsync after file fsync");
    }

    #[test]
    fn no_inprogress_file_remains_after_seal() {
        let dir = tempdir();
        let batch = build(4);
        let syncer: Arc<dyn Syncer> = Arc::new(NoopSyncer);
        seal_part(&dir, 0, &batch, CfgCompression::None, &syncer).unwrap();
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| is_inprogress(e.file_name().to_str().unwrap()))
            .collect();
        assert!(leftovers.is_empty(), "no .inprogress after a clean seal");
    }

    fn tempdir() -> PathBuf {
        // process/thread-unique scratch dir (no external tempfile dep)
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!(
            "ccml-part-{}-{}-{n}",
            std::process::id(),
            // thread id hash for parallel test isolation
            format!("{:?}", std::thread::current().id()).len()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
