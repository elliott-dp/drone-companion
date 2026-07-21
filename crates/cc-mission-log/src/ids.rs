//! Persisted monotonic identity counters (`cc_boot_seq`, `mission_seq`).
//!
//! Each mint reads the current value, writes `value+1` durably
//! (temp → fsync → rename → dir-fsync) and returns it. Monotonic + persisted
//! means every restart yields a strictly greater id, so segment ordering is
//! total and cross-reboot collisions are impossible. A missing or corrupt
//! counter file fails **open**: start at 1 and carry on (identity degraded,
//! never a blocked boot).

use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use crate::env::Syncer;

/// Read-increment-write the counter at `path`, returning the new value.
/// `warn` is invoked (once) if the existing file was missing or unparseable.
pub fn mint_counter(
    path: &Path,
    syncer: &Arc<dyn Syncer>,
    mut warn: impl FnMut(&str),
) -> u64 {
    let current = match std::fs::read_to_string(path) {
        Ok(s) => match s.trim().parse::<u64>() {
            Ok(v) => v,
            Err(_) => {
                warn(&format!("counter {} is corrupt ({s:?}); restarting at 1", path.display()));
                0
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
        Err(e) => {
            warn(&format!("counter {} unreadable ({e}); restarting at 1", path.display()));
            0
        }
    };
    let next = current.saturating_add(1);

    if let Err(e) = write_atomic(path, next.to_string().as_bytes(), syncer) {
        warn(&format!("counter {} could not be persisted ({e}); id may repeat", path.display()));
    }
    next
}

/// Write `bytes` to `path` atomically and durably.
pub fn write_atomic(path: &Path, bytes: &[u8], syncer: &Arc<dyn Syncer>) -> std::io::Result<()> {
    let tmp = tmp_sibling(path);
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        syncer.sync_file(&f)?;
    }
    std::fs::rename(&tmp, path)?;
    if let Some(parent) = path.parent() {
        syncer.sync_dir(parent)?;
    }
    Ok(())
}

fn tmp_sibling(path: &Path) -> std::path::PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".tmp");
    std::path::PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::NoopSyncer;

    fn tmpdir(tag: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("ccml-ids-{}-{tag}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn mint_increments_and_persists() {
        let dir = tmpdir("inc");
        let path = dir.join("cc_boot_seq");
        let s: Arc<dyn Syncer> = Arc::new(NoopSyncer);
        assert_eq!(mint_counter(&path, &s, |_| {}), 1);
        assert_eq!(mint_counter(&path, &s, |_| {}), 2);
        assert_eq!(mint_counter(&path, &s, |_| {}), 3);
        // survives a "restart" (re-reads the file)
        assert_eq!(std::fs::read_to_string(&path).unwrap().trim(), "3");
    }

    #[test]
    fn corrupt_counter_fails_open_at_one_and_warns() {
        let dir = tmpdir("corrupt");
        let path = dir.join("mission_seq");
        std::fs::write(&path, b"garbage").unwrap();
        let s: Arc<dyn Syncer> = Arc::new(NoopSyncer);
        let mut warned = false;
        assert_eq!(mint_counter(&path, &s, |_| warned = true), 1);
        assert!(warned, "corruption must warn");
    }

    #[test]
    fn no_tmp_file_left_behind() {
        let dir = tmpdir("tmp");
        let path = dir.join("cc_boot_seq");
        let s: Arc<dyn Syncer> = Arc::new(NoopSyncer);
        mint_counter(&path, &s, |_| {});
        let tmps: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(tmps.is_empty());
    }
}
