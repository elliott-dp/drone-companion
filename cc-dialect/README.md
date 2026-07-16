# cc-dialect — the wire contract

`cc_dialect.xml` is the **single source of truth** for everything that
crosses the FC↔CC link (spec §3). Both language bindings and the golden
test vectors are generated from it; nothing in this directory is edited by
hand except the XML itself.

```
cc_dialect.xml        THE contract (message IDs 54000–54099, includes common.xml)
gen_c.sh              regenerate + vendor the C library into generated/c/
gen_rust.sh           prove the Rust build-time generation + run the test suite
hash.sh               regenerate generated/dialect_hash.{h,rs}
requirements.txt      pinned mavgen toolchain (pymavlink==2.4.49, lxml==6.0.2)
upstream/             pinned common/standard/minimal.xml (see PROVENANCE.md)
generated/            COMMITTED outputs (C library + dialect hash constants)
golden/               COMMITTED golden vectors + their C generator
  ├── gen_golden.c        encodes 16 fixed frames with the C (mavgen) encoder
  ├── build_golden.sh     compile + run → golden_frames.bin, golden_manifest.txt
  ├── golden_frames.bin   the byte-level contract (749 bytes, 16 frames)
  └── golden_manifest.txt frame table incl. per-message CRC_EXTRA
.venv-mavgen/, build/  local scratch (gitignored)
```

Docs: [toolchain details](../docs/phase0_dialect_toolchain.md) ·
[golden mechanism & fuzz suite](../docs/phase1_protocol_layer.md) ·
[Rust wrapper crate](../docs/cc_protocol_crate.md)

---

## Rules (from spec §3 — enforced by CI and the golden tests)

1. Message IDs are **never reused or renumbered**; deprecated messages keep
   their ID forever. 54008 stays reserved for a future `CC_TELEMETRY_ESC`.
2. New fields go in `<extensions>` sections only (MAVLink 2 zero-truncation
   keeps old parsers compatible). **Semantic** changes bump
   `schema_version` (and the Rust `identity::CC_SCHEMA_VERSION`).
3. Every payload stays ≤ 253 bytes — one MAVLink 2 frame. Anything larger
   toward PX4 is a design error by definition (spec §3.6).
4. Both bindings and the golden vectors are regenerated from the **same
   commit** of this XML. CRC_EXTRA divergence does not error — it silently
   kills messages. The golden round-trip test exists to make it loud.
5. `upstream/*.xml` are verbatim pinned copies — never edit; upgrading them
   is a contract change (rules in `upstream/PROVENANCE.md`).

## The change workflow (one commit, always)

```sh
# 1. edit cc_dialect.xml  (respect rules 1–3)
./gen_c.sh                       # revendor C library + refresh dialect hash
(cd golden && ./build_golden.sh) # rebuild golden vectors with the C encoder
#    ...update the mirrored fixed values in gen_golden.c AND
#    crates/cc-protocol/tests/golden_roundtrip.rs if fields changed...
(cd .. && cargo test --workspace)  # Rust regenerates its bindings; all green?
git add -A && git commit           # XML + generated/ + golden/ + tests together
```

If `cargo test` fails on `golden_roundtrip` right after an XML edit, that is
the system working: one of the three mirrors (XML, C generator, Rust test)
didn't get the matching update.

## First-time setup on a new machine

Needs: python3, a C compiler, Rust. Network is used once to create
`.venv-mavgen` from the pinned `requirements.txt`.

```sh
./gen_c.sh && (cd golden && ./build_golden.sh) && ./gen_rust.sh
```

All three must end green with **zero diffs** under `generated/` and
`golden/` — if regeneration changes committed files on an untouched
checkout, a toolchain drifted (see the determinism notes in
[phase0_dialect_toolchain.md](../docs/phase0_dialect_toolchain.md) §5).
