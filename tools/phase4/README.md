# tools/phase4 — companiond vs SITL (the real RX path)

Companion tooling for **dev plan Phase 4**. Full design + results:
[../../docs/phase4/phase4_companiond.md](../../docs/phase4/phase4_companiond.md).

| File | Purpose |
|---|---|
| `sitl_phase4_check.py` | integration + fault-drill suite: boots headless SIH SITL and the release `companiond --status-json`; asserts link-up, timesync LOCKED ≤ 5 s, all six stream rates ±20%, zero gaps/CRC; drills garbage-injection, telemetry pause (watchdogs), FC reboot (boot-id + relock). `--soak N` keeps it running N seconds and asserts clean counters end-to-end (dev-plan exit: 3600). |
| `last_run.log`, `px4_server_*.log` | committed evidence of the most recent run |

```sh
cargo build --release -p companiond
./sitl_phase4_check.py                 # checks + drills (~3 min)
./sitl_phase4_check.py --soak 3600     # + the 1 h exit-criterion soak
```

## companiond `--status-json` schema (one object per line, 1 Hz)

```json
{"t_ns": 0,                       // companiond monotonic clock, ns
 "link": "UP|DEGRADED|DOWN",
 "fc_hb_age_ms": 200,             // null until the first FC heartbeat
 "px4_boot_id": 123, "mission_id": 0,
 "timesync": {"q": "LOCKED|DEGRADED|UNLOCKED", "offset_ns": 0,
               "rtt_us": 0, "window": 32, "rejected": 0},
 "streams": {"state|imu|power|gps|estimator|actuator|event|safety_status":
             {"n": 0, "hz": 0.0, "gaps": 0, "stale": false}},
 "counters": {"frames_ok": 0, "crc_errors": 0, "garbage_bytes": 0,
               "unknown_msg": 0, "bad_payloads": 0, "bad_source": 0,
               "bad_schema": 0, "tx_frames": 0, "tx_errors": 0,
               "p0_stalls": 0, "rx_drops": 0}}
```

The schema is hand-emitted by companiond (no serde); treat it as an
interface — the harness and any future tooling parse it. Rates in the
harness are computed from `n` deltas against `t_ns` deltas, so sim-speed
and scheduling jitter cancel out.
