# Phase 0.3 — Dialect generation toolchain

**Scope:** development plan Phase 0, point 3 only (points 1, 2, 4 — repo
layout, toolchain installs, CI — are handled separately). Everything here
lives in [`cc-dialect/`](../../cc-dialect/) and was run and verified on this
machine on 2026-07-14.

---

## 1. What was built

```
cc-dialect/
├── cc_dialect.xml            the single source of truth (unchanged)
├── gen_c.sh                  → vendored mavgen C headers  (committed output)
├── gen_rust.sh               → proves Rust build-time generation + runs tests
├── hash.sh                   → dialect_hash constants (C header + Rust file)
├── requirements.txt          pinned mavgen toolchain (pymavlink==2.4.49, lxml==6.0.2)
├── upstream/                 PINNED copies of the MAVLink include chain
│   ├── common.xml  standard.xml  minimal.xml
│   └── PROVENANCE.md         where they came from + SHA-256s + upgrade rules
├── generated/                committed generation outputs
│   ├── c/                    mavgen C library (cc_dialect + common/standard/minimal)
│   ├── dialect_hash.h        C constant   (used by PX4 side + golden generator)
│   └── dialect_hash.rs       Rust mirror  (cross-checked against build.rs by test)
├── golden/                   Phase 1 artifacts — see phase1_protocol_layer.md
└── .venv-mavgen/             local venv (gitignored; created on demand)
```

## 2. Why the upstream XMLs are pinned in-repo

`cc_dialect.xml` includes `common.xml`, which includes `standard.xml` →
`minimal.xml`. Both generators resolve includes **relative to the dialect
file**, and the included files define HEARTBEAT, TIMESYNC, etc. — their
content is part of the wire contract (CRC_EXTRA of standard messages moves
between upstream revisions). Fetching them at build time would make the
build non-reproducible; pinning them next to the dialect makes every
generation input content-addressed. Provenance and upgrade rules:
[`upstream/PROVENANCE.md`](../../cc-dialect/upstream/PROVENANCE.md).

## 3. `gen_c.sh` — C bindings, vendored

What it does, in order:

