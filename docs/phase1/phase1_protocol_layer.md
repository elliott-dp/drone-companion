# Phase 1 — Protocol layer proven on the bench

**Goal (dev plan):** *"the dialect encodes/decodes identically in C and
Rust."* No PX4 code, no hardware — this phase de-risks every later phase,
because after it the wire format can only break loudly, never silently.

**Status: complete.** 29/29 tests green, clippy clean, all generation
deterministic. Run everything with:

```sh
cd cc-dialect && ./gen_c.sh && (cd golden && ./build_golden.sh)   # regenerate C side
cargo test --workspace                                            # prove the contract
```

---

## 1. The problem Phase 1 solves

MAVLink appends a per-message `CRC_EXTRA` byte — a hash of the message's
*definition* (name, field names/types/order) — into every frame's checksum.
If the C bindings (PX4 side) and the Rust bindings (Jetson side) are ever
generated from even slightly different XML, the CRC_EXTRA values differ and
**every frame of that message fails CRC and is silently dropped**. No error
message, no exception — telemetry just never arrives. The same silent-death
mode applies to field *reordering* (MAVLink sorts fields by type size on the
wire) and to MAVLink 2 payload truncation rules.

The defense is mechanical, not procedural: encode known frames with the C
encoder, decode and re-encode them with the Rust stack, and fail CI on any
disagreement. That is the golden-vector mechanism.

## 2. What was built (map to the plan's four points)

| Plan item | Deliverable |
|---|---|
| 1.1 vendor C headers; wire `cc-protocol` build.rs; both compile | `cc-dialect/generated/c/` (mavgen, pinned pymavlink 2.4.49); `crates/cc-protocol/build.rs` (mavlink-bindgen 0.18.0, pinned exact) — see [cc_protocol_crate.md](cc_protocol_crate.md) |
| 1.2 golden vectors from a C program | `cc-dialect/golden/gen_golden.c` + `build_golden.sh` → `golden_frames.bin` (16 frames, 749 bytes) + `golden_manifest.txt` |
| 1.3 Rust round-trip test | `crates/cc-protocol/tests/golden_roundtrip.rs` (3 tests) |
| 1.4 fuzz/property tests | `crates/cc-protocol/tests/fuzz_decoder.rs` (12 tests) + framing/validate unit tests (12) + hash-consistency tests (2) |

## 3. The golden set — 16 frames, frozen

`gen_golden.c` compiles against the **vendored mavgen C headers** — the same
encoder family PX4 runs — and encodes one instance of every message with
fixed, hand-chosen field values. Frame order and header sequence are fixed
(header `seq` == frame index); FC-originated frames are sent as
`sysid 1 / comp 1`, companion-originated as `sysid 1 / comp 191`, exercising
the spec §3.3 addressing contract.

| # | Message | src comp | payload | CRC_EXTRA | purpose beyond field coverage |
|---|---|---|---|---|---|
| 0 | HEARTBEAT | 1 | 9 | 50 | standard messages survived our dialect generation |
| 1 | TIMESYNC | 1 | 15¹ | 34 | link/timesync dependency (spec §3.1) |
| 2 | CC_TELEMETRY_STATE | 1 | 86 | 139 | Class A |
| 3 | CC_TELEMETRY_IMU | 1 | 81 | 203 | Class B |
| 4 | CC_TELEMETRY_POWER | 1 | 40 | 115 | Class C; carries a **NaN** (temperature) |
| 5 | CC_TELEMETRY_GPS | 1 | 47 | 189 | Class D |
| 6 | CC_TELEMETRY_ESTIMATOR | 1 | 41 | 190 | Class E; NaN airspeed_test_ratio |
| 7 | CC_TELEMETRY_ACTUATOR | 1 | 46 | 229 | Class F; NaN in unused slots (spec §6 rule) |
| 8 | CC_EVENT | 1 | 27 | 11 | Class G |
| 9 | CC_SAFETY_STATUS | 1 | 28 | 93 | monitor echo/ack message |
| 10 | CC_HEALTH_REPORT | **191** | 38 | 76 | the one message that can influence PX4 |
| 11 | CC_AI_DIAGNOSTIC | **191** | 26 | 83 | log-only detail |
| 12 | CC_MISSION_CONTEXT | **191** | 41 | 78 | embeds the **real `CC_DIALECT_HASH`** |
| 13 | CC_LOG_CONTROL | **191** | 14 | 64 | dev-only profile request |
| 14 | CC_EVENT (truncation probe) | 1 | **17** of 27 | 11 | MAVLink 2 zero-truncation on the wire |
| 15 | TIMESYNC (min-payload probe) | 1 | **1** of 16 | 34 | "truncate to minimum one byte" rule |

