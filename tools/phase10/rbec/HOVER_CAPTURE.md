# Hover-capture flight card (RBEC validation follow-up 3)

One flight session replaces the RBEC simulation's synthetic sway with your
aircraft's *measured* hover motion. No radar aboard, no special airspace —
an ordinary hover flight with richer logging. Total ≈ 15–20 min of flying.

## Before flying — parameters (set once, revert after if you like)

| Param | Value | Why |
|---|---|---|
| `SDLOG_PROFILE` | **17** (= default 1 + high-rate 16) | high-rate attitude/position logging — the vital band (0.8–3 Hz) needs it |
| `SDLOG_MODE` | 0 (default, log from arming) | fine as-is |

SD card: ≥ 1 GB free (high-rate logging runs a few MB/min). If the card is
fast and roomy, `SDLOG_PROFILE = 273` (adds raw IMU FIFO, bit 256) also
captures the rotor-vibration band — nice to have, not required.

## The flights (each item = one continuous, timed segment)

Hold each hover **as steady and as long as the battery allows — 2 min is the
useful minimum per segment, 3–5 min is better** (the sim consumes 30 s
windows; long segments give many independent windows). Note wind conditions
per flight (a phone note is fine: calm / light / gusty + rough m/s if known).

1. **Ground baseline, rotors spinning**: arm, leave it on the ground at idle
   ~60 s. (Separates vibration from sway in the data.)
2. **Loiter hover, ~10 m AGL** — the reference condition. 2–3 segments.
3. **Loiter hover, ~5 m AGL** — 1–2 segments (ground-effect difference).
4. **Position-mode hover, ~10 m** (pilot in loop, sticks centred) — 1
   segment. (Different control spectrum than Loiter.)
5. **Altitude-mode hover, ~10 m** — 1 segment, only if comfortable: no GNSS
   position hold, so it drifts — this is the honest worst case.
6. If a windy day is available later: repeat item 2 in wind — the single
   most valuable extra condition.

Don't chase perfection on station-keeping — the *real* imperfection is
exactly the data.

## Afterwards

Copy the `.ulg` files off the SD card (one per arming), drop them anywhere,
and run (needs `pip install pyulog` once):

```bash
python3 -m tools.phase10.rbec.hover_ingest /path/to/log1.ulg /path/to/log2.ulg
```

It auto-detects hover segments (armed + Loiter/Position + low speed ≥ 40 s;
manual override `--segment FILEIDX:START:END` in seconds), prints per-axis
position RMS and band-RMS (0.05–0.5 / 0.5–3 Hz) plus attitude RMS, and
writes `hover_data.npz`. The simulation then replays the real motion via:

```python
SimConfig(motion_npz="tools/phase10/rbec/hover_data.npz", motion_segment=0)
```

Re-running `exp3`/`exp4` sweeps on the measured trajectories is the point:
it re-derives the T3 coupling, the seam-failure rates and the budget verdict
against your aircraft's actual hover spectrum instead of the synthetic
stand-in. The same npz is the trajectory source for the hexapod HIL replay
(method paper rung V4) later.

Caveat recorded up front: EKF local position is GNSS-fused and smooth, so
the replay is trustworthy below ~1–2 Hz; the rotor band stays covered by the
synthetic vibration lines (raw-FIFO logging, profile bit 256, is the upgrade
path if we ever want measured vibration too).
