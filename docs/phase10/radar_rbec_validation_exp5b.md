# RBEC exp5b — the P1 upgrade pass: harness, fixture proof, and the real-data verdicts

> **Status: complete.** The harness (Parts A–B2) was built, adversarially
> reviewed, and fixture-proven in-sandbox; the real-data runs (Parts E–F,
> author's machine, 2026-08) then delivered the P1 verdict on
> ec_hallways_run4 — **the 555 µm was the wrap-saturation floor, not a
> tracking result** — and, on Vicon-surveyed ASPEN still windows, the
> first hover-regime numbers: **held-out 14.5–23.4 µm, with the pre-filter
> + D_A gate attributed at 3–5×.** This pass implements every item of thesis_plan §4 P1 —
> per-dwell α report, co-range structural pre-filter, subset-consensus
> solve, IMU-seeded integers, full-sequence dwell processing — as
> `tools/phase10/rbec/exp5b_upgrade.py`, against the specs in
> [`radar_rbec_validation.md`](radar_rbec_validation.md) §F.4 and
> [`radar_rbec_validation_exp67.md`](radar_rbec_validation_exp67.md)
> §B.4/§C.4/§D. Parts B–B2 are **[meas]** on the synthetic fixture
> (committed bundle `results/exp5b.json`, regenerated exactly by
> `--check`); Parts E–F are **[meas-real]** on ColoRadar v1 data and
> **supersede F.4's interpretation**: the 555 µm and the 0.23/0.35 mm
> increment agreement are re-read as saturation-floor and seed-circular
> artifacts of the walking regime (Part E), replaced by the hover-regime
> numbers of Part F.

## Part A — Models and their deliberate boundaries

* **The fixture, not the hallway.** ec_hallways_run4 could not be obtained
  from the sandbox: ColoRadar+ is Globus-only (interactive login) and the
  v1 hosts are proxy-blocked there. (Correction from the real-data pass:
  the v1 OneDrive share IS alive from a browser, no login —
  `fetch_coloradar_v1.py` now pulls windowed cascade frames + vicon/imu
  from it via HTTP Range requests, ~1 GB instead of ~50 GB.) The
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

## Part E — Real data I: ec_hallways_run4 says no, and says why **[meas-real]**

Both arms ran on the author's machine over the GT-covered local frames
(72:299, walking motion ~126 mm/frame ≈ 66 half-wavelengths per frame;
bundles `results/exp5b_ec_hallways_run4_baseline_{gt,imu}_72-299.json`).

* **The holdout statistic pegs the saturation floor, and F.4's number
  dissolves.** Held-out residual 543–556 µm across all four arms — GT- or
  IMU-seeded, either picker — against the analytic wrapped-uniform floor
  π/√3 / K = 547.8 µm. The seeding doesn't matter (IMU arms fixed 0 % of
  integers to the GT values and read the *same* holdout), so the metric is
  measuring phase-wrap saturation, not tracking. Direct anchor probes
  agree: the real wall cells' wrapped Δphase std sits at 1.76/1.86 rad vs
  the 1.814 saturation constant. **F.4's "555 µm held-out" is therefore
  re-tiered: it was the floor of a saturated statistic at walking pace,
  not evidence of sub-mm tracking.** The GT-arm's 0.20–0.32 mm increment
  "agreement" is seed-circular in this regime (integers carry the GT;
  the fractional phase underneath is aliased or near-field leakage).
* **The upgraded picker refused to run — correctly.** Its pool collapsed:
  12/20 candidates sit in range bin 4 (0.24 m — cascade coupling residue
  just past the `emap[:, :4]` near-field guard), and the four real wall
  cells fail D_A (1.3–2.0 vs 0.25) under walking decorrelation. Nothing
  in the scene passes an honest gate at this platform speed.
* **`consensus_regime_valid` = False on every dwell**, exactly as the
  B2.1 clamp predicted for this regime; the consensus exclusions there
  (set-size 3, worse inc-err than plain) are noise and are not quoted.

Verdict: at walking speed the sequence is outside RBEC's hover design
regime, and the harness's own statistics say so instead of flattering it.

## Part F — Real data II: ASPEN still windows — the hover-regime result **[meas-real]**

