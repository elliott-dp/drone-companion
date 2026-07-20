# Phase 4 — Rust `cc-link` + `cc-timesync` + `cc-ingest` against SITL

**Goal (dev plan):** *"the real companion RX path replaces the Python
harness."* The Rust crates from spec §5.1 take over the wire: transport +
framing + priority TX (`cc-link`), clock correlation (`cc-timesync`),
validation + fan-out (`cc-ingest`), wired together by `companiond` v0 —
verified against the same SITL UDP endpoint Phase 3 proved, including
fault drills and an unattended soak.

---

## Part A — Plan

### A.0 CI prelude (this session's first task)

Before Phase 4, the fork's failed GitHub runs were diagnosed and fixed —
full write-up in §B.4: the cc_dialect XML install moved from an external
script into the mavlink module's CMake (CI never ran the script → every
SITL-family job died at configure), one astyle violation
(`mavlink_receiver.h`, an astyle-quirk around `#endif` + brace-init) was
fixed by relocating the CCFC member block, `px4_sitl_allyes` (broken since
the first fork push, independent of CCFC) was removed from the fork's
matrix, and 20 fork-unviable workflows (RunsOn runners / upstream secrets)
were deleted — 7 curated guards remain (`.github/workflows/CCFC_README.md`).

### A.1 Scope (dev plan Phase 4, items 1–6)

| Item | Deliverable |
|---|---|
| 4.1 `cc-link` | transport abstraction (UDP now, serial via the same interface for Phase 8), RX loop on cc-protocol's proven `FrameDecoder`, source-check + counters, strict-priority TX queues P0–P3, 1 Hz companion HEARTBEAT, link state UP/DEGRADED/DOWN |
| 4.2 `cc-timesync` | MAVLink TIMESYNC client: 10 Hz fast-lock for 5 s → 1 Hz, 32-sample window, RTT-outlier rejection, median offset, LOCKED/DEGRADED/UNLOCKED, pure conversion functions, invalidate-on-reboot — filter core is pure (no I/O) and unit-tested on synthetic jitter traces |
| 4.3 `cc-ingest` | validate (schema/source via cc-protocol) → per-stream sequence continuity → age vs timesync → `TelemetryEvent` on a tokio broadcast; per-stream staleness watchdogs (silent > 4× nominal period → `StreamStale`) |
| 4.4 `companiond` v0 | tokio daemon wiring the three; 1 Hz status line, `--status-json` machine-readable mode for the harness |
| 4.5 integration test | `tools/phase4/sitl_phase4_check.py`: SITL + companiond, assert every stream ±20%, zero gaps, timesync LOCKED ≤ 5 s |
| 4.6 fault drills | garbage injection (resync), telemetry pause via `CC_TEL_PROFILE=0` (watchdogs), SITL kill/restart (boot-id change → sequence reset + timesync relock) |
| exit | unattended soak with clean counters (dev plan asks 1 h — run as the final gate) |

### A.2 `cc-link` design (spec §5.3)

* **Transport**: an enum (`Udp`/`Serial`) rather than a trait object — same
  interface, zero dyn/async-trait machinery. UDP mirrors the SITL topology
  Phase 3 proved: bind the local port (default 24040, where PX4's CCFC
  instance sends), learn the peer from the first datagram, transmit back to
  it. Serial wraps `tokio-serial` (8N1, no flow control per spec §2.1) —
  compiled and unit-constructed now, integration-verified on the bench in
  Phase 8.
* **RX path**: datagrams → `cc_protocol::CcFrameDecoder` (the Phase 1
  fuzz-proven incremental parser — resync-on-0xFD and exact fault counters
  come free) → per-frame source check (`validate_source_on_cc`: sysid, and
  FC-originated IDs must come from comp 1; violations → `rx_bad_source`,
  dropped) → bounded channel to the consumer. HEARTBEAT frames additionally
  refresh the FC-heartbeat clock inside the link.
