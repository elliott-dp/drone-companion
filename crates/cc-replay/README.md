# cc-replay

Reads a recorded `cc-mission-log` mission directory, reconstructs the exact
live receive order as a `TelemetryEvent` stream, and drives it through the
**same** `cc-ai-health::Runner` the live companion uses — reproducing the
identical finding timeline offline. This is the determinism *evidence*
engine and the false-positive *audit* engine for Phase 7.

Synchronous, no `tokio` — reads Parquet directly, then calls into
`cc-ai-health` as a library.

## Scope

| In this crate | Not in this crate |
|---|---|
| Reconstructing a mission dataset into a time-ordered event stream | Writing the mission dataset — `cc-mission-log` |
| Driving that stream through `cc-ai-health::Runner` | The health-detection algorithms themselves — `cc-ai-health` |
| Hashing/diffing/auditing finding timelines | The CLI around this crate — `apps/replay-mission` |

## Why this crate exists

`cc-ai-health`'s core determinism claim is that replaying a recorded flight
reproduces byte-identical findings, on any machine, regardless of host
speed. That claim is only meaningful if there's a way to actually replay a
*real, recorded* mission — not just feed a synthetic in-memory event slice
to the `Runner` directly (which is what `cc-ai-health`'s own unit tests do).
`cc-replay` is that path: mission directory in, `Runner` output out, with
the exact same Parquet-round-trip and merge-ordering the live system went
through when it originally wrote the data.

## Reading a mission (`reader::read_mission`)

```rust
pub enum ReplayError { Manifest(String), Io(String), Parquet(String), Schema(String) }
pub fn read_mission(mission_dir: &Path) -> Result<Vec<TelemetryEvent>, ReplayError>;
```

1. Read `manifest.json` to enumerate segments.
2. For each segment, for each of the **six periodic streams** the health
   algorithms actually consume (`State`, `Imu`, `Power`, `Gps`, `Estimator`,
   `Actuator`), read every sealed `NNNNNN.parquet` part in that stream's
   directory and decode each row back into its wire struct + `RxMeta`.
3. **K-way merge-sort every row across every stream and segment** by
   `(cc_receive_time_ns, stream_id, sequence)` — this is exactly the order
   live ingest produced them in, so the reconstructed stream is not just *a*
   valid ordering but *the* original one.

`Event` and `SafetyStatus` are **deliberately not decoded** — no
`HealthAlgorithm` reads them, and the `Runner` only uses their arrival to
bump `last_seen[stream]`, which no algorithm's freshness check consults.
Decoding them could not change any finding, so it's dead work, documented
as replay scope rather than a silent gap.

Column access uses a small macro (`col!`) that downcasts each Arrow column
by name and type, erroring as `ReplayError::Schema` on a mismatch — the
mirror image of `cc-mission-log::schema`'s writer-side column layout;
`FixedSizeListArray` columns (`q`, `accel`, `actuator_output`, …) are
unpacked back into their fixed `[f32; N]` wire representation via `fsl_row`.

## Public API

```rust
pub struct FindingRow {
    pub tick_ns: i64, pub severity: &'static str, pub action: &'static str,
    pub health_flags: u32, pub detail_code: u16,
    pub value: f32, pub limit: f32, pub confidence: u8,
}
pub struct Timeline { pub rows: Vec<FindingRow> }
impl Timeline {
    pub fn hash(&self) -> String;                             // canonical SHA-256
    pub fn findings(&self) -> impl Iterator<Item = &FindingRow>; // WARN | CRITICAL rows
}

pub fn replay_events(events: &[cc_ingest::TelemetryEvent]) -> Timeline;
pub fn run_mission(mission_dir: &Path) -> Result<Timeline, ReplayError>;
pub fn diff(a: &Timeline, b: &Timeline) -> Vec<String>;        // empty = identical

pub struct AuditStats {
    pub missions: usize, pub ticks: usize,
    pub warn_ticks: usize, pub critical_ticks: usize,
    pub by_detail: Vec<(u16, usize)>,                          // (detail_code, count), sorted
}
impl AuditStats {
    pub fn warn_rate(&self) -> f64;
}
pub fn audit(timelines: &[Timeline]) -> AuditStats;
```

`severity`/`action` are serialized as **strings** (`"ok"`, `"warn_only"`,
…), not raw enum discriminants — the timeline is meant to be read as JSON
by humans and tooling, not just re-parsed by this crate.

### Hashing (`Timeline::hash`)

Folds every row into a SHA-256 over the row count followed by each row's
**binary** representation — floats go in as `to_bits()`, not formatted
text, so the hash is bit-exact and immune to any float-formatting
ambiguity. This is the literal determinism token: two `run_mission` calls
(same data, any two machines) must produce the identical hash, or the
determinism guarantee is broken.

### Diffing (`diff`)

Row-by-row comparison, capped at 20 reported differences (with a truncation
marker) so a systematically-broken comparison doesn't dump an unbounded
wall of near-duplicate lines. A row-count mismatch is reported as its own
line before the per-row comparison.

### Auditing (`audit`)

Aggregates WARN/CRITICAL rates across one or more timelines — the
mechanism behind the Phase 7 exit criterion: **every algorithm's CRITICAL
stays advisory until a documented audit over recorded benign missions
(not just synthetic traces) shows ~zero false positives.** `by_detail`
histograms every non-OK finding by its `detail_code`, so a failing audit
immediately points at which specific detector and cause is responsible,
not just an aggregate rate.

## Determinism guarantee: inherited, not re-implemented

This crate does not re-derive determinism — it restores the exact live
event order (`read_mission`) and hands it to
`cc_ai_health::Runner::run_events`, the identical function `cc-ai-health`'s
own tests use. Same bytes in → same bytes out is a property of the
`Runner`/algorithm design (state folds in `on_event`, `evaluate` is a pure
read); `cc-replay`'s job is only to make sure the bytes going in are
*exactly* the ones that went in live.

## Testing

`lib.rs`:
- `replay_hash_is_stable_and_diff_clean` — replaying the same in-memory
  event slice twice yields identical hashes and an empty diff.
- `audit_of_benign_stream_has_zero_findings` — a stream that never arms any
  detector (no State/arm events at all) produces zero WARN/CRITICAL ticks.

`tests/roundtrip.rs` (the crate's proof against **real Parquet**, not just
in-memory events):
- `parquet_roundtrip_reproduces_in_memory_findings` — a synthetic benign
  mission is written to real `cc-mission-log` Parquet parts, read back
  through `read_mission`, and replayed; its hash is asserted equal to
  replaying the same events directly in memory (`replay_events`) — proving
  the Parquet write→read round-trip is lossless *for the purpose of
  findings*, not just structurally.
- `recorded_mission_replays_identically_twice` — reading and replaying the
  same on-disk mission directory twice yields the same hash both times.

## Dependencies

`cc-ai-health` (`Runner`, `default_registry`, `HealthConclusion`),
`cc-ingest` (`TelemetryEvent`, `StreamId`), `cc-protocol` (dialect types),
`cc-mission-log` (`Manifest`, part-file helpers), `cc-health-tx`
(`Severity`, `Action` — for stringifying the conclusion), `arrow` +
`parquet` (reading the dataset), `serde`/`serde_json` (timeline JSON),
`sha2` (the canonical hash).
