# tools/phase3 — MAVLink link verification (both directions)

Companion tooling for **dev plan Phase 3** (CC_* streams out of PX4, the
receiver validation gauntlet into PX4). Full design + results:
[../../docs/phase3/phase3_mavlink_link.md](../../docs/phase3/phase3_mavlink_link.md).

| File | Purpose |
|---|---|
| `gen_python.sh` | generates pymavlink bindings for the dialect from the pinned toolchain (pymavlink 2.4.49 in `cc-dialect/.venv-mavgen`) into `generated/cc_dialect.py` — scaffolding, never vendored (gitignored) |
| `sitl_phase3_check.py` | the Phase 3 test suite: headless SIH SITL + UDP `udpin` on the CCFC companion-link port (24040); asserts every FC→CC stream decodes externally at profile rate, and drives every CC→FC gauntlet fault class against `mavlink status` counter deltas. Exit 0 = all green. |
| `last_run.log` / `px4_server_last.log` | committed evidence of the most recent run |

Typical cycle after touching PX4-side Phase 3 code:

```sh
../phase2/build_px4.sh && ./sitl_phase3_check.py
```

Shared pxh/pty plumbing lives in [`../common/pxh.py`](../common/pxh.py) —
its header documents the PX4 shell behaviors it works around; read it
before modifying transport code. The Python bindings self-verify against
the Phase 1 golden vectors on every run (16/16 frames must decode) before
any SITL traffic is trusted.
