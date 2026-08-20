# cc-mission-log

The crash-safe, replayable mission dataset writer. Consumes the
`TelemetryEvent` broadcast from `cc-ingest` and streams it to disk as
per-stream Parquet files, with a disk-shedding ladder for low-space
conditions and an atomically-maintained manifest. Also owns the reader used
by `log-inspect` and `cc-replay`.

## Scope

| In this crate | Not in this crate |
|---|---|
| Writing telemetry to durable, crash-safe Parquet | Producing `TelemetryEvent` — `cc-ingest` |
| Mission/segment lifecycle, resume-on-restart | Reading a *recorded* mission for health-detection replay — `cc-replay` (built on this crate's reader) |
| Disk-shedding ladder | Configuration itself — `cc-config` (this crate consumes a `cc_config::Config`) |
| Manifest + param snapshot + operational event log | |
| Dataset verification (`inspect_mission`) | |

## Crash-safety model (the crux)

Each stream is a directory of numbered Parquet **part files**. A flush:

1. Writes one complete `RecordBatch` to `NNNNNN.parquet.inprogress`, one row
   group per part (`WriterProperties::set_max_row_group_row_count` capped to
   the batch length).
2. `ArrowWriter::into_inner()` finalizes the file (writes the Parquet
   footer) and hands back the underlying `File`.
3. `fsync(file)`.
4. Atomic same-directory `rename` to `NNNNNN.parquet`.
5. `fsync(dir)` — the step most implementations forget, and the one that
   makes the *rename itself* durable.

**Ordering is the whole point**: fsync(file) → rename → fsync(dir). Reversing
any two would let a crash expose either a rename pointing at unsynced bytes,
or durable bytes under a name a crash could still lose. This exact ordering
is asserted by a test using a `RecordingSyncer` fake.

After a `kill -9` or power loss at any instant:

- Every `NNNNNN.parquet` present is a **standard, footer-complete file**,
  readable by any stock Parquet reader — no recovery code needed.
- At most one `.inprogress` file (the currently-open part) is lost per
  stream, bounded by `flush_rows`/`flush_secs`.
- The operational event log (`events/`) is part-rotated the **same way** as
  telemetry — a single ever-growing `events.parquet` would have no footer
  after a crash, losing exactly the drop/shed forensics that matter most
  right when a crash happens.

## On-disk layout

```text
<mission_root>/
├── cc_boot_seq                          # persisted monotonic counter
├── mission_seq                          # persisted monotonic counter
└── mission_000042/
    ├── manifest.json                    # provenance + per-segment/stream rollup
    ├── px4_params_snapshot.json         # PX4 parameter snapshot at handshake
    └── segment_00/
        ├── state/000000.parquet ...     # one dir per StreamId
        ├── imu/000000.parquet ...
        ├── power/ gps/ estimator/ actuator/ event/ safety_status/
        ├── events/000000.parquet ...    # operational log: drops, shed transitions, lifecycle
        └── raw_mavlink.bin              # length-prefixed ground-truth wire capture
```

A mission directory is `mission_<mission_id:06>`; a segment directory is
`segment_<index:02>`. `mission_id` and `cc_boot_id` are minted from the two
persisted counters (`ids::mint_counter`) — read-increment-write, durable, and
fail-open (a missing or corrupt counter file restarts at 1 rather than
blocking boot).

## Segmentation

A **mission** is the whole recording for one vehicle across possibly many
companiond restarts. A **segment** is one `(mission_id, cc_boot_id,
px4_boot_id)` identity — segments split on:

- companiond restart (new `cc_boot_id` — a fresh `Mission::open` opens a new
  segment)
- PX4 reboot (`Mission::on_boot_change` detects a new `px4_boot_id`, close
  reason `px4_reboot`)
- a size or time rotation cap (`seg_cap_bytes`/`seg_cap_secs`, close reason
  `rotation_cap`)

Splitting bounds both single-file loss and the blast radius of a replay.

### Resume-on-restart

`Mission::open` scans `mission_root` for an **incomplete** mission
(`manifest.complete == false`) belonging to the same `vehicle_id`. If found,
it resumes that `mission_id` at a fresh segment rather than silently minting
a new mission — and retroactively finalizes whatever segment the crashed
process left open, by recomputing that segment's stream stats straight from
the sealed part files on disk and stamping `close_reason = "cc_restart"`, so
a cleanly-resumed mission still reads as `Clean`.

## Public API

### `mission::Mission` — the lifecycle owner

```rust
pub enum OpenError { BelowFloor { free: u64, floor: u64 }, Io(Error) }

impl Mission {
    pub fn open(cfg: Config, clock: Arc<dyn Clock>, space: Arc<dyn SpaceProbe>,
                syncer: Arc<dyn Syncer>, health: Arc<LogHealth>, sw_version: String,
                px4_boot_id: u32, warn: impl FnMut(&str))
        -> Result<Mission, OpenError>;

    pub fn mission_dir(&self) -> &Path;
    pub fn mission_id(&self) -> u32;
    pub fn cc_boot_id(&self) -> u32;

    pub fn on_event(&mut self, ev: &cc_ingest::TelemetryEvent);
    pub fn on_raw(&mut self, frame: &[u8]);
    pub fn note_lag(&self, n: u64);
    pub fn on_boot_change(&mut self, new_px4_boot_id: u32) -> Result<()>;
    pub fn tick(&mut self) -> Result<()>;      // shed-ladder refresh, time-cap seals, rotation
    pub fn finalize(self) -> Result<()>;       // clean shutdown: seal + mark manifest complete
}
```

`open` refuses to start below the configured disk floor
(`OpenError::BelowFloor`) — a mission storage precondition checked once, up
front, distinct from the shedding ladder that governs behavior *after* a
mission has already opened.

### `task` — the single disk-touching async task

```rust
pub fn spawn(
    mission: Mission,
    events_rx: broadcast::Receiver<TelemetryEvent>,
    raw_rx: mpsc::Receiver<Vec<u8>>,
    boot_rx: watch::Receiver<u32>,
    tick_period: Duration,
    shutdown: oneshot::Receiver<()>,
) -> tokio::task::JoinHandle<()>;
```

The **only** writer of the mission dataset, and a **lossy** subscriber of the
telemetry broadcast: a `broadcast::error::RecvError::Lagged(n)` (slow disk)
calls `Mission::note_lag`, never back-pressures the RX path. A `biased`
`select!` gives clean shutdown priority over draining more events. On exit
(shutdown or channel close) it calls `Mission::finalize`.

### `env` — the three injected seams

```rust
pub trait Clock: Send + Sync + 'static {
    fn mono_ns(&self) -> i64;      // strictly non-decreasing, arbitrary origin
    fn wall_unix_ns(&self) -> i64; // may jump; never used for interval decisions
}
pub trait SpaceProbe: Send + Sync + 'static {
    fn free_bytes(&self, dir: &Path) -> io::Result<u64>;
}
pub trait Syncer: Send + Sync + 'static {
    fn sync_file(&self, f: &File) -> io::Result<()>;
    fn sync_dir(&self, dir: &Path) -> io::Result<()>;
}
```

Real implementations: `SystemClock`, `StatvfsProbe` (via `rustix::fs::statvfs`,
`f_bavail × f_frsize`), `RealSyncer` (`File::sync_all`, and directory fsync
via opening the directory read-only and syncing it — POSIX's way of making a
rename durable). `RealEnv` bundles all three for production wiring.

Fakes (always compiled for this crate's own tests; exported to other crates
behind the `test-seams` feature — used by companiond's tests and the Phase 5
harness for deterministic crash/disk-full scenarios):

- `FakeClock` — both clocks advanced explicitly by the test.
- `FakeSpace` — a scripted sequence of free-byte readings (each read advances
  to the next value; the last value sticks), so the whole shedding ladder can
  be driven without a real small volume.
- `NoopSyncer` — skips all fsyncs, so a `Segment` dropped without finalizing
  is byte-identical to a real `kill -9` between seals (this is *how* the
  crash tests simulate a crash — not a separate code path, the same code with
  durability skipped).
- `RecordingSyncer` — records the ordered sequence of sync operations so a
  test can assert the file-before-rename-before-dir ordering directly.

### `shed::ShedLadder` — the disk-pressure state machine

```rust
pub enum ShedStage { Normal = 0, ShedRaw = 1, ShedBf = 2, ShedCrit = 3 }

impl ShedLadder {
    pub fn new(disk: cc_config::Disk) -> Self;
    pub fn stage(&self) -> ShedStage;
    pub fn update(&mut self, free_bytes: u64) -> ShedStage;
    pub fn raw_allowed(&self) -> bool;
    pub fn stream_allowed(&self, s: StreamId) -> bool;
}
pub fn stage_allows(stage: ShedStage, s: StreamId) -> bool; // pure, also used directly
```

Shed order (never State/Event/SafetyStatus, at any stage): **raw capture
first**, then IMU + Actuator (`ShedBf`), then also Power/GPS/Estimator at the
deepest stage (`ShedCrit`). At `ShedCrit` the system still writes State +
Event + SafetyStatus rows and the operational drop log — it never silently
goes fully dark.

**Escalation** (free space falling) is immediate and can skip stages in one
reading (a sudden crater goes straight to `ShedCrit`). **De-escalation**
(free space rising) is one stage per `update()` call and gated on a
*resume* threshold strictly above the shed-low threshold — this hysteresis
is what stops the ladder chattering back and forth right at a boundary.

### `writer::StreamWriter` — per-stream part accumulation

Accumulates rows in a `RowBuf` and seals a part when either `flush_rows` is
reached (checked on every `push`) or `flush_secs` elapses with the buffer
non-empty (checked on `tick`, driven by the log task's independent ticker —
this is what lets a silent stream still seal its buffered rows on a time
cap instead of holding them hostage until the next segment rotation).

```rust
pub struct StreamStats {
    pub sealed_parts: u64, pub rows: u64, pub bytes: u64,
    pub first_cc_ns: Option<i64>, pub last_cc_ns: Option<i64>,
    pub seq_gap_total: u64, pub dropped: u64,
}
```

`StreamStats` mirrors into the manifest; `log-inspect` independently
recomputes the same numbers from the part footers and flags any mismatch. A
Parquet/IO failure while sealing (e.g. `ENOSPC` despite the shedding floor)
drops that batch and counts it — never fatal; a full disk must degrade, not
crash the process.

### `batch::RowBuf` — row accumulation → Arrow

Rows land in plain typed `Vec`s (not live Arrow array builders), so the hot
path is a trivial push and memory is bounded by `flush_rows`; the Arrow
arrays are materialized once, at `finish()`. `SegmentIdentity` (`vehicle_id`,
`mission_id`, `cc_boot_id`, `px4_boot_id`) is stamped as a constant column on
every row — segment-constant values that Parquet's RLE/dictionary encoding
collapses to almost nothing, so each lone part file still carries its full
join key independent of any other file.

### `schema` — the single source of truth for on-disk column layout

Both the writer (`batch`) and the reader (`inspect`) call into this module,
so a schema change is a compile-time event, never silent runtime drift.
Every stream schema is the shared 12-column identity envelope followed by
that stream's typed payload:

```rust
pub fn envelope_fields() -> Vec<Field>;
// vehicle_id, mission_id, px4_boot_id, cc_boot_id, stream_id (all non-null)
// sequence (nullable — SafetyStatus has none)
// fc_timestamp_us, cc_receive_time_ns, seq_gap (non-null)
// age_ns (nullable — null means the timesync offset was unknown at receive time)
// age_locked (disambiguates a genuine 0 ns age from "unknown")
// schema_version
```

Fixed wire arrays (`q[4]`, `accel[3]`, `actuator_output[8]`, …) are stored as
Arrow `FixedSizeList<Float32, N>` — the width is enforced by the type, not
by convention.

### `manifest::Manifest` — provenance + rollup, written atomically

```rust
pub struct Manifest {
    pub manifest_version: u32, pub complete: bool,
    pub vehicle_id: u32, pub mission_id: u32, pub cc_sw_version: String,
    pub dialect_hash: String, pub dialect_sha256: String, pub schema_version: u8,
    pub created_wall_unix_ns: i64, pub params: Option<ParamsSummary>,
    pub segments: Vec<SegmentEntry>,
}
impl Manifest {
    pub fn new(vehicle_id: u32, mission_id: u32, cc_sw_version: String, created_wall_unix_ns: i64) -> Self;
    pub fn write_atomic(&self, mission_dir: &Path, syncer: &Arc<dyn Syncer>) -> Result<()>;
    pub fn read(mission_dir: &Path) -> Result<Manifest>;
}
```

Written **first** at mission open with `complete = false`, rewritten
atomically (temp → fsync → rename → dir-fsync, via `ids::write_atomic`) at
every segment close and at clean mission end (`complete = true`). Stamps
`dialect_hash`/`dialect_sha256`/`schema_version` from `cc-protocol`'s
build-time constants — a dataset can always be matched to the exact protocol
version that wrote it. **The manifest is advisory, not authoritative**:
`log-inspect`/`inspect_mission` recomputes the real counts from the part
footers and reports any divergence as recoverable (`Dirty`), so a
crash-staled manifest never blocks reading otherwise-valid data.

Close reasons: `CLOSE_CLEAN`, `CLOSE_CC_RESTART`, `CLOSE_PX4_REBOOT`,
`CLOSE_ROTATION_CAP`.

### `events::EventLog` — the operational log

Records `EventRow { cc_receive_time_ns, kind, stream_id, reason, shed_stage,
free_bytes, count }` for lifecycle markers (`"open"`), drops (`"drop"`, one
row per drop event at low-volume deep-shed stages), and shed-stage
transitions (`"shed"`). Uses a smaller flush-row cap (clamped to ≤ 256) than
telemetry streams, since events are low-volume and a crash should lose as
little forensic data as possible. Sealed the identical way telemetry parts
are (`part::seal_part`).

### `raw::RawCapture` — the ground-truth wire tap

`raw_mavlink.bin`: repeated `[u32 LE length][frame bytes]`. These are the
**exact bytes off the link, tapped before decode** (`cc-link`'s
`spawn_with_raw_tap`), so raw capture is an independent check on the
decoder — it must never depend on decoder correctness to be useful. Buffered
in memory; durability comes from periodic `flush()` (called on the segment's
tick), which bounds how much of the tail can be lost to a crash. A torn
trailing record after `kill -9` (a declared length that runs past
end-of-file) is expected and detectable, not an error.

### `params::ParamSnapshot` — the PX4 parameter capture

```rust
pub struct ParamSnapshot {
    pub captured_wall_unix_ns: i64, pub px4_boot_id: u32, pub mission_id: u32,
    pub mode: String, pub complete: bool,
    pub expected_count: u32, pub received_count: u32, pub timed_out: bool,
    pub params: Vec<ParamEntry>,
}
impl ParamSnapshot {
    pub fn stub(px4_boot_id: u32, mission_id: u32, wall_ns: i64) -> Self;
    pub fn write_atomic(&self, mission_dir: &Path, syncer: &Arc<dyn Syncer>) -> Result<()>;
    pub fn summary(&self) -> manifest::ParamsSummary;
}
```

Completeness is first-class: `expected_count`/`received_count`/`timed_out`
make a partial parameter read a *deterministic representation* ("received
812/840") instead of a silent stub. `stub()` mode writes a fixed, small
placeholder set so crash/disk-full test harnesses never depend on live FC
parameter-request timing.

### `inspect` — dataset verification (`log-inspect`'s engine)

```rust
pub enum Verdict { Clean, Dirty(Vec<String>), Corrupt(Vec<String>) }
pub struct Report { pub mission_id: u32, pub vehicle_id: u32, pub complete: bool,
                     pub dialect_hash_ok: bool, pub schema_version_ok: bool,
                     pub segments: Vec<SegReport>, pub verdict: Verdict, /* .. */ }
pub fn inspect_mission(mission_dir: &Path) -> Report;
```

Recomputes authoritative per-stream row counts, time ranges, and gap totals
directly from Parquet part footers rather than trusting the manifest.
Three-state verdict:

- **`Clean`** (exit 0) — complete mission, dialect/schema hash match, every
  part footer valid, no leftover `.inprogress` files, rollup reconciles,
  zero drops.
- **`Dirty`** (exit 1) — recoverable: a killed-but-intact dataset (the
  headline crash-test success state), a stale manifest, disk-pressure
  drops, a torn raw tail. Usable with bounded, known loss.
- **`Corrupt`** (exit 2) — unreadable or wrong-binary: missing/unparseable
  manifest, dialect-hash mismatch, or a sealed part with a broken footer.

### `ids` — persisted monotonic counters

```rust
pub fn mint_counter(path: &Path, syncer: &Arc<dyn Syncer>, warn: impl FnMut(&str)) -> u64;
pub fn write_atomic(path: &Path, bytes: &[u8], syncer: &Arc<dyn Syncer>) -> std::io::Result<()>;
```

Read-increment-write, durable (same temp→fsync→rename→dir-fsync pattern as
everything else in this crate). Monotonic + persisted means every companiond
restart yields a strictly greater `cc_boot_id`, so segment ordering is total
and cross-reboot ID collisions are impossible. A missing or corrupt counter
file **fails open**: restart at 1 and carry on (a warning is raised, but
identity, once degraded, never blocks boot).

### `health::LogHealth` — live status (atomics, `Arc`-shared)

```rust
pub struct LogHealthSnapshot {
    pub shed_stage: u8, pub warn: bool, pub dropped: [u64; 8],
    pub raw_dropped: u64, pub write_errors: u64, pub parts_sealed: u64,
    pub last_free_bytes: u64, pub lagged: u64,
}
impl LogHealthSnapshot {
    pub fn stage_name(&self) -> &'static str;
    pub fn total_dropped(&self) -> u64;
}
```

Read by companiond's status JSON today; `cc-health-tx` folds `warn` into
`CC_HEALTH_REPORT.health_flags` (a "companion log degraded" bit) from the
same `Arc` in Phase 6. `warn` latches `true` on any drop, write error, or
non-`Normal` shed stage — it does not self-clear.

## Errors

```rust
pub enum Error { Io(io::Error), Parquet(ParquetError), Arrow(ArrowError), Json(serde_json::Error), Corrupt(String) }
pub type Result<T> = std::result::Result<T, Error>;
```

## Testing

`tests_lifecycle.rs` — deterministic end-to-end tests: a whole mission
written through `Mission`, read back by `inspect_mission`, with a "crash"
modeled as **dropping the `Segment` without finalizing** (byte-identical to
a real `kill -9` between seals when `NoopSyncer` is in use, and host-runnable
in milliseconds):

- `clean_mission_is_inspect_clean` — a normally-finalized mission verifies
  `Clean`.
- `crash_drop_leaves_sealed_parts_and_dirty_verdict` — a mid-write crash
  leaves every already-sealed part readable and verifies `Dirty`, never
  `Corrupt`.
- `restart_resumes_same_mission_id_new_segment` — reopening after a crash
  resumes the same `mission_id` at a new segment, retroactively finalizing
  the abandoned one.
- `disk_full_sheds_in_order_and_never_state` — a scripted `FakeSpace`
  sequence drives the ladder through every stage; State/Event/SafetyStatus
  are never dropped at any stage.
- `stray_inprogress_file_reads_dirty` — a leftover `.inprogress` file (an
  expected crash artifact) is recognized and reported, not mistaken for
  corruption.
- `boot_change_rotates_into_new_segment` — a PX4 boot-ID change closes the
  current segment (`px4_reboot`) and opens a fresh one.

Each module additionally carries focused unit tests against the injected
seams: `part.rs` proves the fsync/rename ordering and that a sealed part
round-trips through a stock Parquet reader; `shed.rs` proves the full
ladder walk (including hysteresis and stage-skipping) and the exact
per-stage stream-allow set; `writer.rs`/`events.rs` prove row-cap and
independent time-cap sealing; `manifest.rs`/`params.rs`/`ids.rs` prove
atomic round-tripping and fail-open corruption handling; `raw.rs` proves
length-prefixed frames round-trip byte-for-byte.

## Dependencies

`cc-protocol` (dialect-hash/schema-version provenance stamps), `cc-ingest`
(`TelemetryEvent`, `StreamId`), `cc-config` (`Config`, `Disk`,
`Compression`), `arrow` + `parquet` (columnar storage), `serde` +
`serde_json` (manifest/param-snapshot/JSON), `rustix` (`statvfs`), `tokio`
(the log task). Dev-only: `tokio`'s `test-util` feature.
