# cc-ingest

Consumes decoded frames from `cc-link` and produces the single
`TelemetryEvent` broadcast stream every downstream consumer subscribes to
(`cc-mission-log`, `cc-ai-health`, companiond's status task). Per frame:
schema validation → stream classification → sequence-continuity accounting →
age computation against the timesync snapshot → broadcast. Also runs the
per-stream staleness watchdogs.

## Scope

| In this crate | Not in this crate |
|---|---|
| Schema-version gate on decoded frames | Source/range validation — already done in `cc-link`'s RX path (`cc-protocol::validate`) |
| Per-stream sequence-gap counting | Frame decoding — `cc-protocol::CcFrameDecoder` |
| Age computation (FC timestamp → CC clock) | The offset estimate itself — `cc-timesync` |
| Per-stream staleness watchdogs | Persistence — `cc-mission-log` |
| `px4_boot_id` change detection → sequence reset + `boot_tx` publish | Consuming the boot-id watch — `cc-timesync::runner` invalidates from it |

## Pipeline

```text
LinkFrame (mpsc, from cc-link)
        │
        ▼
  validate_schema  ──fail──►  bad_schema += 1, dropped
        │ ok
        ▼
  classify by MavMessage variant → StreamId
        │
        ▼
  sequence continuity (wrapping diff, gap counted once)
        │
        ▼
  age = now_cc − fc_us_to_cc_ns(fc_timestamp_us)   [only if timesync Locked]
        │        else AgeInfo::UnknownOffset — never fabricated
        ▼
  broadcast::Sender<TelemetryEvent>  (capacity 1024, lossy to slow subscribers)
```

A parallel 100 ms watchdog tick (`sweep_watchdogs`) walks every periodic
stream and emits `TelemetryEvent::StreamStale` once on the silent→stale
transition; a link-status change is forwarded as
`TelemetryEvent::LinkStatus`. All three sources (frame arrival, watchdog
tick, link-status change) are handled by one `tokio::select!` loop — there
is exactly one ingest task.

## Public API

### `StreamId` — the eight FC→CC streams

```rust
pub enum StreamId { State, Imu, Power, Gps, Estimator, Actuator, Event, SafetyStatus }
impl StreamId {
    pub const ALL: [StreamId; 8];
    pub fn name(self) -> &'static str;
    pub fn nominal_period_ns(self) -> Option<i64>; // None = event-driven, no watchdog
}
```

Nominal periods (spec §6; watchdog threshold = 4× this):
State 40 ms, IMU 20 ms, Power 100 ms, GPS 200 ms, Estimator 100 ms,
Actuator 60 ms. `Event` and `SafetyStatus` are event-driven and carry no
watchdog.

### `TelemetryEvent` — the single fan-out type

```rust
pub enum TelemetryEvent {
    State(CC_TELEMETRY_STATE_DATA, RxMeta),
    Imu(CC_TELEMETRY_IMU_DATA, RxMeta),
    Power(CC_TELEMETRY_POWER_DATA, RxMeta),
    Gps(CC_TELEMETRY_GPS_DATA, RxMeta),
    Actuator(CC_TELEMETRY_ACTUATOR_DATA, RxMeta),
    Estimator(CC_TELEMETRY_ESTIMATOR_DATA, RxMeta),
    SafetyStatus(CC_SAFETY_STATUS_DATA, RxMeta),
    Event(CC_EVENT_DATA, RxMeta),
    LinkStatus(cc_link::LinkStatus),
    StreamStale(StreamId),
}
```

Every payload variant carries the generated wire struct **plus** `RxMeta` —
the identity envelope travels with the payload rather than being looked up
separately (deviation from the original spec sketch, which showed bare
payloads).

```rust
pub struct RxMeta {
    pub cc_receive_time_ns: i64,
    pub seq_gap: u32,       // gap detected AT this message, 0 = contiguous
    pub age: AgeInfo,
}
pub enum AgeInfo {
    Locked { age_ns: i64 }, // now_cc − to_cc(fc_timestamp); only when timesync Locked
    UnknownOffset,          // never fabricated
}
```

### `IngestStats` — cumulative counters (atomics)

```rust
pub struct IngestStats {
    pub count: [AtomicU64; 8],
    pub gaps: [AtomicU64; 8],
    pub last_rx_ns: [AtomicI64; 8],
    pub stale: [AtomicBool; 8],
    pub bad_schema: AtomicU64,
    pub mission_id: AtomicU32,
}
impl IngestStats {
    pub fn stream_count(&self, s: StreamId) -> u64;
    pub fn stream_gaps(&self, s: StreamId) -> u64;
    pub fn stream_stale(&self, s: StreamId) -> bool;
    pub fn total_gaps(&self) -> u64;
}
```

Written by the ingest task, read by companiond's status task (or anything
else) without contending the hot path — plain atomics, `Ordering::Relaxed`.