A Vicon survey (98 Hz mm-class mocap, preferred automatically by the
bridge) of all twelve v1 aspen runs found five ≥40-frame still stretches
(median 95–124 µm/frame — inside λ/4, the design regime). One 12 s dwell
each, GT-seeded, both pickers (bundles
`results/exp5b_2_24_2021_aspen_run*_*.json`):

| Window | Baseline plain [µm] | Upgraded plain [µm] | Gain |
|---|---|---|---|
| run1 408:458 | 73.7 | **14.5** | 5.1× |
| run2 411:476 | 58.6 | **16.7** | 3.5× |
| run3 533:584 | 63.6 | **23.4** | 2.7× |
| run0 1:42 | 64.3 | 56.1 | 1.1× |
| run10 631:694 | 46.2 | pool collapse (co-range flags 11/19 — that viewpoint's wall arc is genuinely equidistant) | — |

* **Non-circular by measurement:** GT seeding fixed 0–0.4 % of integers
  to nonzero wraps in these windows (integer-free regime), so the solve
  is pure radar phase; `consensus_regime_valid` holds on every GT-arm
  dwell; the holdout reads 7–38× below the saturation floor that pegged
  Part E.
* **The P1 prediction, settled with attribution:** the co-range
  pre-filter + D_A gate deliver the 3–5×; the identical-range clusters
  *were* structural (upgraded anchors occupy distinct bins vs the
  baseline's co-range triples plus 2–4 bin-4 leakage cells).
* **Consensus as-tuned gives part of it back** (14.5→38.9 µm etc.): at
  ~0.1 mm sway the tolerance sits at its ~21 µm noise floor and
  over-excludes an already-clean pool — the C.5 trade, now measured on
  real data. Tolerance refinement (Part D.5) is the fix; the exclusions
  are availability loss, not error.
* **Two caveats carried honestly:** (1) every upgraded arm still admits
  one bin-4 near-field leakage cell as an anchor (platform-fixed, D_A ≈
  0.001 — the stability gate *likes* it); a platform-fixed cell votes
  "zero motion" on its LOS, which plausibly contributes to solve RMS
  reading below Vicon RMS in these windows. Extend the near-field guard
  and re-run (Part D.6). (2) The IMU arms failed outright
  (0 % integer agreement, inc-err 10²–10³ mm): the world-aligned-attitude
  dead reckoning fails on the real handheld/hover attitude profile, as
  its own honesty note predicted — the extrinsics/attitude TODO is now
  load-bearing, not cosmetic.

## Part C — Remaining runs

```bash
# full-hover (non-still) ASPEN segments, e.g. aspen_run9, via the v1 fetcher:
python3 -m tools.phase10.rbec.fetch_coloradar_v1 <root> 2_24_2021_aspen_run9
python3 -m tools.phase10.rbec.exp5b_upgrade --root <root> \
    --sequence 2_24_2021_aspen_run9 --dwell-s 12
```

Full-hover segments need the attitude-rotated IMU seed (D.7) first —
per-frame motion there exceeds λ/4, and Part E shows what GT-seeded
statistics are worth in that regime.

## Part D — Follow-ups

1. ~~Real-data rerun~~ **done** (Parts E–F). Remaining P1-adjacent items
   below.
2. Stride-reduced consensus scoring for the saturated regime (B2.1) —
   still open, though Part E suggests walking-pace data is better simply
   ruled out of regime than rescued.
3. **α on all five real windows reads p95 ≈ 0.037–0.043 — above the 0.02
   gate everywhere** (bound-only, ±5° el). Real-data confirmation of the
   fixture's negative; P2 must propagate per-anchor elevation before any
   2-D certification claim.
4. (unchanged) P2 α-uncertainty propagation.
5. Tolerance refinement — now urgent, not cosmetic: the C.5 over-exclusion
   is measured on real data (Part F); per-anchor measured-SNR σ_φ + an
   el-leak term.
6. **Extend the near-field guard** past bin 4 (the cascade coupling
   residue survives it and gets admitted as an ultra-stable anchor);
   re-run the ASPEN windows and check the solve-vs-Vicon correlation.
7. **Attitude-rotated IMU seed** (extrinsics + quaternion rotation) — the
   world-aligned assumption is measured broken (Part F caveat 2); needed
   for any beyond-still-window hover work.
8. Vitals-bank D_A note: D_A under *walking* decorrelates real anchors
   (Part E) — the gate's validity is regime-dependent; document in C.4.
