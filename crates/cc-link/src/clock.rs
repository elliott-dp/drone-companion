//! Process-monotonic clock shared by all companion crates.
//!
//! `cc_receive_time_ns` in the identity envelope (spec §3.4) is "CC
//! CLOCK_MONOTONIC at frame receipt"; every Phase 4 crate stamps against
//! this one source so ages and offsets are always comparable. The epoch is
//! process start (first use) — absolute wall time is deliberately not used
//! (spec §5.4: the CC monotonic clock pairs with UTC only in the mission
//! log, Phase 5).

use std::sync::OnceLock;
use std::time::Instant;

static EPOCH: OnceLock<Instant> = OnceLock::new();

/// Nanoseconds since process epoch. Monotonic, never goes backwards.
pub fn now_ns() -> i64 {
    let epoch = EPOCH.get_or_init(Instant::now);
    epoch.elapsed().as_nanos() as i64
}