### `spawn`

```rust
pub struct Ingest { pub events: broadcast::Sender<TelemetryEvent>, pub stats: Arc<IngestStats> }

pub fn spawn(
    frames: mpsc::Receiver<LinkFrame>,
    ts_snapshot: watch::Receiver<cc_timesync::Snapshot>,
    link_status: watch::Receiver<cc_link::LinkStatus>,
    boot_tx: watch::Sender<u32>,
) -> Ingest;
```

`boot_tx` is created by the **caller** (companiond), not by this crate —
`cc-timesync::runner::spawn` needs a receiver of it before `cc-ingest::spawn`
exists, so the sender has to originate outside both crates to avoid a
circular dependency.

## Design notes

- **Sequence continuity** uses a wrapping subtraction (`s.wrapping_sub(prev)`)
  so a `u32` sequence counter wrapping around is not misread as a massive
  gap: a difference greater than `u32::MAX / 2` is treated as a
  duplicate/regression (gap = 0) rather than a gap. `CC_EVENT` sources its
  sequence from PX4's native `u16` event counter, which therefore wraps far
  more often — the same wrapping-diff logic handles that transparently.
- **`CC_SAFETY_STATUS`** carries no per-stream sequence of its own (its
  `last_report_sequence` field is a report-ACK, not a stream sequence), so
  it is emitted with `seq: None` — never contributes to gap counting.
- **`px4_boot_id` change** (observed on `CC_TELEMETRY_STATE`) resets every
  per-stream sequence tracker to `None` (sequences restart on FC boot, spec
  §4.2) and publishes the new boot ID on `boot_tx` — `cc-timesync` watches
  this to invalidate its filter and re-enter fast-lock.
- **Age is never fabricated.** If the timesync snapshot's quality is not
  `Locked` at classification time, the event's age is `UnknownOffset`
  rather than a guessed or stale-offset-derived value (system invariant 7:
  missing data is missing).
- **Broadcast is lossy to slow subscribers** (`BROADCAST_DEPTH = 1024`): a
  subscriber that falls behind drops the oldest unread events rather than
  backpressuring the ingest task, which must never block the RX path (spec
  §5.2).
- **Staleness watchdog** (`WATCHDOG_TICK = 100 ms`, `STALE_FACTOR = 4`)
  tracks last-seen time on the **runtime clock** (`tokio::time::Instant`)
  rather than the process-monotonic `cc_link::clock` — this makes the
  watchdog correctly controllable under `tokio::time::pause()` in tests. A
  stream that has never been seen at all is treated as absent, not stale
  (no watchdog event fires for it). The stale flag only fires
  `TelemetryEvent::StreamStale` on the silent→stale transition (edge, not
  level) and clears silently when data resumes.

## Testing

`tests/ingest_behavior.rs` (uses `tokio::time::pause()` for deterministic
watchdog timing):

- `continuity_gaps_counted_once_and_attached` — a dropped sequence number
  produces exactly one gap count, attached to the next event's `RxMeta`.
- `boot_change_resets_sequences_and_publishes_watch` — a `px4_boot_id`
  change resets sequence tracking and is observable on `boot_tx`.
- `schema_mismatch_dropped_and_counted` — a frame with the wrong
  `schema_version` is dropped and counted in `bad_schema`, never broadcast.
- `age_flags_follow_timesync_quality` — events carry `AgeInfo::Locked` only
  when the timesync snapshot is `Locked`, `UnknownOffset` otherwise.
- `watchdog_fires_once_after_4x_nominal_silence` — `StreamStale` fires
  exactly once on the silent→stale transition, not repeatedly.

## Dependencies

`cc-protocol` (dialect types, schema validation), `cc-link` (`LinkFrame`,
`LinkStatus`, the shared clock), `cc-timesync` (`Snapshot`, `Quality`),
`tokio` (channels, the ingest task). Dev-only: `tokio`'s `test-util` feature
for paused-clock watchdog tests.
