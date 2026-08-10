# RBEC numerical validation stack

Numerical validation (rung V1 groundwork) for the method proposed in
[`docs/phase10/radar_rbec_method.md`](../../../docs/phase10/radar_rbec_method.md).
Results and interpretation live in
[`docs/phase10/radar_rbec_validation.md`](../../../docs/phase10/radar_rbec_validation.md).

Pure Python + NumPy. Run from the repo root:

```bash
python3 -m tools.phase10.rbec.exp1_sidelobe_mc   # correlated-cal sidelobe MC (~20 s)
python3 -m tools.phase10.rbec.exp2_gdop          # anchor-geometry DOP maps  (<1 s)
python3 -m tools.phase10.rbec.exp3_endtoend      # end-to-end solve sweeps   (~60 s)
python3 -m tools.phase10.rbec.exp4_mitigations   # seam-RAIM + leakage topologies (~35 s)
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

| Module | Contents |
|---|---|
| `core.py` | Constants (79 GHz, 4π/λ), vital bands, band-RMS (Parseval-checked), shaped hover noise |
| `array_model.py` | 86-element λ/2 virtual ULA, per-chip-correlated calibration errors, tapers (measured SLLs), beam patterns, leakage MC |
| `geometry.py` | Anchor scenes, GDOP/target-DOP, common-mode-rejection factor, condition numbers |
| `endtoend.py` | Chirp-granularity 30 s dwell simulator: sway + vibration lines, anchor angle errors, complex-domain leakage, APLL/chip step trains, IMU-seeded integer unwrap, angle-aware WLS solve |
| `exp1/2/3_*.py` | The runnable experiments |
