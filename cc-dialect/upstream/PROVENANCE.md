# Pinned upstream MAVLink definitions

`cc_dialect.xml` declares `<include>common.xml</include>`, and upstream
`common.xml` pulls in `standard.xml` → `minimal.xml`. Both binding generators
(mavgen for C, mavlink-bindgen for Rust) resolve those includes relative to
the dialect file, so the exact include files are part of the wire contract
(HEARTBEAT, TIMESYNC, etc. live here). They are therefore **pinned in-repo**,
not fetched at build time.

| File | Taken from | SHA-256 |
|---|---|---|
| `common.xml` | pymavlink 2.4.49 (`message_definitions/v1.0/`) | `9af62c02a93c3eac4e068761f9ddf409b5d2aa65cd475f6bd522ca22dc6b2afd` |
| `standard.xml` | pymavlink 2.4.49 (`message_definitions/v1.0/`) | `33876296a0bf118eb773ac31cc8257b4ca75ec42ab0b060044f0e510d66d6ce4` |
| `minimal.xml` | pymavlink 2.4.49 (`message_definitions/v1.0/`) | `bb336f9efb0f772748f69acc33805bbaa64e9f1c09654ef3dda6acab0a454153` |

## Rules

- **Never edit these files.** They are verbatim upstream copies.
- Upgrading them is a **contract change**: regenerate both bindings *and* the
  golden vectors in the same commit (see `../README.md`), and re-run the full
  Phase 1 test suite. Standard-message CRC_EXTRA values can move between
  upstream revisions.
- When upgrading, update this table (source version + hashes) in the same
  commit.
- PX4 has its **own** copy of these files inside its `mavlink/mavlink`
  submodule. The golden-vector round-trip test is what proves the PX4-side
  generation and the Rust-side generation still agree on the wire — do not
  assume the pins match PX4 by construction.
