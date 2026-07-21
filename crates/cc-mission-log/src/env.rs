//! Environment seams: the three traits that let every disk-durability and
//! disk-pressure behaviour be driven deterministically in a host unit test,
//! with real OS-backed implementations in production.
//!
//! * [`Clock`] — monotonic + wall time (a `FakeClock` makes rotation-by-time
//!   and timestamps reproducible).
//! * [`SpaceProbe`] — free bytes on a filesystem (a `FakeSpace` scripts the
//!   whole shedding ladder without a real small volume).
//! * [`Syncer`] — `fsync(file)` and `fsync(dir)` (a `NoopSyncer` lets the
//!   crash tests run in milliseconds, and a recording fake asserts the
//!   file-before-rename-before-dir ordering that makes a rename durable).

use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::Arc;

/// Monotonic clock (for rotation intervals) and wall clock (for the
/// human-facing `*_wall_unix_ns` fields written into the manifest).
pub trait Clock: Send + Sync + 'static {
    /// Strictly non-decreasing nanoseconds from an arbitrary origin.
    fn mono_ns(&self) -> i64;
    /// Wall-clock nanoseconds since the Unix epoch (may jump; never used for
    /// interval decisions).
    fn wall_unix_ns(&self) -> i64;
}

/// Free space available to an unprivileged writer under a directory.
pub trait SpaceProbe: Send + Sync + 'static {
    fn free_bytes(&self, dir: &Path) -> io::Result<u64>;
}

/// Durable-write primitives. `sync_file` forces a file's bytes to stable
/// storage; `sync_dir` forces a directory entry (i.e. a rename) durable — the
/// step most implementations forget.
pub trait Syncer: Send + Sync + 'static {
    fn sync_file(&self, f: &File) -> io::Result<()>;
    fn sync_dir(&self, dir: &Path) -> io::Result<()>;
}

// --- real implementations --------------------------------------------------

/// `std::time`-backed clock.
#[derive(Debug, Default, Clone)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn mono_ns(&self) -> i64 {
        use std::time::Instant;
        use std::sync::OnceLock;
        static ORIGIN: OnceLock<Instant> = OnceLock::new();
        let origin = *ORIGIN.get_or_init(Instant::now);
        origin.elapsed().as_nanos() as i64
    }
    fn wall_unix_ns(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0)
    }
}

/// `statvfs`-backed free-space probe (unprivileged-available blocks).
#[derive(Debug, Default, Clone)]
pub struct StatvfsProbe;

impl SpaceProbe for StatvfsProbe {
    fn free_bytes(&self, dir: &Path) -> io::Result<u64> {
        let s = rustix::fs::statvfs(dir).map_err(io::Error::from)?;
        // f_bavail is counted in units of f_frsize (the fundamental block).
        Ok(s.f_bavail.saturating_mul(s.f_frsize))
    }
}

/// Real `fsync` for files and directories.
#[derive(Debug, Default, Clone)]
pub struct RealSyncer;

impl Syncer for RealSyncer {
    fn sync_file(&self, f: &File) -> io::Result<()> {
        f.sync_all()
    }
    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        // Opening a directory read-only and fsyncing it persists renames /
        // creations within it (POSIX durability of the directory inode).
        File::open(dir)?.sync_all()
    }
}

/// Convenience bundle of the three real seams for production wiring.
#[derive(Clone)]
pub struct RealEnv {
    pub clock: Arc<dyn Clock>,
    pub space: Arc<dyn SpaceProbe>,
    pub syncer: Arc<dyn Syncer>,
}

impl Default for RealEnv {
    fn default() -> Self {
        Self {
            clock: Arc::new(SystemClock),
            space: Arc::new(StatvfsProbe),
            syncer: Arc::new(RealSyncer),
        }
    }
}

// --- fakes (crate tests always; exported behind `test-seams`) --------------

#[cfg(any(test, feature = "test-seams"))]
mod fakes {
    use super::*;
    use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
    use std::sync::Mutex;

    /// Deterministic clock: `mono_ns` and `wall_unix_ns` are advanced by the
    /// test.
    #[derive(Debug, Default)]
    pub struct FakeClock {
        mono: AtomicI64,
        wall: AtomicI64,
    }

    impl FakeClock {
        pub fn new(mono_ns: i64, wall_unix_ns: i64) -> Self {
            Self { mono: AtomicI64::new(mono_ns), wall: AtomicI64::new(wall_unix_ns) }
        }
        /// Advance both clocks by the same delta.
        pub fn advance_ns(&self, d: i64) {
            self.mono.fetch_add(d, Ordering::SeqCst);
            self.wall.fetch_add(d, Ordering::SeqCst);
        }
    }

    impl Clock for FakeClock {
        fn mono_ns(&self) -> i64 {
            self.mono.load(Ordering::SeqCst)
        }
        fn wall_unix_ns(&self) -> i64 {
            self.wall.load(Ordering::SeqCst)
        }
    }

    /// Free-space that returns a scripted sequence: each read advances to the
    /// next value; the final value sticks.
    #[derive(Debug)]
    pub struct FakeSpace {
        seq: Vec<u64>,
        idx: AtomicUsize,
    }

    impl FakeSpace {
        pub fn new(seq: Vec<u64>) -> Self {
            assert!(!seq.is_empty(), "FakeSpace needs at least one value");
            Self { seq, idx: AtomicUsize::new(0) }
        }
        /// A fixed free-space value forever.
        pub fn fixed(bytes: u64) -> Self {
            Self::new(vec![bytes])
        }
    }

    impl SpaceProbe for FakeSpace {
        fn free_bytes(&self, _dir: &Path) -> io::Result<u64> {
            let i = self.idx.fetch_add(1, Ordering::SeqCst).min(self.seq.len() - 1);
            Ok(self.seq[i])
        }
    }

    /// No-op syncer for fast, deterministic tests (a dropped-without-finalize
    /// Segment then behaves byte-identically to a real `kill -9` between
    /// seals, since nothing was force-persisted differently).
    #[derive(Debug, Default)]
    pub struct NoopSyncer;

    impl Syncer for NoopSyncer {
        fn sync_file(&self, _f: &File) -> io::Result<()> {
            Ok(())
        }
        fn sync_dir(&self, _dir: &Path) -> io::Result<()> {
            Ok(())
        }
    }

    /// Records the ordered sequence of sync operations so a test can assert
    /// that a part's file is fsynced *before* the dir fsync that publishes it.
    #[derive(Debug, Default)]
    pub struct RecordingSyncer {
        pub ops: Mutex<Vec<String>>,
    }

    impl Syncer for RecordingSyncer {
        fn sync_file(&self, _f: &File) -> io::Result<()> {
            self.ops.lock().unwrap().push("file".into());
            Ok(())
        }
        fn sync_dir(&self, dir: &Path) -> io::Result<()> {
            self.ops.lock().unwrap().push(format!("dir:{}", dir.display()));
            Ok(())
        }
    }
}

#[cfg(any(test, feature = "test-seams"))]
pub use fakes::{FakeClock, FakeSpace, NoopSyncer, RecordingSyncer};
