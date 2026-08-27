# RBEC exp10 (thesis P5) — measured hover replaces the last synthetic input

> **Headline: on measured hover motion the cardiac budget's margin grows
> from 1.16× to 2.2–3.3× — and the record gains a measured envelope
> requirement: RBEC dwells need altitude-hold drift at the ~1 cm/s
> level.** exp3/exp4's four decision slices rerun with the synthetic
> 0.3 Hz-knee sway model replaced by real EKF hover trajectories (module
> `tools/phase10/rbec/exp10_hover_measured.py`, replay source
> `results/hover_p5.npz`, bundle `results/exp10.json`), each slice paired
> with its synthetic twin. The seam cliff proves motion-invariant,
> confirming exp4's attribution on measured motion. Author's run
> (2026-08); independently re-verified in-sandbox — self-test green,
> `--check` exact on 967 of 974 bundle lines with the 7 exceptions
> confined to one integer-cliff arm (Part C).

## Part A — Data and its deliberate boundaries

* **Source**: one public hover-endurance ULog (UUID- and sha256-pinned in
  the bundle's `source` field) from a **representative multirotor — not
  the project airframe**. Claims are tiered accordingly: this closes
  P5's "replace the synthetic sway input" at the representative tier;
  the flight card on the actual aircraft
  ([`HOVER_CAPTURE.md`](../../tools/phase10/rbec/HOVER_CAPTURE.md))
  remains the final upgrade.
* **Ingest**: `hover_ingest` auto-detected four Loiter segments
  (162 / 845 / 596 / 1175 s; 10 Hz EKF position, 20 Hz attitude),
  committed as `results/hover_p5.npz` — also the replay source for the
  future HIL rung V4. Vital-band (0.5–3 Hz) sway ≈ 2 mm RMS/axis on
  segments 1–3; attitude ≈ 0.45° RMS. Segment 0 carries 40–60 cm
  station-keeping excursions (the sloppy-hover arm); segment 3 carries
  the endurance z-descent (71 cm total, with 30 s windows at 11.4 cm/s
  commanded descent — **deliberately kept as the out-of-envelope arm**).
* **EKF-band caveat** (per HOVER_CAPTURE.md): the measured input covers
  the low-frequency band the EKF resolves; the rotor-vibration band
  stays synthetic, as in exp3.

## Part B — The four slices, measured vs synthetic twin

1. **Worst-realistic budget verdict** (register claim 1): the synthetic
   twin reproduces the on-record **0.0954 rad vs 0.110** (14 % margin);
   measured hover reads **0.0329 / 0.0369 / 0.0485 rad** (segments
   1/2/0) — margin 2.2–3.3×. The synthetic sway model was conservative
   for true hover; the budget claim strengthens, tier-annotated
   (measured *motion* through the simulated radar chain).
2. **T3 coupling behaves exactly as doctrine says** — linear in both
   σ_θ and platform excursion: at 0° angle error every arm sits on the
   same 0.018 rad floor; at 0.3°, segment 1 reads 0.0322 vs the
   synthetic 0.0421 vs segment 3's 0.1649.
3. **Segment 3 fails the budget (0.1653 rad), and the failure is pure
   T3 excursion-coupling** (clean at 0°; seams unaffected): commanded
   descent puts metres of in-dwell excursion against 0.3° of angle
   error. The measured envelope requirement that follows: **dwell-level
   altitude-hold drift at the ~1 cm/s level** (segment 1's median
   0.11 cm/s passes 3.3× inside budget; 11 cm/s busts it 1.5×) — or
   dwell gating on EKF velocity, whose signals the harness already
   carries. This is a new, quotable payload requirement, measured
   rather than assumed.
4. **The seam cliff is motion-invariant.** Integer failures at
   200/300/450/500 µm per gap are near-identical across the synthetic
   twin and all four measured segments (e.g. 331 target + 1610 anchor
   at 450 µm everywhere): exp4's attribution — chest velocity, not
   platform sway, consumes the seam margin — confirmed on measured
   motion. Seam-RAIM's rescue reproduces on real sway (bare 0.77–0.81 →
   0.022–0.063 rad at 200 µm), and the chest-prior's harm reproduces
   too (prior+raim+repair 0.48–0.52 — worse than RAIM alone), keeping
   that C9 negative intact on measured data.

## Part C — Verification and the one cross-platform note

`--self-test` (segment count, seam-schedule invariance, in-budget
default, the 950 µm cliff) and `--check` are green on the authoring
machine (macOS); the sandbox re-verification (Linux) reproduces the
974-line bundle exactly **except 7 leaves, all inside the IMU-500 µm
arm of measured segments 1–2** (largest deviation 3.9e-4 relative).
That arm sits deep past the integer cliff, where residuals are
dominated by hundreds of integer failures — a single `round()` decision
landing within a ULP of its boundary flips across BLAS orderings and
shifts a whole window's residual. The committed comparator's 1e-9
tolerance absorbs continuous ULP noise but *cannot* absorb
discretely-amplified ULPs; cliff-regime cells are therefore
platform-exact only per-platform. No headline number is affected (the
cliff arms are quoted qualitatively). This extends the BLAS-ULP note
from the D.7 rebase with its discrete-amplification mechanism.

## Part D — Follow-ups

1. The final P5 tier: fly HOVER_CAPTURE.md's card on the project
   airframe and re-run this module unchanged (`hover_p5.npz` is the
   only input).
2. Feed segment 0 (sloppy hover) and segment 3 (descent) into exp9's
   availability surface as measured-motion arms (exp9 D.3).
3. V4 HIL: `hover_p5.npz` is now the hexapod replay source the method
   doc's validation ladder specifies.
4. Altitude-hold envelope: wire the ~1 cm/s dwell gate into the
   harness-side dwell scheduler (the EKF velocity channel exists).
