# Phase 5 — `cc-config` + `cc-mission-log` + supervision

**Goal (dev plan):** the mission dataset exists, crash-safe, replayable. A 1 h
SITL mission produces a complete, `log-inspect`-clean dataset; kill/disk-full
tests pass.

This phase was designed with a **judge-panel process**: four independent
designs (durability / simplicity / throughput / testability angles) were scored
by an adversarial judge panel. All four converged on the same crash-safety
answer; the judges unanimously flagged the same handful of fixes. What follows
is the synthesized, judge-corrected design as built.

---

## Part A — Design

### A.1 The crux: crash-safe Parquet (row-group-per-file)

The whole phase turns on one question: after a `kill -9` or power loss, which
rows are readable? A long-lived `ArrowWriter` that flushes row groups
periodically is the trap — flushed data pages **without a footer** are not
openable by any standard reader, so it fails the "flushed row groups readable
after kill -9" requirement literally.

The answer, adopted from all four designs: **one flush = one complete Parquet
part file.** [`part::seal_part`](../../crates/cc-mission-log/src/part.rs):

1. write the batch to `NNNNNN.parquet.inprogress`;
2. `ArrowWriter::into_inner` — finalizes the file (**writes the footer**) and
   hands back the `File`;
3. `fsync` the file (bytes + footer durable);
4. **atomic rename** `.inprogress` → `NNNNNN.parquet` (same directory);
5. `fsync` the directory (the rename itself is now durable).

Every sealed `NNNNNN.parquet` is therefore a self-contained, footer-complete
file readable by any Parquet reader (pyarrow, duckdb, our reader) with **zero
recovery code**. The fsync ordering — file → rename → dir — is the point, and is
asserted by a unit test via a recording syncer.

Because rows accumulate **in memory** ([`batch::RowBuf`], plain typed `Vec`s)
and a part file only ever exists in its final, footered form (the `.inprogress`
name lives only for the microseconds of step 4), the on-disk state is always
"sealed part" or "nothing" — there is never a long-lived footer-less file. The
bounded loss on a crash is exactly the in-memory buffer of each stream:
`≤ flush_rows` rows or `≤ flush_secs` of data.

Enforced **one row group per part** (`max_row_group_row_count == batch rows`)
so the loss is scoped to a single tail and the file is trivially reader-clean.

### A.2 Schema — single source, typed, self-describing

[`schema`](../../crates/cc-mission-log/src/schema.rs) is the **one** definition
of every on-disk schema; the writer ([`batch`]) and the reader ([`inspect`])
both call it, so a schema change is a compile-time event, never a silent drift.
Each stream schema is the shared identity envelope (§3.4) followed by that
stream's typed payload columns:

- **Envelope** (12 cols): `vehicle_id, mission_id, px4_boot_id, cc_boot_id`
  (segment-constant, RLE-collapsed → each lone part stays join-able on the
  dedup key), `stream_id`, `sequence` (nullable — SafetyStatus has none),
  `fc_timestamp_us`, `cc_receive_time_ns`, `seq_gap`, `age_ns` (**nullable** —
  null = unknown timesync offset, never fabricated), `age_locked` (bool —
  disambiguates a real `0 ns` age from unknown), `schema_version`.
- **Payloads**: one typed column per wire field; fixed wire arrays (`q`,
  `actuator_output`, …) are `FixedSizeList<Float32, N>` so the width is enforced
  by the type. NaN (e.g. unused actuator slots, no-airspeed estimator ratio)
  survives the round-trip.

### A.3 Segments, resume, manifest

A **segment** is one directory for one `(mission_id, cc_boot_id, px4_boot_id)`.
Segments split on: companiond restart (new `cc_boot_id`), PX4 reboot (new
`px4_boot_id`), or a size/time rotation cap — bounding single-file loss and the
replay blast radius.

**Restart continues the same mission (spec §7).** On startup
[`Mission::open`](../../crates/cc-mission-log/src/mission.rs) scans
`mission_root` for an incomplete mission for this vehicle; if found it **resumes
that `mission_id`** and opens a new segment, and it **retroactively finalizes**
the segment the crashed process left open — recomputing its stats from the
sealed parts on disk and stamping `close_reason="cc_restart"` — so the manifest
reconciles and a cleanly-resumed mission reads Clean. `cc_boot_id` and
`mission_id` are **persisted monotonic counters** (temp→fsync→rename→dir-fsync;
fail-open to 1 + WARN on corruption) so every restart yields a strictly greater
id and cross-reboot collisions are impossible.

`manifest.json` is written **first** at segment open (with `complete=false` and
an open-segment placeholder) and rewritten atomically at each segment close and
at clean mission end. It carries provenance (`dialect_hash`, `dialect_sha256`,
`schema_version`, `cc_sw_version` from `git describe`) and the per-segment /
per-stream rollup. It is **advisory**: `log-inspect` recomputes authoritative
counts from the part footers and reconciles.

### A.4 The shed ladder (disk pressure)

