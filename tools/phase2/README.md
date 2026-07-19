# tools/phase2 — PX4-side build + SITL verification

Companion tooling for **dev plan Phase 2** (uORB topics +
`cc_telemetry_publisher` in the `PX4-Autopilot-CCFC` fork, pinned v1.17.0).
Full design + results: [../../docs/phase2/phase2_px4_telemetry.md](../../docs/phase2/phase2_px4_telemetry.md).

| File | Purpose |
|---|---|
| `build_px4.sh` | reproducible `px4_sitl_default` build of the fork (pinned python venv, selective submodules, gz/protobuf skips — see doc §A.5). Optional arg: PX4 tree path. |
| `sitl_phase2_check.py` | the Phase 2 test suite: headless SIH SITL driven over a pty-pxh session; asserts stream rates (±20 %, from sample timestamps), sequence monotonicity, boot-id constancy, live param changes, MINIMAL silence, in-flight value plausibility, `print_status`, work-queue placement. Exit 0 = all green. |
| `last_run.log` | harness log of the most recent run (committed evidence for the doc) |
| `px4_server_last.log` | full pxh transcript of the most recent run |

Typical cycle after touching the PX4-side code:

```sh
./build_px4.sh && ./sitl_phase2_check.py
```

Both scripts locate the fork at `../../../../PX4-Autopilot-CCFC` by default.
Read the header comment of `sitl_phase2_check.py` before "simplifying" its
I/O: the pty transport and one-command-at-a-time protocol work around real
PX4 shell behaviors (stdin-polling `listener`, stdout buffering) that
silently break pipe-based automation.
