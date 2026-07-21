//! Live mission-log health, shared via `Arc` of atomics.
//!
//! companiond's status JSON reads it now; in Phase 6 `cc-health-tx` folds
//! `warn` into `CC_HEALTH_REPORT.health_flags` (companion-log-degraded bit)
//! from the same `Arc` — Phase 5 only surfaces it.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

use cc_ingest::StreamId;

use crate::shed::ShedStage;

/// Atomically-updated counters and flags for the mission-log subsystem.
#[derive(Debug, Default)]
pub struct LogHealth {
    shed_stage: AtomicU8,
    warn: AtomicBool,
    /// Per-telemetry-stream dropped-row totals (shed or write-error).
    dropped: [AtomicU64; 8],
    /// raw_mavlink.bin frames dropped (shed or back-pressure).
    raw_dropped: AtomicU64,
    /// Parquet/IO write failures (e.g. ENOSPC on a flush) — counted, never
    /// fatal (the writer drops the batch and continues).
    write_errors: AtomicU64,
    parts_sealed: AtomicU64,
    last_free_bytes: AtomicU64,
    /// Telemetry events the log task missed because it lagged the broadcast
    /// (slow disk) — the logger is a lossy subscriber, so RX is never blocked.
    lagged: AtomicU64,
}

impl LogHealth {
    pub fn set_stage(&self, stage: ShedStage) {
        self.shed_stage.store(stage.as_u8(), Ordering::Relaxed);
        // Any shedding at all raises the WARN flag (latched until cleared).
        if stage != ShedStage::Normal {
            self.warn.store(true, Ordering::Relaxed);
        }
    }

    pub fn set_free_bytes(&self, free: u64) {
        self.last_free_bytes.store(free, Ordering::Relaxed);
    }

    pub fn add_dropped(&self, stream: StreamId, n: u64) {
        self.dropped[stream as usize].fetch_add(n, Ordering::Relaxed);
        self.warn.store(true, Ordering::Relaxed);
    }

    pub fn add_raw_dropped(&self, n: u64) {
        self.raw_dropped.fetch_add(n, Ordering::Relaxed);
        if n > 0 {
            self.warn.store(true, Ordering::Relaxed);
        }
    }

    pub fn add_write_error(&self) {
        self.write_errors.fetch_add(1, Ordering::Relaxed);
        self.warn.store(true, Ordering::Relaxed);
    }

    pub fn add_parts_sealed(&self, n: u64) {
        self.parts_sealed.fetch_add(n, Ordering::Relaxed);
    }

    pub fn add_lagged(&self, n: u64) {
        self.lagged.fetch_add(n, Ordering::Relaxed);
    }

    /// Snapshot for the status JSON / manifest.
    pub fn snapshot(&self) -> LogHealthSnapshot {
        let mut dropped = [0u64; 8];
        for (i, d) in self.dropped.iter().enumerate() {
            dropped[i] = d.load(Ordering::Relaxed);
        }
        LogHealthSnapshot {
            shed_stage: self.shed_stage.load(Ordering::Relaxed),
            warn: self.warn.load(Ordering::Relaxed),
            dropped,
            raw_dropped: self.raw_dropped.load(Ordering::Relaxed),
            write_errors: self.write_errors.load(Ordering::Relaxed),
            parts_sealed: self.parts_sealed.load(Ordering::Relaxed),
            last_free_bytes: self.last_free_bytes.load(Ordering::Relaxed),
            lagged: self.lagged.load(Ordering::Relaxed),
        }
    }
}

/// A plain-data snapshot of [`LogHealth`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogHealthSnapshot {
    pub shed_stage: u8,
    pub warn: bool,
    pub dropped: [u64; 8],
    pub raw_dropped: u64,
    pub write_errors: u64,
    pub parts_sealed: u64,
    pub last_free_bytes: u64,
    pub lagged: u64,
}

impl LogHealthSnapshot {
    /// Human name for the current shed stage.
    pub fn stage_name(&self) -> &'static str {
        match self.shed_stage {
            1 => "SHED_RAW",
            2 => "SHED_BF",
            3 => "SHED_CRIT",
            _ => "NORMAL",
        }
    }

    /// Total dropped rows across every stream.
    pub fn total_dropped(&self) -> u64 {
        self.dropped.iter().sum::<u64>() + self.raw_dropped
    }
}
