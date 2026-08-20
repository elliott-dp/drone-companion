# cc-timesync

FC↔CC clock correlation via the standard MAVLink `TIMESYNC` exchange. The
companion's own clock (`cc-link::clock::now_ns`) is monotonic-but-arbitrary;
PX4's is microseconds-since-boot. This crate estimates the offset between the
two so any FC timestamp (`fc_timestamp_us` in every `CC_TELEMETRY_*` message)
can be converted into the companion's clock for age computation and log
correlation.

## Scope

| In this crate | Not in this crate |
|---|---|
| RTT-compensated offset estimation, outlier rejection, quality judgement | Sending/receiving the raw TIMESYNC frames — `cc-link` TX/RX, routed in by companiond's demux |
| Fast-lock/steady-state request cadence | Using the offset to age telemetry — `cc-ingest` |
| Reboot/regression detection and re-lock | | |

## The exchange

```text
CC  ──TIMESYNC{tc1:0, ts1:cc_mono_ns}──►  FC
CC  ◄──TIMESYNC{tc1:fc_ns, ts1:echoed cc_ns}──  FC

rtt       = now_cc − ts1
offset_ns = tc1 − (ts1 + rtt/2)        // fc_ns ≈ cc_ns + offset_ns
```

Requests go out at TX priority **P0** (safety traffic outranks data, spec
§4) — a starved timesync would eventually stop telemetry from being aged at
all.

## Public API

### `Filter` — pure offset estimator (`lib.rs`)

No clocks, no I/O; every input is passed in by the caller. This is what the
dev plan requires be unit-testable against synthetic jitter traces without a
runtime.

```rust
pub struct Filter { /* 32-sample rolling window */ }
impl Filter {
    pub fn new() -> Self;
    pub fn add_reply(&mut self, tc1_ns: i64, ts1_ns: i64, now_ns: i64) -> bool; // accepted?
    pub fn invalidate(&mut self);                                              // FC reboot / regression
    pub fn estimate(&self) -> Snapshot;
}
```

- **Window**: `WINDOW = 32` most recent `(offset, rtt)` samples.
- **Outlier rejection**: once the window has ≥ 8 samples
  (`REJECT_MIN_SAMPLES`), a reply is rejected if `rtt > p90(window) × 3/2`.
  A reply whose apparent RTT is negative (reply timestamped before the
  request — clock nonsense) is rejected outright regardless of window size.
- **Offset**: median of the window's per-sample offsets (not mean — resistant
  to asymmetric-path outliers that slip past the RTT filter).
- **Quality** (`Quality::{Locked, Degraded, Unlocked}`):
  - `Locked` — ≥ 8 samples **and** RTT jitter (p90 − p10) ≤ 20 ms
    (`LOCK_JITTER_NS`) **and** rejection rate < 30% over the recent outcome
    window.
  - `Degraded` — ≥ 4 samples but not meeting the `Locked` bar.
  - `Unlocked` — fewer than 4 samples, or an empty window.
- **`invalidate()`** clears the window, outcome history, and rejection
  counter — called on FC reboot detection (either an explicit `px4_boot_id`
  change observed by `cc-ingest`, or a raw FC-timestamp regression seen
  directly by the runner).

```rust
pub enum Quality { Locked, Degraded, Unlocked }

pub struct Snapshot {
    pub offset_ns: i64,   // fc_ns ≈ cc_ns + offset_ns
    pub rtt_ns: i64,      // median RTT of the current window
    pub quality: Quality,
    pub window_len: usize,
    pub rejected: u32,    // outlier rejections since the last invalidate()
}
impl Snapshot {
    pub const UNLOCKED: Snapshot;
    pub fn fc_us_to_cc_ns(&self, fc_us: u64) -> i64;
    pub fn cc_ns_to_fc_us(&self, cc_ns: i64) -> i64;
}
```

Conversions are only meaningful when `quality == Locked` — callers are
expected to gate on that themselves (`cc-ingest` flags ages
`AgeInfo::UnknownOffset` rather than fabricating an age otherwise, per spec
§5.5).

### `runner` — async half

```rust
pub struct Reply { pub tc1_ns: i64, pub ts1_ns: i64, pub rx_ns: i64 }
pub struct Runner { pub replies: mpsc::Sender<Reply>, pub snapshot: watch::Receiver<Snapshot> }

pub fn spawn(tx: cc_link::TxHandle, boot_id: watch::Receiver<u32>) -> Runner;
```

`spawn` starts one task that:

1. **Requests** on a two-phase cadence: 10 Hz for the first 5 s after start
   or any invalidation (`FAST_PERIOD`/`FAST_WINDOW` — fast-lock), then 1 Hz
   steady-state (`SLOW_PERIOD`). Each request is `TIMESYNC { tc1: 0, ts1:
   clock::now_ns() }`.
2. **Consumes replies** pushed by the caller (companiond's demux routes
   inbound `TIMESYNC` frames here) on the `replies` channel. A reply whose
   `tc1_ns` regresses by more than 1 s from the last observed `tc1_ns` is
   treated as an FC reboot the boot-ID watch hasn't caught yet:
   `filter.invalidate()` and the fast-lock timer restarts. Otherwise the
   reply is fed to `Filter::add_reply`, and the resulting estimate is
   published via `send_replace` (always notifies, so consumers can observe
   every reply's effect, not just changes).
3. **Watches `boot_id`**: any change (fed by `cc-ingest` from the State
   telemetry stream's `px4_boot_id` field) invalidates the filter, resets
   the reboot-detection baseline, and re-publishes `Snapshot::UNLOCKED`
   immediately — this is the primary reboot-detection path; the raw
   `tc1_ns` regression check above is a fallback for the window before
   `cc-ingest` has seen a fresh State message.

Consumers read `Runner.snapshot` (a `watch::Receiver<Snapshot>`) and **must
never re-derive an offset themselves** — the filter is the single source of
truth for FC↔CC time correlation across the whole companion process.

## Testing

`lib.rs`'s test module drives `Filter` directly with synthetic traces
(deterministic xorshift64* PRNG, no `rand` dependency — same discipline as
the Phase 1 fuzz suite) rather than a live runtime:

- `clean_trace_locks_with_exact_offset` — symmetric path delays yield the
  exact true offset once `Locked`.
- `asymmetric_jitter_stays_within_half_rtt_bound` — random asymmetric
  one-way delays still converge within a bounded error.
- `outlier_bursts_are_rejected_and_estimate_holds` — a burst of abnormally
  slow replies (RTT far past p90×1.5) is entirely rejected and leaves the
  estimate unmoved.
- `sustained_rejections_degrade_quality` — a high sustained rejection rate
  drops quality from `Locked` to `Degraded` even though enough samples
  remain in the window.
- `invalidate_returns_to_unlocked_then_relocks` — `invalidate()` resets to
  `Snapshot::UNLOCKED`, and a fresh trace with a *different* true offset
  (simulating an FC restart) relocks correctly.
- `high_jitter_never_reaches_locked` — wide RTT variance alone (no outliers
  extreme enough to be rejected) keeps quality below `Locked`.
- `conversions_round_trip` — `fc_us_to_cc_ns`/`cc_ns_to_fc_us` are exact
  inverses and match the documented sign convention.

## Dependencies

`cc-protocol` (TIMESYNC message type), `cc-link` (`TxHandle`, `Priority::P0`,
the shared monotonic clock), `tokio` (the runner task, channels).