¹ naturally truncated: the fixed `ts1` value's top byte is zero — also a
truncation exercise, discovered rather than designed, and locked in.

Design rules for the fixed values (enforced in both sources):

* **Every value is distinct and recognizable** (`0xB007B007` boot id,
  `0x0BAD` detail code, mission 777001…) so a mismatch pinpoints the field.
* **All floats are exactly representable in binary** (`-9.8125`, `0.40625`,
  `343.125`…) so cross-language asserts use exact equality, never epsilon.
* **NaN uses a pinned bit pattern** `0x7FC00000`, written via `memcpy` in C:
  the C `NAN` macro's payload is not pinned by the standard, and golden bytes
  must be identical on every platform. The Rust test asserts NaN via
  `to_bits()`, and byte-identical re-encode proves NaN payload transport.
* **No timestamps or environment data** in generator output — regeneration
  is byte-stable, so CI can `git diff` it (same policy as `gen_c.sh`, see
  [phase0_dialect_toolchain.md](phase0_dialect_toolchain.md) §5).

The values are mirrored **literally, twice**: in `gen_golden.c` and in
`golden_roundtrip.rs`. That duplication is the mechanism, not an accident —
the two sides are independent codebases, and the test failing is how a
one-sided edit is caught. Change them only together, regenerating the
vectors in the same commit.

## 4. What the round-trip test proves (`tests/golden_roundtrip.rs`)

For every frame, in order:

1. **Reference decode** through `mavlink_core::read_v2_msg` — a CRC_EXTRA or
   layout drift fails here first;
2. **Header contract**: sysid/compid/sequence match the manifest;
3. **Field-exact contract**: every field of every message asserted against
   the golden value (including `to_bits()` NaN checks);
4. **CRC_EXTRA table agreement**: Rust `MavMessage::extra_crc(id)` equals the
   manifest's value recorded by the C side (`mavlink_get_msg_entry`);
5. **Byte-identical re-encode**: `write_v2_msg` with the decoded header must
   reproduce the original frame bit for bit — this pins field ordering,
   zero-truncation, and the min-1-byte rule;
6. **Hash pipeline agreement**: frame 12's `dialect_hash` (embedded by C from
   `hash.sh` output) equals the Rust build-time constant;
7. **Own-decoder agreement**: `cc-protocol`'s `FrameDecoder` re-decodes the
   whole file and must byte-agree with the reference path (compared via
   re-serialization, because `NaN != NaN` under float `PartialEq`), with
   perfectly clean counters;
8. Manifest self-consistency: offsets/lengths tile the file exactly; nothing
   trails.

## 5. The fuzz/property suite (`tests/fuzz_decoder.rs`)

Target: `cc_protocol::framing::FrameDecoder` — the incremental parser that
`cc-link`/`cc-ingest` will sit on in Phase 4 (decoder semantics and counter
definitions: [cc_protocol_crate.md](cc_protocol_crate.md) §4). The plan's
requirement is strict: *"parser must never panic and counters must match
injected faults"* — so the suite asserts **exact** counter values wherever
the fault model is deterministic, and this conservation invariant after
every single push:

```
bytes_in == frames_ok_bytes + unknown_msg_bytes + bad_payload_bytes
            + garbage_bytes + pending()
```

All randomness is a seeded xorshift64\* — failures reproduce exactly, no
fuzzing infrastructure needed (a coverage-guided cargo-fuzz target can be
layered on later without touching this suite).

