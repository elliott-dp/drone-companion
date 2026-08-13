# RBEC numerical validation stack

Numerical validation (rung V1 groundwork) for the method proposed in
[`docs/phase10/radar_rbec_method.md`](../../../docs/phase10/radar_rbec_method.md).
Results and interpretation live in
[`docs/phase10/radar_rbec_validation.md`](../../../docs/phase10/radar_rbec_validation.md)
(exp1–exp5) and
[`docs/phase10/radar_rbec_validation_exp67.md`](../../../docs/phase10/radar_rbec_validation_exp67.md)
(exp6–exp7).

Pure Python + NumPy. Run from the repo root:

```bash
python3 -m tools.phase10.rbec.exp1_sidelobe_mc   # correlated-cal sidelobe MC (~20 s)
python3 -m tools.phase10.rbec.exp2_gdop          # anchor-geometry DOP maps  (<1 s)
python3 -m tools.phase10.rbec.exp3_endtoend      # end-to-end solve sweeps   (~60 s)
python3 -m tools.phase10.rbec.exp4_mitigations   # seam-RAIM + leakage topologies (~35 s)
python3 -m tools.phase10.rbec.exp6_zaxis         # z-aliasing gain: is 3-D the right exp5 upgrade? (~1 s)
python3 -m tools.phase10.rbec.exp7_ghost_anchors # ghost anchors vs robust solves (~6 s)
python3 -m tools.phase10.rbec.exp67_report       # regenerate docs/phase10/results/exp67.{json,png} (~10 s; --check verifies)
```

All experiments are seeded and bit-reproducible. Model boundaries (what these
simulations deliberately do **not** capture — range/beam migration, multipath,
rotation-induced beam-gain modulation, real hover PSDs) are stated in each
module docstring and in the results document; pushing past them is what
validation rungs V2–V6 in the method paper are for.

To feed the simulation **measured** hover motion instead of the synthetic
sway: fly the [HOVER_CAPTURE.md](HOVER_CAPTURE.md) flight card, then
`python3 -m tools.phase10.rbec.hover_ingest <logs...>` (needs `pyulog`) and
pass `SimConfig(motion_npz=...)`.

**Real-cascade-data track** (`coloradar_bridge.py` + `exp5_coloradar.py`):
parses ColoRadar's cascade (4×AWR2243) raw ADC + calibration, runs the cube
chain (calibrate → range FFT → virtual-array beamform), and scores the RBEC
anchor solve against the dataset's ground-truth poses. The bridge is
validated by a synthetic-fixture round-trip
(`python3 -m tools.phase10.rbec.coloradar_bridge`) so it runs the moment
data is on disk — and it has been validated against the **real** cascade
calibration (a public vendored copy of the dataset's calib files; fetch
with `./tools/phase10/rbec/fetch_coloradar_calib.sh <root>`): 86 unique
azimuth virtual positions spanning 0–85, λ = 3.831 mm, 5.93 cm range bins,
coupling matrix (12, 16, 128) parsed and applied.

**Data acquisition status**: only the *sequence* archives remain. The
historical anonymous routes are dead (SharePoint share removed; the 2021
Google Drive links revoked; Radatron distributes heatmaps only, no raw
ADC) — the live channel is the ColoRadar+ Globus collection
(`arpg.colorado.edu/coloradarplus`), which needs a (free) Globus login.
Best target: **`2_24_2021_aspen_run9`** (83 s, and ASPEN sequences carry
**Vicon mm-class ground truth**, far better for scoring the solve than the
unquantified pose-graph GT elsewhere); fallback `ec_hallways_run4` (90 s,
wall-rich). Lay out as `<root>/kitti/<sequence>/`, then:

```bash
python3 -m tools.phase10.rbec.exp5_coloradar <root> 2_24_2021_aspen_run9
```

| Module | Contents |
|---|---|
| `core.py` | Constants (79 GHz, 4π/λ), vital bands, band-RMS (Parseval-checked), shaped hover noise |
| `array_model.py` | 86-element λ/2 virtual ULA, per-chip-correlated calibration errors, tapers (measured SLLs), beam patterns, leakage MC |
| `geometry.py` | Anchor scenes, GDOP/target-DOP, common-mode-rejection factor, condition numbers |
| `endtoend.py` | Chirp-granularity 30 s dwell simulator: sway + vibration lines, anchor angle errors, complex-domain leakage, APLL/chip step trains, IMU-seeded integer unwrap, angle-aware WLS solve |
| `exp1/2/3_*.py` | The runnable experiments |