[`shed::ShedLadder`](../../crates/cc-mission-log/src/shed.rs) is a pure state
machine: free-space → stage, with hysteresis (escalate immediately on a falling
reading, de-escalate one step per tick only after clearing a higher *resume*
threshold). Shed order (spec §5.6): **raw first**, then Class B (Imu) + F
(Actuator), then Class C/D/E (Power/Gps/Estimator) — but **never** State, Event
or SafetyStatus. The system never silently stops writing everything: at the
deepest stage it still lands State + Event + Safety rows and the drop ledger.

A **startup floor** refuses to *open* a mission below `disk.floor_bytes`. Every
dropped row is counted (per-stream atomics in [`health::LogHealth`]) and
ledgered into `events/` — which is **part-rotated exactly like the telemetry
streams**, so drop/shed forensics survive a crash too (a single growing
`events.parquet` would be footer-less and lost precisely when it matters).

### A.5 Wiring & non-blocking guarantee

The mission-log task ([`task`]) is the **only** disk writer and a **lossy**
subscriber of the telemetry broadcast: a slow disk makes it lag (broadcast
drops the oldest, counted as `lagged`) rather than back-pressuring RX
(spec §5.2). It owns a second broadcast receiver, the pre-decode raw tap, the
PX4 boot-id watch (segment rotation), and an independent 500 ms tick (time-cap
seals so a **silent** stream still seals within `flush_secs`; disk polling;
size/time rotation).

companiond's [`supervise`](../../apps/companiond/src/supervise.rs) task does the
handshake: on the stub-ack (first FC heartbeat → link leaves DOWN) it opens the
mission, writes the param snapshot, spawns the log task, and streams
`CC_MISSION_CONTEXT` at 1 Hz (P1) until shutdown, then seals + marks the mission
complete.

### A.6 Test seams (deterministic crash / disk-full without `kill -9`)

[`env`](../../crates/cc-mission-log/src/env.rs) injects `Clock`, `SpaceProbe`
and `Syncer`. Real impls back production; fakes drive host unit tests. A
"crash" is modelled by **dropping the Segment/Mission without finalizing** —
byte-identical to a real `kill -9` between seals, but reproducible in
milliseconds. `FakeSpace` scripts a whole shedding-ladder run; `NoopSyncer`
makes the crash tests fast; a recording syncer asserts the seal ordering.

---

## Part B — Crate inventory & deviations

### B.1 `crates/cc-config`

Layered config: built-in defaults → TOML file → environment
(`CC_<SECTION>__<FIELD>`) → CLI. Per-field precedence via an all-`Option`
`PartialConfig` mirror; the merge is a pure function (unit-tested per layer);
cross-field validation (`vehicle_id != 0`, serial-needs-path, `floor ≥ seg_cap`,
hysteretic + strictly-ordered thresholds). Public flat `Overrides` for the CLI.

### B.2 `crates/cc-mission-log`

`env` (seams) · `schema` (single-source) · `batch` (RowBuf → RecordBatch) ·
`part` (the crux) · `writer` (per-stream rotation + independent seal ticker +
manifest stats) · `shed` (pure ladder) · `raw` (length-prefixed capture) ·
`events` (part-rotated ledger) · `ids` (atomic counters) · `manifest` ·
`segment` · `mission` (resume + rotation + disk polling) · `params` · `task`
(the single async writer) · `health` · `inspect` (reader → three-state verdict).

### B.3 `apps/log-inspect`

Thin sync binary over `inspect::inspect_mission`: human summary or `--json`,
`--raw` capture summary, `--lenient`. Exit 0 Clean / 1 Dirty / 2 Corrupt /
3 usage.

### B.4 companiond

`cc-config` replaces flag parsing (back-compat flags retained + `--config`,
`--vehicle-id`, `--mission-root`, `--disk-floor`, `--param-snapshot`); the
mission supervisor + raw tap; `log` object in the status JSON; `build.rs` stamps
`CC_SW_VERSION`.

### B.5 Deviations

- **D35 — crash-safety = row-group-per-file** (not a long-lived writer with
  periodic flush). A footer-less flushed file is unreadable; part-files satisfy
  "flushed row groups readable after kill -9" literally with zero recovery code.
  Accepted loss: the in-memory buffer (`≤ flush_rows` / `≤ flush_secs`). Every
  design proposed this; the judges unanimously endorsed it.
- **D36 — `events/` is part-rotated** like the telemetry streams (judges
  flagged a single `events.parquet` as a crash hole that loses all drop/shed
  forensics for the crashed segment).
- **D37 — restart resumes the same `mission_id`** and retroactively finalizes
  the crashed segment (`close_reason="cc_restart"`) from disk. Corrects a §7
  violation the judges caught in three of the four designs (which minted a new
  mission on restart).
- **D38 — independent seal ticker.** The `flush_secs` cap is enforced on the log
  task's tick, not only inside `push`, so a silent/stalled stream still seals on
  time (judge-caught "stalled stream" loss bug).
- **D39 — persisted monotonic `cc_boot_id` + `mission_id`** (not random/uuid);
  fail-open to 1 + WARN. Total, collision-free segment ordering with an unset
  RTC.