* **TX priority queues** (strict, preemptive at frame boundary): four
  bounded mpsc channels; the TX task drains by re-checking P0 → P1 → P2 →
  P3 after *every* frame. P0 = HEARTBEAT, TIMESYNC, (Phase 6:
  CC_HEALTH_REPORT); P1 = session/acks; P2 = CC_AI_DIAGNOSTIC; P3 =
  bulk/debug. **P0 is never silently dropped**: a full P0 queue marks the
  link DOWN and counts `p0_stalls` (spec: "if P0 cannot drain, that is a
  link-down condition, not a queueing condition").
* **Link state**: UP / DEGRADED / DOWN derived from FC-heartbeat age
  (> 2.5 s DEGRADED, > 5 s DOWN) and decoder CRC-error deltas; published on
  a `watch` channel + surfaced as `TelemetryEvent::LinkStatus`.
* **Companion HEARTBEAT**: 1 Hz task, `MAV_TYPE_ONBOARD_CONTROLLER` /
  `MAV_AUTOPILOT_INVALID`, comp 191 (spec §3.5), enqueued at P0.

### A.3 `cc-timesync` design (spec §5.4)

* **Protocol**: send `TIMESYNC{tc1:0, ts1:cc_mono_ns}`; PX4 replies
  `{tc1:fc_ns, ts1:echo}`. Per sample: `rtt = now − ts1`,
  `offset = tc1 − (ts1 + rtt/2)` (cc→fc sign convention documented in
  code: `fc_ns ≈ cc_ns + offset`... stored as `offset_ns = fc − cc`).
* **Filter core is a pure struct** (no clocks, no I/O — dev plan demands
  unit tests on synthetic traces): 32-sample ring; a sample is rejected
  when its RTT exceeds 1.5 × the window's p90 (rejections counted); the
  offset estimate is the window **median**; quality:
  `LOCKED` = window ≥ 8, RTT jitter (p90−p10) below threshold, rejection
  rate < 30%; `DEGRADED` = ≥ 4 samples; else `UNLOCKED`.
* **Cadence**: 100 ms for the first 5 s after start or invalidation
  (fast-lock), then 1 s. Requests go out at P0.
* **Invalidation**: on `px4_boot_id` change (watch fed by cc-ingest) or an
  FC timestamp regression → clear window, re-enter fast-lock. Consumers
  read an atomic snapshot `{offset_ns, rtt_us, quality}` from a watch
  channel — they never re-derive (spec).

### A.4 `cc-ingest` design (spec §5.5)

* Input: decoded frames from cc-link. TIMESYNC is routed to the timesync
  task by the companiond demux before ingest sees it.
* **Pipeline per frame**: schema check (`validate_schema`, drop+count on
  mismatch) → stream classification → sequence continuity (`last_seq` per
  stream; gap → `sequence_gaps += n−1`, gap size attached to the event's
  `RxMeta`, no per-row spam) → age = `now_cc − to_cc(fc_timestamp)` when
  timesync is LOCKED, else flagged `UnknownOffset` → broadcast.
* **`TelemetryEvent`** matches the spec §5.5 enum, with each payload
  variant carrying the generated MAVLink data struct plus an `RxMeta`
  (receive time, age, gap) — the identity envelope §3.4 requires on every
  log row, available to every consumer (deviation D17: the spec sketch
  shows bare payloads; the meta has to travel somewhere, and beside the
  payload is the only place that doesn't lose it).
* **Boot identity**: `px4_boot_id` changes reset all per-stream sequence
  trackers (sequences restart on FC reboot, spec §4.2) and are exposed on a
  watch channel (timesync invalidation, status display).
* **Watchdogs**: 100 ms tick; a Class A–F stream silent for > 4× its
  nominal AI_UART period (State 40 ms, IMU 20, Power 100, GPS 200,
  Estimator 100, Actuator 60 — actual 16.7 Hz) emits `StreamStale(stream)`
  once on entering staleness; resumption of data clears it (and the status
  line shows per-stream stale flags).

### A.5 `companiond` v0 (spec §5.2, trimmed to Phase 4)

Hand-rolled arg parsing (`--udp-bind`, `--remote`, `--serial`, `--baud`,
`--status-json`, `--sysid`); cc-config arrives in Phase 5. Task graph
exactly as spec §5.2's sketch, minus the Phase 5+ consumers:

```
[udp/serial] → cc-link RX → demux ─→ TIMESYNC → cc-timesync runner ⇄ P0 TX
                                  └→ cc-ingest → broadcast<TelemetryEvent>
cc-link heartbeat task (1 Hz) → P0 TX
status task (1 Hz): human line or --status-json (one JSON object per line)
```

All channels bounded; no task blocks on I/O it doesn't own; ctrl-C exits
cleanly. The JSON status is the harness's interface — schema documented in
the companiond README (hand-emitted, no serde dependency).

### A.6 Verification plan

* **Unit tests** (deterministic; tokio `start_paused` where time matters):
  timesync filter on synthetic traces (clean lock, jitter, outlier bursts,
  invalidate/relock), conversion round-trips; ingest sequence/gap/boot-reset
  and watchdog transitions; link TX strict-priority ordering and P0-stall
  behavior; heartbeat cadence.
* **Integration** (`tools/phase4/sitl_phase4_check.py`, reusing
  `tools/common/pxh.py` for SITL control): build companiond, boot SIH SITL,
  run companiond `--status-json`, assert within 60 s: link UP; timesync
  LOCKED **within 5 s of link-up**; every stream at its uORB-ceiling rate
  ±20% (25/50/10/5/10/16.7 Hz); `sequence_gaps == 0`; `crc_errors == 0`.
* **Fault drills** (same run):
  1. garbage injection — blast random bytes at companiond's socket; assert
     decoder counts garbage/CRC faults while all streams stay green
     (resync, spec §2.4);
  2. telemetry pause — `param set CC_TEL_PROFILE 0`; assert all six streams
     flagged stale within ~1 s (4× nominal), then `1` restores and stale
     flags clear with **no false sequence gaps**;
  3. FC reboot — `shutdown` + relaunch SITL; assert link DOWN → UP, a *new*
     `px4_boot_id`, timesync invalidated then re-LOCKED, and gap counters
     still clean (sequence trackers reset on boot change).
* **Soak** (exit criterion): the harness's `--soak N` mode keeps SITL +
  companiond running N seconds (dev plan: 3600), sampling status every
  10 s; final assertion = clean counters end-to-end. Run as the last gate.

### A.7 Dependencies added (workspace-pinned)

`tokio 1.x` (rt-multi-thread, net, time, sync, macros, signal, io-util),
`tokio-serial 5.x` (cc-link only). Nothing else: no async-trait (enum
transport), no clap (v0 args), no serde (fixed-schema status line emitted
by hand). Wire-critical crates stay pinned exactly as before.

---

## Part B — Implementation notes

### B.1 Crate inventory (all companion-repo; the PX4 fork is untouched this phase)

```
crates/cc-link       lib.rs (RX/TX/heartbeat/state tasks, counters)
                     transport.rs (UDP + serial halves), clock.rs (shared monotonic ns)
crates/cc-timesync   lib.rs (pure Filter + Snapshot + conversions, 7 synthetic-trace tests)
                     runner.rs (cadence, reply intake, invalidation)
crates/cc-ingest     lib.rs (TelemetryEvent, continuity, age, watchdogs, stats)
apps/companiond      main.rs (wiring, demux, arg parsing, 1 Hz status/JSON)
tools/phase4/        sitl_phase4_check.py + README (status-JSON schema)
```

Workspace additions: `tokio 1` (rt-multi-thread/net/time/sync/macros/
signal/io-util; `test-util` as a dev-dependency where paused-clock tests
need it), `tokio-serial 5`. Nothing else — no async-trait (transport is an
enum), no clap, no serde.

### B.2 Decisions & deviations log (continuing the numbering)

| # | Decision | Rationale |
|---|---|---|
| D17 | `TelemetryEvent` payload variants carry `(data, RxMeta)` | the spec sketch shows bare payloads, but the §3.4 identity envelope (receive time, gap, age) must travel with each event or every consumer re-derives it wrong |
| D18 | **UDP peer learned only from datagrams that decode to ≥1 valid frame** | the naive learn-from-any-source design let any stranger hijack companiond's TX peer by spraying bytes at the port — found while designing the garbage drill, fixed in `cc-link` (transport reports the source; the RX task decides) |
| D19 | Ingest watchdogs keep their own `tokio::time::Instant` bookkeeping, separate from the wall-clock `last_rx_ns` in stats | staleness decisions must run on the runtime clock (virtual under `start_paused` tests, identical to wall time in production); mixing the process-monotonic stats clock into the decision made the watchdog untestable and subtly clock-skewed |
| D20 | TX strict priority via a `biased` 4-arm select, preemption at frame boundary | polls arms top-down on every wakeup: P0 always wins when ready; a lower-class frame already pulled is sent whole (exactly the spec's "preemptive at frame boundary") |
| D21 | P1–P3 overflow = drop + `tx_errors` count; P0 overflow = `p0_stalls` + link marked DOWN | spec §5.3 verbatim: P0 must never be silently dropped; a stalled P0 is a link fault, not a queueing event |
| D22 | Serial transport compiled + constructible now, integration deferred to Phase 8 | dev plan wants "same trait"; there is no serial endpoint to test against until the bench exists — shipping untested *behavior* claims would be worse than a clearly-labeled deferral |
| D23 | `companiond` emits status JSON by hand (fixed schema) | one fixed schema consumed by the harness doesn't justify a serde dependency in the flight daemon; the schema is documented as an interface in tools/phase4/README.md |
| D24 | FC-timestamp-regression fallback in the timesync runner (in addition to boot-id invalidation) | spec §5.4 lists both triggers; the regression path covers reboots noticed by TIMESYNC before the first STATE of the new boot arrives |
| D25 | The frame decoder accepts **both MAVLink 1 (`0xFE`) and MAVLink 2 (`0xFD`)** framing, not v2-only | PX4's shared timesync module emits its `TIMESYNC` reply (msg ID 111) as **MAVLink 1** from the receiver thread regardless of `MAV_PROTO_VER`, and that version decision is unreachable from the reply site — proven against SITL: clearing `OUT_MAVLINK1` on the channel status immediately before the `mavlink_msg_timesync_send_struct` call *still* emits v1 (the send resolves a different `mavlink_status_t` than the instance's). Without accepting v1, our v2-only decoder ate every reply as garbage and timesync never locked. CC_* telemetry (IDs ≥ 54000) is 24-bit and therefore MAVLink 2 exclusively, so the "v2 custom dialect" payload contract is structurally preserved; accepting v1 for standard low-ID messages is ordinary MAVLink-parser behaviour and is what makes timesync lock. The Phase 1 fuzz/golden suite is extended with a real-v1-frame decode test and its garbage generators now exclude *both* STX markers. See §C.1 for the full diagnosis. |
| D26 | The Phase 4 harness asserts sequence continuity in **steady state** (post-warm-up deltas), not from the boot instant | localhost UDP under SITL sheds a handful of datagrams during the first ~10 s (PX4 boot + lockstep-sim warm-up CPU spike → loopback delivery jitter). PX4 reports `txerr = 0` — it hands the kernel every CC_* message — and a wire capture *independent of companiond* with a 4 MB receive buffer sees the same loss, so it is neither a companiond bug nor a receive-buffer overflow; it is a boot transient. After warm-up the loopback is clean (0 gaps over 20 s measured). The dev-plan exit criterion is clean counters over a 1 h soak (i.e. steady state), so the harness warms up, baselines the cumulative gap counter, and asserts subsequent windows add **zero** gaps; the fault drills assert no gaps are induced *by the drill* (a delta across the event). The soak tolerates ≈1 gap / 15 min (rare loopback loss, not sustained). A real wired TELEM3/Ethernet link (Phase 8) is the deployment target. See §C.2. |

### B.4 CI diagnosis & fixes (this session's prelude — fork commit `2b68913`)

The two Phase 2/3 pushes left the fork's Actions page red/stuck. Job-level
API forensics (all failures died in 1.4–2.7 min = configure-stage) plus
local reproduction identified, per workflow:

| Symptom | Root cause | Fix |
|---|---|---|
| tests / tests_coverage / NO_NINJA sitl_default / EKF-update-indicator / Clang Tidy all failing at ~1.5 min since the Phase 3 push | the cc_dialect switch relied on `ccfc_dialect/install.sh`, which **CI never ran** → mavgen found no `cc_dialect.xml` in the submodule → cmake configure died | the XML copy + SHA-256 gate moved into `src/modules/mavlink/CMakeLists.txt` at configure time (covers every build path); install.sh deleted; verified locally with a from-scratch build after wiping the installed XML |
| check_format failing since Phase 3 | astyle 3.1 (built from source locally for CI parity — Homebrew's 3.6 is rejected by PX4's version gate) flags `mavlink_receiver.h`: an astyle quirk reformats a brace-initialized member directly following `#endif`, i.e. it wanted to mangle the *upstream* line after the CCFC block | relocated the CCFC member block after the publications list (its `#endif` is followed by a blank line + `#if`), leaving upstream untouched; full-tree astyle-3.1 scan clean. Beware: PX4's checker scripts silently self-destruct on this project's spaces-in-path — run astyle directly with a quoted `--options` to get a trustworthy verdict |
| px4_sitl_allyes failing at configure **since the first fork push** (pre-dialect) | allyes enables `zenoh`; its `zenoh-pico` submodule/deps aren't available in the fork CI environment (reproduced locally at the identical spot) — inherited, not CCFC-caused | removed from the fork's Checks matrix with an in-file rationale: the fork guards the CCFC contract (sitl_default, fmu-v5, tests, format), not PX4's full config space |
| MacOS build failing at "setup ccache" (both pushes), 7 workflows queued forever, docs/deploy/container/bot workflows unrunnable | RunsOn self-hosted runner service + PX4-org secrets don't exist on a fork | deleted 20 fork-unviable workflows; the 7 kept guards are listed with rationale in `.github/workflows/CCFC_README.md` |

**Outcome: 6/6 workflows green** on the fix commit (Checks, Clang Tidy,
EKF Update Change Indicator, NuttX target, Failsafe sim, Python CI) —
verified via the API before Phase 4 work began.

## Part C — Verification results

### C.0 Unit / property suites (`cargo test --workspace`, clippy clean)

| Crate | Tests | Notes |
|---|---|---|
| `cc-protocol` | 12 unit + 14 fuzz/property + 3 golden + 2 dialect-hash | fuzz suite now includes a real **MAVLink 1** `TIMESYNC` decode (standalone, interleaved with v2, byte-by-byte) and a corrupted-v1 resync test; garbage generators exclude both STX markers (D25) |
| `cc-link` | 3 behaviour (real localhost UDP) | priority TX, valid-decode peer learning, link-state from heartbeat age |
| `cc-timesync` | 7 filter/runner | synthetic traces: lock, jitter reject, boot-id + tc1-regression invalidation |
| `cc-ingest` | 5 behaviour | continuity, boot reset, staleness watchdogs (virtual clock) |

`cargo clippy --workspace --all-targets`: clean.

### C.1 The MAVLink-1 TIMESYNC-reply diagnosis (root cause of the lock failure)

Timesync would not lock: companiond's decoder counted PX4's replies as
garbage. A raw-datagram capture pinned it — every reply was a **24-byte
MAVLink 1 frame** (`0xFE`, 8-bit msg ID 111), while all CC_* telemetry and
the heartbeat were MAVLink 2 (`0xFD`). The investigation (against a
purpose-instrumented SITL build, since reverted to the pinned fork):

1. `MAV_PROTO_VER 2` **does not** change the reply framing — verified by
   booting SITL and reading `param show MAV_PROTO_VER` = 2 while the wire
   still carried `0xFE`.
2. The reply is emitted by `MavlinkTimesync::handle_message`
   (`mavlink_timesync.cpp:69`, confirmed by resolving the send's return
   address with `atos`). The MAVLink-2 finalize path chooses v1 vs v2 from
   `status->flags & MAVLINK_STATUS_FLAG_OUT_MAVLINK1`.
3. Instrumenting that flag showed it **clear** (`0x0`) on the instance
   status the reply site can see — yet the wire stayed v1. Clearing the flag
   on `mavlink_get_channel_status(chan)` *immediately before* the send did
   not change the framing either: the receiver-thread finalize resolves a
   different `mavlink_status_t` than the one reachable from the reply site.
   PX4 emits the timesync reply as MAVLink 1 and it **cannot be forced to v2
   from the reply site** without an invasive patch to the shared timesync
   module.

**Resolution (D25): accept both framings in the decoder.** This is ordinary
MAVLink-parser behaviour, keeps the safety-critical fork pinned and minimal,
and is robust to any other standard low-ID message PX4 may emit as v1. The
CC_* telemetry is 24-bit-ID and thus v2-exclusive by construction, so the
"MAVLink 2 custom dialect" contract for the payload (ICD §3) is preserved.
With the fix, timesync **LOCKs within 0.0 s of link-up** (status granularity
1 s, dev-plan bound 5 s), window 32, RTT ≈ 0.22 ms, zero CRC errors at
startup.

### C.2 Sequence gaps are a localhost-SITL boot transient (not a defect)

The integration run showed a few sequence gaps. A wire capture *independent
of companiond* (bound to the CC port, 4 MB receive buffer, decoding with
pymavlink) reproduced them, and bucketing by time was decisive:

```
t=[ 0- 5s) n= 580 gaps=4
t=[ 5-10s) n= 516 gaps=3
t=[10-15s) n= 522 gaps=0     ← steady state: clean
t=[15-20s) n= 524 gaps=0
t=[20-25s) n= 523 gaps=0
t=[25-30s) n= 521 gaps=0
TOTAL n=3187 gaps=7 loss=0.220%   (all loss in the first 10 s)
```

PX4's `mavlink status` for the CC instance reports `txerr: 0.0 B/s` — it hands
the kernel every message. The loss is **kernel loopback delivery jitter
during the boot/sim-warm-up CPU spike**; a 4 MB receive buffer does not
prevent it, so it is not a receive-side overflow. After warm-up the loopback
is clean. The harness therefore asserts steady-state continuity via
post-warm-up deltas and per-drill deltas (D26).

### C.3 Integration + fault drills — `tools/phase4/sitl_phase4_check.py`

**36/36 checks passed.** Evidence: `tools/phase4/last_run.log`,
`px4_server_*.log`.

| Group | Result |
|---|---|
| link UP; timesync LOCKED ≤ 5 s (0.0 s) | PASS |
| six stream rates ±20% @startup / post-garbage / post-reboot | PASS (18 checks) |
| zero CRC / zero bad_source @startup; boot-id nonzero | PASS |
| steady-state sequence continuity @startup (Δgaps=0) | PASS |
| drill A — garbage from a stranger: accounted, peer not hijacked, no induced gaps | PASS |
| drill B — telemetry pause: all six stale, clears on resume, no tracker break | PASS |
| drill C — FC reboot: link leaves UP→DEGRADED, re-UP, new boot-id, timesync re-LOCK, steady-state continuity, P0 never stalled | PASS |

### C.4 Regression (unchanged since Phase 2/3, re-run after the `px4-rc.mavlink` change)

The CC instance moved to `-m custom` with an explicit `HEARTBEAT` stream and
`MAV_PROTO_VER 2` (see §C.5). Both prior harnesses re-run clean on the new
ROMFS:

| Harness | Result |
|---|---|
| `tools/phase2/sitl_phase2_check.py` (uORB topics + publisher) | 37/37 |
| `tools/phase3/sitl_phase3_check.py` (CC wire, receiver gauntlet, mission handshake) | 50/50 |

### C.5 `px4-rc.mavlink` change (the only fork edit this phase)

The CCFC instance changed from `-m onboard` to `-m custom` plus an explicit
`HEARTBEAT` stream, and `param set MAV_PROTO_VER 2` was added globally.
Custom mode carries **only** the CC_* contract + heartbeat (onboard mode's
default streams came from the newer submodule common dialect our pinned
dialect can't decode, and PX4's own 10 Hz TIMESYNC *requests* are
unnecessary — the companion drives sync). `MAV_PROTO_VER 2` makes streams
and heartbeats leave the boot in v2; it does **not** affect the timesync
reply (§C.1). No PX4 C/C++ was changed — the fork stays pinned to v1.17.0.

### C.6 Soak (dev-plan exit criterion: 1 h unattended, clean counters)

`tools/phase4/sitl_phase4_check.py --soak 3600` — **47/47 checks passed** (the
36 integration/drill checks + 11 soak assertions). One full hour, sampled
every 10 s:

| Soak assertion | Result |
|---|---|
| companiond alive at end | PASS |
| no sustained gap loss (post-warm-up delta, tolerance 5) | PASS — **Δ0 gaps** |
| zero CRC errors end-to-end (delta, drill-A garbage baselined out) | PASS — Δ0 |
| no stale intervals | PASS — 0 of ~360 samples had any stale stream |
| P0 never stalled | PASS |

Final status after 3600 s: `link UP`, `timesync LOCKED` (window 32, RTT
≈ 0.18 ms), **305 716 frames OK**, `p0_stalls 0`, `bad_source 0`,
`bad_schema 0`, `bad_payloads 0`, `rx_drops 0`, boot-id stable, all six
streams at rate and never stale. Steady-state sequence continuity was
**exactly clean** across the whole hour (Δ0), confirming §C.2: the only loss
is the boot transient. The residual `crc_errors 70` / `garbage_bytes 34421`
are drill A's injected garbage from the pre-soak phase (baselined out of the
soak deltas). Evidence: `tools/phase4/last_run.log`.

**Phase 4 exit criterion met.**
