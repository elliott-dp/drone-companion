//! `cc-mission-log` — the crash-safe, replayable mission dataset (spec §5.6,
//! dev-plan Phase 5.2).
//!
//! ## Crash-safety model (the crux)
//!
//! Each stream is a directory of numbered Parquet **part files**. A flush
//! writes one complete, footer-terminated part (`crate::part`), fsyncs it,
//! atomically renames it into place, and fsyncs the directory. After a
//! `kill -9` or power loss, every sealed `NNNNNN.parquet` is readable by a
//! stock Parquet reader with no recovery code; at most one open in-progress
//! part per stream is lost (bounded by `flush_rows` / `flush_secs`). The
//! operational log (`events/`) is part-rotated the same way, so drop/shed
//! accounting also survives a crash.
//!
//! ## Test seams
//!
//! [`env::Clock`], [`env::SpaceProbe`] and [`env::Syncer`] are injected, so
//! rotation-by-time, the disk-shedding ladder, and crash recovery are all
//! driven deterministically in host unit tests (a Segment dropped without
//! finalising is byte-identical to a real crash between seals).

pub mod batch;
pub mod env;
pub mod events;
pub mod health;
pub mod ids;
pub mod inspect;
pub mod manifest;
pub mod mission;
pub mod params;
pub mod part;
pub mod raw;
pub mod schema;
pub mod segment;
pub mod shed;
pub mod task;
pub mod writer;

#[cfg(test)]
mod tests_lifecycle;

pub use batch::SegmentIdentity;
pub use health::{LogHealth, LogHealthSnapshot};
pub use inspect::{inspect_mission, Report, Verdict};
pub use mission::{Mission, OpenError};
pub use params::ParamSnapshot;

/// Errors from the mission-log writer/reader.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Parquet(parquet::errors::ParquetError),
    Arrow(arrow::error::ArrowError),
    Json(serde_json::Error),
    /// A structural problem in an on-disk dataset (used by the reader).
    Corrupt(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Parquet(e) => write!(f, "parquet: {e}"),
            Error::Arrow(e) => write!(f, "arrow: {e}"),
            Error::Json(e) => write!(f, "json: {e}"),
            Error::Corrupt(m) => write!(f, "corrupt dataset: {m}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
impl From<parquet::errors::ParquetError> for Error {
    fn from(e: parquet::errors::ParquetError) -> Self {
        Error::Parquet(e)
    }
}
impl From<arrow::error::ArrowError> for Error {
    fn from(e: arrow::error::ArrowError) -> Self {
        Error::Arrow(e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
