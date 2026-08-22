# RBEC exp5b — the P1 upgrade harness, built, adversarially reviewed, fixture-proven

> **Status: P1's machinery is implemented, survived an adversarial review
> pass, and is validated end-to-end on a seeded synthetic fixture; the
> real-data rerun is one command away, blocked only on the sequence
> archive.** This pass implements every item of thesis_plan §4 P1 —
> per-dwell α report, co-range structural pre-filter, subset-consensus
> solve, IMU-seeded integers, full-sequence dwell processing — as
> `tools/phase10/rbec/exp5b_upgrade.py`, against the specs in
> [`radar_rbec_validation.md`](radar_rbec_validation.md) §F.4 and
> [`radar_rbec_validation_exp67.md`](radar_rbec_validation_exp67.md)
> §B.4/§C.4/§D. Everything measured here is **[meas]** on the synthetic
> fixture (committed bundle `results/exp5b.json`, regenerated exactly by
> `--check`); **no number here supersedes F.4's** — the 0.23/0.35 mm /
> 555 µm real-data results stand untouched until the rerun.

## Part A — Models and their deliberate boundaries

* **The fixture, not the hallway.** ec_hallways_run4 could not be obtained
  in this environment: ColoRadar+ is Globus-only (interactive login), the
  v1 SharePoint/Drive routes are gone, and no public mirror of the raw ADC
  exists (checked: GitHub code/LFS/releases, reachable S3/GCS hosts). The
  fixture is a 10-anchor hallway-like scene written in the ColoRadar
  on-disk layout — 3,145,728-byte frames, the vendored calib's waveform
  constants, **1-indexed frame filenames as ColoRadar+ ships them** (so
  the bridge's autodetect branch the real rerun will take is exercised) —
  processed by the same bridge code paths a real sequence would take. It
  validates the *machinery*; it neither replaces nor predicts the
  real-data numbers.
* **Ghost model = exp7's mis-attribution, realized in raw ADC.** Two
  injected scatterers carry a believed direction (their steering vector)
  while their range-phase follows the *parent* anchor's LOS. G1 sits in
  the parent's exact range bin (the pre-filter's catch); G2 three bins
  away and strong enough to rank inside the anchor set (the consensus
  solve's catch). Boundary stated plainly: because parent and ghosts share
  one phase modulation by construction, window/beam leakage among them is
  phase-transparent — leakage robustness on co-located returns that move
  *differently* is not probed here (it is T8's bench question).
* **IMU is a seeding aid, not an INS.** The synthetic accelerometer is the
  true second derivative of band-limited (0.12–2.2 Hz) sway plus bias and
  noise; dead-reckoning assumes world-aligned attitude and takes the
  dwell-start velocity from a GT central difference. The measured
  integer-agreement statistic therefore bundles v0 finite-difference
  error (the dominant term at radar-rate GT), integrator error, bias and
  noise — the seeding error budget of the method *as run*, not an isolated
  IMU spec. The real archive's `imu/` layout is encoded to the dev-kit
  convention and must be re-verified when data lands; on an absent,
  malformed, or non-covering log the run falls back to GT seeding and says
  so (`int_seed_mode` stays `"gt"`).
* **α with unmeasured elevations is a bound, not a value.** With all-zero
  elevations the nominal α degenerates to a constant, so on azimuth-only
  data the per-dwell report carries *only* the seeded p95 |α| bound over
  ±5° anchor elevations (target elevation held nominal — its uncertainty
  is P2's question). A nominal α is reported only when elevations are
  actually known.

## Part B — What the fixture run measures

Committed bundle `results/exp5b.json` (figure `fig_rbec_exp5b.png`);
regenerate + verify with

```bash
python3 -m tools.phase10.rbec.exp5b_upgrade --self-test
python3 -m tools.phase10.rbec.exp5b_upgrade --check
```

| Quantity | Dwell 0 | Dwell 1 |
|---|---|---|
| Baseline (exp5-equivalent picker, ghosts admitted), plain [µm] | 133.8 (mean) | — |
| Pre-filtered, plain → consensus [µm] | 73.1 → 69.1 | 97.6 → 73.6 |
| Consensus exclusion (identity committed) | az −15.0° = G2 | az −15.0° = G2 |
| Consensus set size / regime valid / tol [µm] | 8 / yes / 121 | 8 / yes / 135 |
| Increment error vs truth, consensus [mm RMS x/y] | 0.029/0.007 | 0.031/0.009 |
| IMU-seeded integer agreement vs GT-seeded | 97.2 % | 99.3 % |
| α p95 over ±5° el bound (nominal: n/a, el unknown) | 0.036 | 0.039 |

* **The P1 prediction's mechanism, decomposed.** The `--baseline` arm
  (co-range pre-filter and D_A gate off — the exp5-equivalent picker)
  admits both ghosts: held-out residual 133.8 µm plain. The pre-filter
  alone brings it to 85.3 µm (mean); consensus takes it to 71.3 µm — and
  its own exclusion in the baseline arm (both ghosts dropped, [2, 2])
  independently rescues 75.1 µm. On the real sequence the same
  baseline-vs-upgraded comparison *is* the test of the on-record
  prediction, with attribution.
* **Exclusion by identity, committed.** `consensus_excluded_az_deg` pins
  *which* anchor was dropped (G2, both dwells; no good anchor lost), and
  the self-test asserts the identity, not a count.
* **The C.5 tolerance trade, measured.** With the matched-noise tolerance
  (σ_θ = 0.1°) the ghost is still excluded but 3 good anchors fall out
  with it: the 2-D model's elevation leak sin(el)·dz (up to ~90 µm at ±5°
  el and mm-class z sway) is real residual the tolerance must absorb. The
  shipped grid-quantization tolerance does so at hallway scales; a
  per-anchor σ_φ/el-aware tolerance is the follow-up.
* **IMU seeding is viable at fixture error levels:** ≥ 97 % integer
  agreement per 4.8 s dwell (93.5 % over a stressed 9.6 s dwell).
* **An informative α negative (unchanged by review):** with 9
  azimuth-spread anchors and only a ±5° bound on elevations, p95 |α| ≈
  0.037–0.039 — above the 0.02 gate. Azimuth geometry alone cannot certify
  the 2-D solve when anchor elevations are unknown at that bound;
  certification needs per-anchor elevation knowledge or a tighter bound —
  the quantity P2 must propagate. (The fixture's *true* drawn geometry
  gives α = −0.019, under the gate — consistent with exp67's single-draw
  hallway α = +0.006.)
* **Exactness retained:** the noise-free drop-z error equals α·K·dz to
  < 1e-9 (exp6's law), and the fixture's phase-displacement gain now
  matches the solver's mid-sweep K exactly (geometric phase written at the
  start frequency; the range FFT's window-centroid term supplies the
  remaining half-sweep).

## Part B2 — What the adversarial review changed (kept as honest negatives)

An independent five-lens review pass (consensus-port fidelity, fixture
physics, α/doc consistency, indexing + untested paths, fresh-eyes
reproduction) refuted the first cut in ways the fixture alone never would
have. The substantive corrections, all now in the code and re-validated:

1. **Wrap-aware consensus tolerance (was a BLOCKER).** Integer fixing caps
   every pair residual near λ/4 of the seeded prediction, while the ported
   tolerance grows with motion through T3 — beyond ~0.1 m/s it would cross
   the statistic's ceiling and admit everything, i.e. exactly on
   walking-pace ec_hallways the solve would have reported `set_size = N`
   vacuously. The tolerance is now clamped at 0.5·λ/4 and every dwell
   carries `consensus_regime_valid`; when false, per-pair discrimination
   is saturated (σ_θ·d_pair approaching λ/4) and the honest fix is C.1
   stride/chirp-rate processing, not a bigger tolerance. **Consequence for
   the rerun:** at 5 Hz walking pace the flag will likely read false —
   plan the rerun's consensus scoring on reduced stride, or read its
   verdicts as degraded.
2. **Full-RMS agreement statistic.** exp7's mean-removed std is blind, in
   the pair domain, to the DC residual a mis-attributed anchor produces
   under sustained velocity (differencing turns exp7's ramp signature into
   exactly that DC). RMS restores the sensitivity; legitimate because
   differencing already removed the static per-anchor bias that motivated
   exp7's mean-removal. A pure-estimator self-test injects a
   constant-velocity ghost and asserts RMS catches what std would not.
3. **Fixture phase constant** (1.6 % gain systematic, was dominating the
   committed "accuracy") and **fixture motion synthesis** (piecewise-linear
   positions gave the synthetic IMU a flat-to-Nyquist acceleration
   spectrum) — both corrected; increment errors are now noise-limited
   (~30 µm) and the IMU metric measures the seeding budget, not
   integrator artifacts.
4. **Fault tolerance on the real path:** a thin dwell no longer aborts a
   2192-frame run (recorded per dwell, run continues); an IMU log that
   fails to cover the radar window falls back to GT seeding *visibly*
   instead of fabricating 100 % agreement; malformed IMU files degrade
   instead of crashing; short frame windows raise instead of emitting NaN
   means; the dropped tail is counted (`frames_dropped_tail`).

## Part C — The rerun, when data lands

```bash
./tools/phase10/rbec/fetch_coloradar_calib.sh <root>       # calib: works now
# place kitti/ec_hallways_run4/ under <root> (Globus: arpg.colorado.edu/coloradarplus)
python3 -m tools.phase10.rbec.exp5b_upgrade --root <root> \
    --sequence ec_hallways_run4 --baseline          # exp5-equivalent arm
python3 -m tools.phase10.rbec.exp5b_upgrade --root <root> \
    --sequence ec_hallways_run4 --seed-mode imu     # upgraded arm
```

Each writes `results/exp5b_<sequence>_<mode>….json` + figure over the full
2192 frames in 30 s dwells. The baseline-vs-upgraded holdout comparison
settles the prediction with attribution; watch `consensus_regime_valid`
per dwell (Part B2.1) before quoting consensus verdicts.

## Part D — Follow-ups

1. Real-data rerun (above) — the only remaining P1 item, blocked on the
   archive. Highest-value user action alongside P4/P5 (thesis_plan §4).
2. Stride-reduced consensus scoring for the saturated regime (B2.1): score
   agreement on k-frame strides sized so σ_θ·d_stride ≪ λ/4, keeping the
   solve per-pair. Needed before consensus verdicts on walking-pace data
   are quotable.
3. Extrinsics: `gt_increments`/`imu_increments` still use GT world axes
   (exp5's documented TODO); the mm3DGS mirror also vendors
   `transforms/base_to_{cascade,imu}.txt` — wire before the rerun if
   increment-axis fidelity matters beyond magnitude scoring.
4. The α-bound negative feeds P2 directly: propagate per-anchor elevation
   uncertainty instead of a uniform bound.
5. Tolerance refinement (B's C.5 trade): per-anchor measured-SNR σ_φ and
   an el-leak term sin(el_bound)·|dz| in the consensus variance model.