- **D40 — two-column age** (`age_ns` nullable + `age_locked` bool) to
  disambiguate a genuine locked `0 ns` from unknown offset (invariant: missing
  age is never fabricated).
- **D41 — pre-decode raw tap** (`cc_link::spawn_with_raw_tap`): `raw_mavlink.bin`
  is the exact wire bytes tapped **before** decode, so it is ground truth
  independent of the decoder. Lossy `try_send` — never blocks RX. Existing
  `spawn()` unchanged (Phase 4 behaviour preserved).
- **D42 — mission-log writer runs inline in the single log task** (not per-stream
  threads with bounded channels). Keeps the "never shed State/Event/Safety"
  invariant airtight (no writer-stall backpressure path that could drop Class
  A) and the RX-non-blocking guarantee (the task is a lossy broadcast
  subscriber). Trade-off: a seal's fsync briefly occupies one runtime worker —
  acceptable at Phase-5 rates; moving seals to a dedicated blocking thread is a
  Phase-8 optimization.
- **D43 — ENOSPC contract.** A failed flush (e.g. ENOSPC despite the shedding
  floor) is counted (`write_errors`, per-stream `dropped`) and the batch
  dropped; the log task never panics — a full disk degrades, never crashes.
- **D44 — PX4 param snapshot: Stub for Phase 5.** The config supports
  `real|stub|off`; companiond writes a deterministic stub (`px4_params_snapshot.json`
  with completeness fields). A real `PARAM_REQUEST_LIST` capture is deferred —
  keeping the safety-critical handshake window free of a param flood (a concern
  the judges raised) and the crash/disk harness independent of FC param timing.

---

## Part C — Results

### C.1 Unit / integration suites (host, deterministic)

| Suite | Result |
|---|---|
| `cc-config` | **13/13** — layer precedence (per level + same-field), malformed/unknown TOML, env parse, every validation rule |
| `cc-mission-log` | **29/29** — schema round-trips + FixedSizeList widths + NaN/null; the seal crux (stock-reader-openable, fsync order, no stray inprogress); the shed ladder (full walk + hysteresis + never-shed + exact per-stage set); writer rotation (row cap + independent time cap + gap totals); atomic counters + manifest; raw round-trip; events parts; **lifecycle: clean→Clean, crash-drop→Dirty (sealed parts survive), resume→same mission_id + Clean, disk-full→shed order + drop ledger + state-never-shed, stray-inprogress→Dirty, boot-change→2 segments** |
| clippy | clean, whole workspace |

The crash and disk-full paths are proven **deterministically** (no real
`kill -9`) via the injected seams — fast and 100 % reproducible.

### C.2 SITL integration + fault drills

`tools/phase5/sitl_phase5_check.py` — **20/20** against headless SIH SITL and
the release `companiond`:

- **clean mission** → `log-inspect` **CLEAN** (exit 0): complete, dialect +
  schema match, all six streams have rows, zero drops, raw present.
- **crash drill**: `kill -9` mid-mission → **DIRTY** (exit 1), incomplete, but
  the sealed parts are readable (rows survived); **restart resumes the same
  `mission_id`** with a new segment and a clean shutdown reads **CLEAN**, two
  segments linked in one manifest.
- **disk-full drill**: shed thresholds forced above free space → ladder reaches
  **SHED_CRIT**, WARN set, imu/actuator shed while **State keeps landing**, drop
  ledger populated, `log-inspect` **DIRTY** with the queryable drops.

### C.3 Soak (dev-plan exit criterion: 1 h `log-inspect`-clean mission)

`tools/phase5/sitl_phase5_check.py --soak 3600` — one hour, unattended,
`companiond` alive throughout, clean shutdown, then `log-inspect`:

| | Result |
|---|---|
| verdict | **CLEAN** (exit 0) |
| complete | true |
| total rows | **363 233** |
| total drops | 0 |
| sequence gaps | 0 (every stream, every segment) |
| segments | 3 — rotated on the 30 min `seg_cap_secs` cap (two full 1800 s segments + a sub-second tail before shutdown), all cleanly closed |
| parts/stream/segment | 172 (a 30 min segment at the 10 s time cap) |

Per-stream row totals (summed across segments) vs nominal `Hz × 3600`, all
within tolerance — the CC streams deliver at ~86 % of nominal in SITL (the same
~22 Hz State / ~43 Hz IMU observed in Phase 4):

| stream | rows | ~expected | |
|---|---|---|---|
| state | 77 836 | 90 000 | 86 % |
| imu | 155 671 | 180 000 | 86 % |
| power | 31 134 | 36 000 | 86 % |
| gps | 15 567 | 18 000 | 86 % |
| estimator | 31 135 | 36 000 | 86 % |
| actuator | 51 890 | 60 000 | 86 % |

Raw capture: **374 172 frames** across the segments (`raw_mavlink.bin`, torn-tail
clean). **Phase 5 exit criterion met.** Evidence: `tools/phase5/last_run.log`.

> A first soak run captured the identical clean dataset but the harness's
> row-count assertion compared each segment against the whole-hour expectation
> instead of summing across the (rotated) segments; the assertion was corrected
> and re-verified against the recorded mission before this green run.