| Test | Fault class | Exact assertions |
|---|---|---|
| `never_panics_on_pure_random_streams` | 64 seeds × 16 KiB random bytes, random chunking | no panic; conservation after every push |
| `never_panics_on_mutated_valid_streams` | valid stream × 8 random bit flips × 64 seeds | no panic; `frames_ok ≤ sent` |
| `corrupted_crc_costs_exactly_one_error_and_the_frame` | 1 payload bit flipped in the middle frame of 3 | `frames_ok=2`, `crc_errors=1`, `garbage_bytes == len(corrupted frame)` |
| `interleaved_garbage_counted_byte_exact` | FD-free garbage runs 17/251/3 B between frames | `garbage_bytes == 271` exactly |
| `truncated_frame_all_cut_points_then_recovery` | every cut point 1..len−1, then a valid frame | truncated part never decodes; follow-up always recovered |
| `hotplug_mid_stream_resyncs` | start listening at every offset inside a frame (spec §2.4) | exactly 5-of-6 frames recovered for any mid-frame start |
| `unknown_msgid_counted_and_skipped_between_frames` | hand-built frame with ID 54008 (reserved, undefined) | `unknown_msg_ids=1`, `unknown_msg_bytes` exact, `crc_errors=0` |
| `unknown_msgid_at_stream_end_counted` | same, ending exactly at buffer end | boundary rule holds |
| `unknown_incompat_flags_dropped_exactly_once` | incompat flag 0x02 | `bad_incompat_flags=1`, frame drains to garbage, neighbors survive |
| `signed_frame_length_handled_and_flagged` | MAVLINK_IFLAG_SIGNED frame (13-byte signature) | framed correctly, `signed=true`, stream stays in sync |
| `five_thousand_frames_with_giant_sequences_in_7_byte_chunks` | 5000 reports, `sequence` crossing u32::MAX, 7-byte delivery | all decode; sequences verbatim; counters perfectly clean |
| `seeded_fault_soup_conserves_every_byte` | 32 seeds × random mix of valid/corrupted/unknown/garbage | every clean frame decodes, none invented; conservation throughout |

## 6. Exit criteria & how this maps to CI

Dev-plan exit criteria: *"golden round-trip green in CI; fuzz suite green."*
Both suites are green locally via the exact commands CI should run
(`cargo test --workspace`, plus the regeneration drift check) — see
[phase0_dialect_toolchain.md](phase0_dialect_toolchain.md) §7 for the job
mapping. What CI adds over this session is only *continuity*, not new
verification.

One caveat carried forward (also flagged in the phase 0 doc): PX4's own
build will generate C bindings from the copy of `cc_dialect.xml` placed in
**its** mavlink submodule, whose bundled `common.xml` may differ from our
pins. The golden vectors are exactly the tripwire for that — in Phase 3,
point the pymavlink validation harness at SITL and any divergence surfaces
as CRC failures on standard messages. Do not skip re-running the golden
suite after the PX4-side vendoring.

## 7. Decisions & deviations log (vs the letter of the plan)

1. **Golden set is a superset of "every CC_* message":** HEARTBEAT and
   TIMESYNC were added (the link cannot exist without them, spec §3.1/§3.5)
   plus two edge-case probes (truncation, min-payload). Manifest names carry
   a `_PROBE` suffix; the test accepts the suffix by prefix-match.
2. **Golden vectors embed the real dialect hash** in frame 12 instead of a
   dummy value — this turns the handshake constant into a tested, end-to-end
   value across four implementations (hash.sh, dialect_hash.h/C, build.rs,
   test).
3. **`FrameDecoder` lives in cc-protocol** (the plan calls Phase 1.4
   "cc-protocol/cc-ingest foundations"): framing is protocol-layer; the
   stateful link/ingest logic (sequence gaps, staleness, flood) stays in
   Phase 4 crates. The decoder's resync/counter semantics are documented in
   the crate doc and pinned by tests.
4. **Rust bindings are never vendored** — generated on every build. Only the
   C side is vendored (PX4-style), with CI diffing it.
5. **Versions pinned exactly** where the wire is at stake: pymavlink
   2.4.49 + lxml 6.0.2 (requirements.txt), `mavlink-core`/`mavlink-bindgen`
   `=0.18.0` (Cargo). Upstream XML pins recorded with SHA-256s in
   PROVENANCE.md. Bump any of these **deliberately** and re-run the golden
   suite in the same commit.
6. **A dialect quirk worth remembering:** `CC_LOG_CONTROL`'s message ID
   54013 = `0xD2FD` — its little-endian header always contains a `0xFD`
   byte. Harmless on the wire (MAVLink frames are length-delimited), but it
   means resync-counter tests can't use that message for "STX-free" streams;
   the fuzz helpers use CC_HEALTH_REPORT (54010) and assert FD-freeness at
   runtime.
7. **`validate::direction_of_id` follows the spec table exactly** (54008
   classified FC→CC as reserved; 54009/54014+ unclassified until the spec
   assigns them) rather than inventing a block convention.
