# tools/phase5 — mission dataset vs SITL (crash-safe logging)

Companion tooling for **dev-plan Phase 5**. Full design + results:
[../../docs/phase5/phase5_mission_log.md](../../docs/phase5/phase5_mission_log.md).

| File | Purpose |
|---|---|
| `sitl_phase5_check.py` | boots headless SIH SITL + the release `companiond`, then verifies the mission dataset with `log-inspect`: a clean mission reads **CLEAN**; a `kill -9` mid-mission reads **DIRTY** with the sealed parts still readable, and a restart **resumes the same mission_id** with a new segment; forcing the shed thresholds above free space trips the shedding ladder (raw → imu/actuator → …, State/Event/Safety never shed) with a queryable drop ledger. `--soak N` runs one N-second mission and asserts `log-inspect` **CLEAN** with per-stream row counts in range (dev-plan exit: 3600). |
| `last_run.log`, `px4_server_*.log` | committed evidence of the most recent run |

```sh
cargo build --release -p companiond -p log-inspect
./sitl_phase5_check.py                        # clean + crash + disk-full drills (~1 min)
./sitl_phase5_check.py --soak 3600            # + the 1 h exit-criterion mission
./sitl_phase5_check.py --skip-crash --skip-diskfull --soak 3600   # soak only
```

Flags: `--px4 DIR` (fork path), `--keep` (leave the rootfs + mission dirs),
`--skip-crash`, `--skip-diskfull`. `mission_root` defaults to
`$CCFC_SCRATCH` (a volume with room above the 5 GiB floor).

## The mission dataset (what `companiond` writes)

```text
<mission_root>/
  cc_boot_seq  mission_seq                 persisted monotonic id counters
  mission_000001/
    manifest.json                          provenance + per-segment/-stream rollup
    px4_params_snapshot.json               PX4 param snapshot (stub in Phase 5)
    segment_00/                            (cc_boot_id, px4_boot_id)
      state/  000000.parquet 000001.parquet …   one row group per part file
      imu/ power/ gps/ estimator/ actuator/ event/ safety_status/
      events/  000000.parquet …            drop / shed / lifecycle ledger
      raw_mavlink.bin                      length-prefixed pre-decode wire capture
    segment_01/ …                          new segment on restart / PX4 reboot / cap
```

Crash-safety crux: **one flush = one complete Parquet part**
(`ArrowWriter` footer → fsync file → atomic rename → fsync dir). Every sealed
`NNNNNN.parquet` is readable by a stock reader after `kill -9`; bounded loss is
the in-memory buffer (`flush_rows` / `flush_secs`). `log-inspect` recomputes
the authoritative rollup from the part footers and treats the manifest as
advisory (three-state verdict: Clean / Dirty / Corrupt → exit 0 / 1 / 2).

## `companiond --status-json` — the new `log` object

The Phase 4 schema (link / timesync / streams / counters) gains one object:

```json
"log": {"shed_stage": "NORMAL|SHED_RAW|SHED_BF|SHED_CRIT", "warn": false,
        "drops": 0, "raw_drops": 0, "parts": 0, "write_errors": 0,
        "lagged": 0, "free_mib": 0}
```

`lagged` counts telemetry events the (lossy) log subscriber missed — logging
never back-pressures the RX path (spec §5.2).
