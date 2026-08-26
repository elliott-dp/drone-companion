# RBEC exp5b — the P1 upgrade pass: harness, fixture proof, and the real-data verdicts

> **Status: complete.** The harness (Parts A–B2) was built, adversarially
> reviewed, and fixture-proven in-sandbox; the real-data runs (Parts E–F,
> author's machine, 2026-08) then delivered the P1 verdict on
> ec_hallways_run4 — **the 555 µm was the wrap-saturation floor, not a
> tracking result** — and, on Vicon-surveyed ASPEN still windows, the
> hover-regime numbers. The D.6 guard rerun (Part F2) then corrected
> Part F's first headline: the 14.5–23.4 µm figures rode on a coupling
> cell acting as a zero-motion regularizer; **the surviving claim is
> held-out 28.6–110 µm across all five windows, 5–19× below the wrap
> floor, robust to the guard choice** — and a new honest negative:
> every anchor-quality gate is structurally biased toward
> platform-fixed returns. This pass implements every item of thesis_plan §4 P1 —
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
* **IMU is a seeding aid, not an INS** *(the naive world-aligned mode
  described here remains the fixture's arm; Part G adds the
  frame-correct rotated modes that real data requires)*. The synthetic accelerometer is the
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

> **Superseded in part by Part F2:** the table below is the guard-4
> (exp5-default near-field guard) result; its "Gain" column did not
> survive the D.6 leakage exclusion. Kept because the *mechanism* of the
> correction is itself a result.

| Window | Baseline plain [µm] | Upgraded plain [µm] | Gain (guard 4 — retracted in F2) |
|---|---|---|---|
| run1 408:458 | 73.7 | 14.5 | 5.1× |
| run2 411:476 | 58.6 | 16.7 | 3.5× |
| run3 533:584 | 63.6 | 23.4 | 2.7× |
| run0 1:42 | 64.3 | 56.1 | 1.1× |
| run10 631:694 | 46.2 | pool collapse (co-range flags 11/19) | — |

* **Non-circular by measurement:** GT seeding fixed 0–0.4 % of integers
  to nonzero wraps in these windows (integer-free regime), so the solve
  is pure radar phase; `consensus_regime_valid` holds on every GT-arm
  dwell; the holdout reads 7–38× below the saturation floor that pegged
  Part E.
* **The P1 prediction at guard 4, as first read:** the co-range
  pre-filter + D_A gate appeared to deliver 3–5×. Part F2 shows the gain
  was contingent on the bin-4 coupling cell; what stands from this table
  is the regime demonstration, not the picker attribution.
* **Consensus as-tuned gives part of it back** (14.5→38.9 µm etc.): at
  ~0.1 mm sway the tolerance sits at its ~21 µm noise floor and
  over-excludes an already-clean pool — the C.5 trade, now measured on
  real data. Tolerance refinement (Part D.5) is the fix; the exclusions
  are availability loss, not error.
* **Two caveats carried honestly at the time:** (1) every upgraded arm
  admitted one bin-4 near-field leakage cell as an anchor — flagged here
  as a bias suspect and confirmed load-bearing by the D.6 rerun (Part
  F2). (2) The IMU arms failed outright (0 % integer agreement, inc-err
  10²–10³ mm): the world-aligned-attitude dead reckoning fails on the
  real attitude profile, as its own honesty note predicted — the
  extrinsics/attitude TODO (D.7) is load-bearing, not cosmetic.

## Part F2 — D.6: the guard rerun corrects the headline **[meas-real]**

`--guard-bins` was added to the harness (default 4 = exp5 behavior;
recorded in every bundle). Measured coupling residue on ASPEN still
frames: +20 dB at bin 4, +14 at bin 5, +5–7 at bin 6, background from
bin 7 — guards 8 (0.47 m) and 17 (1.0 m) were run on all five windows,
both GT arms (20 bundles, `_g8`/`_g17`), plus a matched-holdout
leave-one-out probe (`tools/phase10/rbec/d6_leaveoneout.py`).
**Guard 8 and guard 17 agree to the last bit in every cell** (verified
field-by-field over all ten pairs; only the recorded `guard_bins`
differs) — the entire effect is the single bin-4 cell.

| Window | Baseline plain/cons, g4 → g8 [µm] | Upgraded plain/cons, g4 → g8 [µm] |
|---|---|---|
| run0 | 64.3/65.4 → 52.6/59.9 | 56.1/133.6 → 51.2/67.3 |
| run1 | 73.7/116.4 → 37.4/77.0 | 14.5/38.9 → 89.6/90.7 |
| run2 | 58.6/32.8 → 61.4/28.6 | 16.7/29.6 → 73.3/37.2 |
| run3 | 63.6/74.8 → 65.5/80.8 | 23.4/41.6 → 63.6/33.3 |
| run10 | 46.2/79.5 → 83.2/110.0 | collapse → 82.7/96.6 |

The mechanism, triangulated three ways (run notes, b11e31c): the
matched leave-one-out shows dropping the bin-4 cell moves holdout by
−8.6…+37.3 µm while any real-anchor drop stays within ±10 µm — the
coupling cell (same ~+50° azimuth in every run: the array's own
coupling direction) acted as an **unintentional zero-motion
regularizer**; the cross-arm control shows leakage-as-anchor alone is
not sufficient (the baseline arm carried bin-4 cells too and scored
only 46–74 µm); and the matched ablation reaches only 25–61 µm vs the
full repick's 51–90 µm — the remainder is pool/holdout reshuffle.

Consequences, both quotable:

* **Retraction with mechanism:** the 14.5–23.4 µm figures and the
  "pre-filter 3–5×" attribution do not survive leakage exclusion. The
  deeper finding: **every anchor-quality gate — energy rank, co-range
  uniqueness, D_A, even consensus (which kept bin-4 while excluding four
  real anchors) — is structurally biased toward a platform-fixed
  return**, because zero motion is maximally coherent and maximally
  self-consistent. This joins the C9 honest-negative catalogue beside
  "D_A cannot gate ghosts." On a ghost-free still scene the two pickers
  are statistically indistinguishable at guard 8 (run1 even flips 2.4×
  in the baseline's favor — its co-range front-wall cells were
  redundant good data, not ghosts); the picker's anti-ghost value rests
  on the fixture's ghost scene (Part B) until a real ghost-bearing
  scene is run.
* **The design-regime claim survives, cleaner:** with leakage excluded
  at the source, all five windows read **28.6–110.0 µm** (best-arm),
  5–19× below the 547.8 µm wrap floor, robust to guard choice, run10
  recovered — hover-regime phase consistency at tens of µm on real
  cascade data, integer-free and Vicon-surveyed. The code default is now
  guard 8 (fixture-verified bit-identical; bundle regolded).

## Part G — D.7: attitude-rotated IMU seeding **[meas-real]**

The machinery (author's machine, 2026-08, commit 3dca33e): the bridge
now loads `calib/transforms/*.txt` (vendored by the fetch script;
`base_to_imu` is a ~180° flip about (1,1,0) — the z-down mounting that
silently killed the naive arm in Part F — and `base_to_cascade` a ~90°
yaw plus a 15.3 cm lever arm), and three seed modes join the harness:
`gt-rot` (cascade-point reference: base GT + rotated lever arm,
increments expressed in the cascade frame the steering vectors live in),
`imu-rot` (ZUPT-bias-calibrated open-loop dead reckoning), and
`imu-rot-track` (per-pair velocity re-anchoring). `d7_seed_budget.py`
carries the convention proofs — gravity recovered to +9.79 z after
rotation; an IMU–Vicon clock offset tested by accel cross-correlation
and refuted — and the measured seed budgets. All verified against the
22 committed bundles on this side; cross-platform determinism also
closed here: the Linux `--check` passes against the macOS-regolded
fixture bundle, because `_compare`'s rtol 1e-9 already absorbs the
15th-digit BLAS ULPs (the commit's tolerance caveat needed no code).

**The frame fix is a finding of its own.** On the run1 still window,
`gt-rot` reads 18.1 µm plain / **11.4 µm consensus** vs 89.6/90.7 for
Part F2's base-referenced control — the cascade antenna wobbles on its
15.3 cm lever arm even when the base is "still", and referencing the
right point removes that from the residual. Best real-data holdout on
record, quotable only with its conditions: GT-attitude-aided,
cascade-referenced, still window, guard 8.

**Still windows — the unlock, validated where GT can referee.**
`imu-rot-track` integer agreement vs the GT-seeded fix: **0.989 /
0.983 / 0.984 / 0.951** (runs 1/2/3/10), holdouts within ~2× of the
`gt-rot` floor (12.8 vs 18.1, 55.6 vs 58.1, 113.5 vs 66.8, 97.0 vs
90.0 µm). run0 (0.625 — its sway grazes λ/4; median seed error 248 µm)
is the consensus solve's **first real-data save**: plain 462.5 µm with
37 % wrong integers poisoning LS → 139.2 µm by excluding the
wrong-integer anchors — the designed role, finally observed on real
data.

**Sway windows — not unlocked, and the cause is measured.** Open-loop
agreement 0.005–0.073 (INS physics: the ~4e-3 m/s² post-ZUPT residual
double-integrates past λ/4 within ~1 s); tracked agreement 0.023–0.544
with holdouts at or near the 548 µm wrap floor. The per-pair bridge
error is 248 µm median on still vs 1.5–10 mm on sway — it scales with
motion, attributed to ~0.2–0.6° effective attitude error leaking
gravity (~0.05 m/s²); conventions are proven, bias is ZUPTed, clock
offset is refuted. The same leak term (∝Δt²) extrapolates to **~55 µm
at the flight design's 47 ms inter-burst gap** — ColoRadar's 0.2 s
frame gap, not the mechanism, is the limiter. Two sway windows
(run1 370:420, run3 5:55) pool-collapsed thin and produced no bundles;
recorded here for window-list completeness.

**Qualifications adopted from the run's adversarial review**, kept
verbatim in spirit: `imu-rot-track` as scored is an optimistic
single-gap bound (the carrier is the true previous increment, excluding
the tracker's own ~0.3–0.6 mm solve error), and integer misses are
**absorbing** — one miss injects λ/2 into the next seed, so
0.95⁴⁹–0.99⁴⁹ gives only an 8–61 % chance of an error-free 50-pair
dwell; RAIM-style integer-chain detection is load-bearing, not
optional (D.9). All IMU arms are conditional on externally supplied
attitude; the claim demonstrated is exactly "known attitude + measured
accel suffice", no more. And `int_agreement` is agreement with the
GT-seeded fix, not with truth.

**Verdict:** frame-correct IMU pair-bridging replaces GT integer
seeding on hover-regime data; the reference-point correction alone was
worth up to 5× on still-window residuals; the remaining flight-design
risk is the absorbing integer chain — precisely the RAIM item the plan
already carries.

## Part C — Remaining runs

```bash
# full-hover (non-still) ASPEN segments, e.g. aspen_run9, via the v1 fetcher:
python3 -m tools.phase10.rbec.fetch_coloradar_v1 <root> 2_24_2021_aspen_run9
python3 -m tools.phase10.rbec.exp5b_upgrade --root <root> \
    --sequence 2_24_2021_aspen_run9 --dwell-s 12
```

The attitude-rotated seed (D.7) exists now — Part G shows it unlocks
still windows but not 0.2 s-gap sway segments; full-hover progress runs
through the D.9 integer-chain RAIM and, ultimately, 47 ms-gap data from
the actual payload, not through more ColoRadar reruns.

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
6. ~~Extend the near-field guard~~ **done** (Part F2), and the code
   default is now 8 (fixture-verified: numbers bit-identical, bundle
   regolded with `guard_bins` recorded; pass `--guard-bins 4` to
   reproduce exp5-era behavior). Remaining: a ghost-bearing real scene
   to restore a real-data basis for the picker's anti-ghost claim.
7. ~~Attitude-rotated IMU seed~~ **done** (Part G): still windows seed
   at 95–99 %, sway stays locked by the attitude leak; ~55 µm at the
   design's 47 ms gap by extrapolation of the leak term only.
9. ~~D.9 — integer-chain RAIM~~ **done** (exp9,
   [`radar_rbec_validation_exp9.md`](radar_rbec_validation_exp9.md)):
   built, measured jointly with consensus + seam-RAIM; snaps isolated
   slips (~8/dwell at 300 µm) and keeps the ghost-excluded shared-error
   estimate ~17× cleaner; the raw-residual statistic is the load-bearing
   implementation note. Doctrine: min-N 5/6/9 at the ≤100 µm budget;
   no anchor count rescues dwells above it.
8. Vitals-bank D_A note: D_A under *walking* decorrelates real anchors
   (Part E) — the gate's validity is regime-dependent; document in C.4.