1. Creates/reuses `.venv-mavgen` and installs the **pinned** toolchain from
   `requirements.txt`. (mavgen is pymavlink's generator — the same family
   PX4's own build uses; pinning the generator pins the generated code.)
2. Stages `cc_dialect.xml` + the three pinned includes into
   `build/definitions/` so include resolution can only see pinned files.
3. Runs mavgen: `--lang C --wire-protocol 2.0`, with lxml installed so the
   XML is **schema-validated** on every generation.
4. Normalizes non-deterministic output (see §5).
5. Vendors the result to `generated/c/` and refreshes the hash artifacts
   via `hash.sh`.

Run it after **any** edit to `cc_dialect.xml`:

```sh
cd cc-dialect && ./gen_c.sh
```

### PX4 note (matters from Phase 2 on)

Modern PX4 generates its MAVLink headers at firmware build time from XML in
its `mavlink/mavlink` submodule. So "vendor into PX4" means: put
`cc_dialect.xml` into
`<PX4>/src/modules/mavlink/mavlink/message_definitions/v1.0/` and set
`CONFIG_MAVLINK_DIALECT="cc_dialect"` in the board config
(`boards/cuav/v6x/default.px4board`). `./gen_c.sh --px4 <PX4_ROOT>` does the
copy and prints the reminder. The headers vendored *here* serve everything
that is **not** the PX4 firmware build: the golden-vector generator, host
tests, and the CI drift check. (Caveat: PX4's submodule ships its own
`common.xml`; the golden round-trip test — not wishful thinking — is what
proves the two sides agree. See phase1 doc §6.)

## 4. `hash.sh` — the dialect hash

Definition (also embedded in both output files):

```
CC_DIALECT_SHA256 = SHA-256 over the raw bytes of cc_dialect.xml
CC_DIALECT_HASH   = first 4 digest bytes as a big-endian u32
                    (= first 8 hex chars of the digest)
```

Current values: `dc5b8e9fd57504d18cd905a81282cfdcfccf3e6fd11069c4cc4b44539900e442`
→ `CC_DIALECT_HASH = 0xdc5b8e9f`.

* Emitted to `generated/dialect_hash.h` (C) and `generated/dialect_hash.rs`
  (Rust mirror).
* This is the value the companion sends in `CC_MISSION_CONTEXT.dialect_hash`
  during the session handshake (spec §7); PX4 refuses the mission on
  mismatch (spec §11 "Schema mismatch").
* Any byte change to the XML — comments included — changes the hash. That is
  deliberate: it identifies the exact file bindings were generated from, not
  a semantic fingerprint.
* `crates/cc-protocol/build.rs` **recomputes** the same value independently
  (sha2 crate) at every Rust build; two tests plus the golden
  `CC_MISSION_CONTEXT` frame prove shell-side and Rust-side pipelines agree
  end to end.

## 5. Determinism (what CI's drift check relies on)

Regenerating from an unchanged XML is **byte-identical**. Two fixes were
needed to get there, both encoded in `gen_c.sh`:

| Problem | Fix |
|---|---|
| mavgen embeds `MAVLINK_*_XML_HASH` computed with Python's `hash()`, which is salted per process → every run differed | run mavgen with `PYTHONHASHSEED=0` |
| `version.h` embeds the generation date (`MAVLINK_BUILD_DATE`) | sed-normalize to a fixed string after generation |

Verified by generating twice and `diff -r`-ing the trees. The golden
artifacts (Phase 1) are deterministic the same way — the generator embeds no
timestamps, and NaN payloads use a pinned bit pattern.

Also encoded in the script (learned the hard way, kept for the next person):

* the venv's `mavgen.py` console script is invoked as
  `python <path-to-script>` because (a) its shebang breaks on paths
  containing spaces — this project's absolute path has several — and
  (b) the pip package deprecated `-m pymavlink.generator.mavgen` as an
  executable module.
* `golden/build_golden.sh` is a shell script, not a Makefile: make target
  paths break on spaces too.

## 6. `gen_rust.sh` — Rust bindings, deliberately NOT vendored

rust-mavlink's supported custom-dialect path is **build-time generation**,
and that is what keeps the Rust side drift-proof: `cc-protocol/build.rs`
stages the same four XMLs and runs `mavlink-bindgen` on every build, so
there is no vendored Rust artifact that could go stale. The script:

```sh
./gen_rust.sh --check   # CI: build only (forces binding generation)
./gen_rust.sh           # build + run the full cc-protocol test suite
```

Details of the build.rs pipeline: [cc_protocol_crate.md](cc_protocol_crate.md) §2.

## 7. CI wiring guidance (for your Phase 0.4)

Mapping the dev plan's four jobs onto these scripts:

| Dev-plan job | Command(s) | Fails when |
|---|---|---|
| 1 — drift guard | `cd cc-dialect && ./gen_c.sh && cd golden && ./build_golden.sh && git diff --exit-code` | committed `generated/` or `golden/` differ from fresh regeneration |
| 2 — Rust build+test | `cargo build --workspace && cargo test --workspace` | any test red |
| 3 — PX4 SITL build | (Phase 2+, in the px4-firmware repo) | — |
| 4 — golden round-trip | already inside job 2 (`cc-protocol` tests); `cc-dialect/gen_rust.sh` is the standalone equivalent | wire-format drift between C and Rust |

Notes: job 1 needs `python3` + network for the first venv creation (or cache
`.venv-mavgen`); jobs 2/4 need only Rust + the committed artifacts. Keep the
XML edit → regenerate → commit flow in **one commit** (checklist in
[`cc-dialect/README.md`](../../cc-dialect/README.md)).
