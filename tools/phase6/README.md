# tools/phase6 — the companion safety loop vs SITL

Companion tooling for **dev-plan Phase 6**. Full design + results:
[../../docs/phase6/phase6_safety_loop.md](../../docs/phase6/phase6_safety_loop.md).

| File | Purpose |
|---|---|
| `sitl_phase6_check.py` | boots headless SITL + the release `companiond`, drives the FC `cc_safety_monitor` with scripted CC_HEALTH_REPORTs (`cc-health-tx --health-scenario`), and asserts the monitor's response from companiond's `--status-json` `safety` object. 12/12: nominal→OK, escalation→CRITICAL/BLOCK_OFFBOARD, ACK under the 5 Hz repeat, OK_COUNT recovery, garbage/flood immunity, staleness+recovery, disabled path. |
| `last_run.log` | committed evidence of the most recent run |

```sh
cargo build --release -p companiond
./sitl_phase6_check.py        # ~1 min
```

## The safety decision core is host-tested (exit criterion)

The pure policy table + state machine live in the fork and build/run on the
host with no PX4 tree:

```sh
cd <PX4-Autopilot-CCFC>/src/modules/cc_safety_monitor
c++ -std=c++14 -I. cc_policy_table_test.cpp -o /tmp/t && /tmp/t   # 40/40
```

## `companiond --status-json` — the new `safety` object

```json
"safety": {"state": 0..4, "action": 0..5, "reject": 0..6, "ack_seq": 0, "seen": false}
```

`state` = CC_COMPANION_STATE (0 UNKNOWN … 4 STALE), `action` =
CC_RECOMMENDED_ACTION the monitor executed, `reject` = CC_REJECT_REASON,
`ack_seq` = the monitor's `last_report_sequence` (the ACK). This is the FC
`cc_safety_monitor`'s `CC_SAFETY_STATUS` echoed back to the companion.
