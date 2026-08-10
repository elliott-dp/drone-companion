# RBEC numerical validation — does the budget close on paper?

Companion to [`radar_rbec_method.md`](radar_rbec_method.md). This document
reports the **V1-groundwork simulations** (code:
[`tools/phase10/rbec/`](../../tools/phase10/rbec/README.md)) that make the
method paper's budgets computable before any hardware exists. Everything here
is **[meas]** in the repo's sense — computed on this machine by seeded,
reproducible scripts — against models whose boundaries are stated in §A.

> **Answer up front: the budget closes, with one new hard requirement and two
> honest negatives.** At the nominal operating point the cardiac in-band
> residual is ~0.023 rad against the 0.110 rad budget, and even the combined
> worst-realistic case stays inside at 0.095 rad. The new hard requirement:
> **IMU dead-reckoning across the inter-burst gap must be good to ≲100 µm**,
> and the fragile seam is the **target track**, not the anchors — the chest's
> own respiration velocity consumes up to ~2/3 of the π unwrap margin across
> a 47 ms gap, which is why failures onset near 200 µm (all target-track)
> while anchors obey the plain λ/4 Gaussian arithmetic. The negatives:
> angle-aware WLS showed **no gain** over plain LS under
> per-dwell-systematic angle errors, and the corner-reflector anchor did
> **not** improve the solve in this configuration — both matter for what the
> bench should actually test.
>
> This document reflects the **post-review state**: an independent
> three-reviewer adversarial pass re-derived every formula, found ten
> defects (three of which changed conclusions), and all numbers below are
> from the fixed code. §F records what was found.

---

## Part A — Models and their deliberate boundaries

* **Array (exp1):** 86-element contiguous λ/2 azimuth ULA; per-element phase
  error = i.i.d. part + TX-chip-common + RX-chip-common parts (TI SPRACF4C:
  calibration codes are common per MMIC). The exact SWRU553A Fig. 27 chip
  map was **not** transcribed — a plausible mapping plus a seeded random
  permutation ablation is used; conclusions are shown to be
  mapping-insensitive. Kaiser tapers with *measured* design SLLs (β = 4.0 →
  −30.4 dB; β = 5.5 → −40.5 dB; uniform → the textbook −13.3 dB, which also
  validates the pattern code).
* **Geometry (exp2):** hover at h = 10 m; anchors from ground rings,
  structures, corner-reflector placements, and uniform sectors; unit
  weights. DOPs are reported dimensionless — multiply by
  σ_φ·(λ/4π) for displacement.
* **End-to-end (exp3):** 30 s dwell, 20 Hz frames, 12-chirp bursts at 250 µs
  spacing; 3-axis sway (shaped noise, knee 0.3 Hz — a synthetic stand-in for
  a measured hover PSD) + two rotor vibration lines (93/187 Hz, 76/30 µm);
  9 anchors with **per-dwell-fixed** angle errors; complex-domain casualty
  leakage into every anchor; master-APLL common step train (~1 Hz) +
  per-chip steps through a beam-weight-mismatch factor; per-chirp phase
  noise (anchor 25 dB, target 15 dB SNR per chirp); within-burst unwrap and
  IMU-seeded integer fixing across the ~47 ms inter-burst gap; plain LS or
  angle-aware WLS (Stöckel-structured weights), 2 iterations.
* **Not modelled** (stated in the method paper as V2–V6 territory):
  range/beam migration, rotation-induced beam-gain modulation and voxel
  jumps, multipath, near-field curvature, downwash-driven anchor motion
  (T11), RAIM-style integer-failure detection/repair. Leakage is applied
  with the same ratio and phase to *all* anchors (partially common-mode) —
  the single-contaminated-anchor case is worse and untested here.

Metrics: in-band residual **error** RMS (residual minus true chest signal)
in the respiration (0.1–0.5 Hz) and cardiac (0.8–3.0 Hz) bands, at frame
rate; budgets from the method paper: cardiac < 0.110 rad (= cardiac/3
≈ 33 µm), respiration ≪ 4 mm. 8 seeds per case; ± is the seed spread.

---

## Part B — Exp1: correlation is floor-benign but makes discrete spurs; separation is taper-limited

800-draw Monte Carlo of anchor-beam leakage toward a casualty at a given
angular offset (dB relative to the anchor mainlobe; **variance-matched**
cases, fixed chip map per run, mean-of-power statistics — all three were
review fixes; the first version's comparison was confounded):

| Case (total σ_φ matched) | 2° | 3° | 5° | 10° | floor (mean power) | analytic σ²Σw²/(Σw)² |
|---|---|---|---|---|---|---|
| uniform, iid 2.0° | −13.5 | −20.0 | −24.3 | −27.3 | −36.7 (deterministic-sidelobe-dominated) | −48.5 |
| kaiser30, iid 2.0° | −27.5 | −37.7 | −37.7 | −43.5 | −46.7 | −47.5 |
| kaiser30, chip 0.8°, tot 2.0° | −27.5 | −37.8 | −37.8 | −44.0 | −47.7 | −47.5 |
| kaiser30, chip permuted | −27.5 | −38.0 | −37.9 | −43.6 | −46.9 | −47.5 |
| kaiser40, iid 2.0° | **−16.9** | −39.7 | −45.6 | −46.7 | −46.8 | −46.9 |
| kaiser30, iid 2.0°, **steered 40°** | **−12.0** | −39.8 | −35.0 | −45.3 | −45.9 | −47.5 |

Spur scan (fine sin-space error-*excess* power, the observable the first
version lacked):

| Map | Top spur | Median excess |
|---|---|---|
| default map, chip-only 0.8° | **−43.6 dB at 7.2° (sin θ = 1/8 — the period-16 RX block structure, exactly as array theory predicts)** | −66.3 dB |
| permuted map, chip-only 0.8° | −49.4 dB at ~39° (flat, i.i.d.-like) | −54.2 dB |

Findings:

1. **At matched total variance, per-chip correlation does not raise the
   offset-table leakage or the mean floor** (kaiser30 rows agree within
   0.3 dB; floors track the taper-aware analytic value within ~1 dB — the
   honest comparator after fixing a missing taper-efficiency term and a
   −2.5 dB dB-averaging bias the review caught).
2. **But a fixed chip map produces discrete spurs at predictable angles**:
   the default map's period-16 RX block structure concentrates chip-common
   error power at sin θ = k/8 — a −43.6 dB spur, ~23 dB above the median
   excess, invisible to the offset table. The method paper's F.3 caveat is
   therefore **resolved with a nuance**: floor-benign, spur-real. The spur
   angles are computable from the map, so the anchor-admission rule gains
   one clause: *do not place anchors at the map's spur angles relative to
   the casualty* — and the measured cal vector (follow-up 5) settles the
   real spur levels.
3. **The separation rule is mainlobe-limited and steering-dependent.** The
   −40 dB taper widens the mainlobe (first null 2.7°), so 2° separation
   sits on its shoulder at −16.9 dB; a 40°-steered anchor beam is wider in
   real angle, making 2° separation catastrophic (−12.0 dB) while ≥3°
   survives everywhere (−37 to −40 dB). Rule: **anchor–casualty separation
   ≥ 3° at any steering** — at 10 m standoff, ≈ 52 cm cross-range.

## Part C — Exp2: geometry is forgiving, with one sharpened nuance

| Scene (target el −30°) | N | DOP(x,y,z) | DOP(u_t) | CM-leak | cond |
|---|---|---|---|---|---|
| ground ring, one range (el −30°) | 7 | 3.19, 0.83, 4.69 | **0.59** | 0.001 | 176 |
| ground rings 6/12/25 m | 12 | 0.87, 0.66, 0.82 | 0.46 | 0.128 | 12 |
| rings + 3 structure anchors | 15 | 0.62, 0.55, 0.68 | 0.35 | 0.206 | 8 |
| rings + corner reflector (3 placements) | 13 | ~0.8, ~0.65, ~0.8 | 0.42–0.43 | 0.11–0.17 | 11 |
| sector el −60…−5, N=24 | 24 | 0.46, 0.53, 0.63 | 0.24 | 0.128 | 11 |

1. **A flat-ground scene is degraded, not singular** — with the geometry
   stated precisely (review-corrected): anchors on one range ring have
   exactly *coplanar LOS tips* (shared z), but the translation-only solve
   is singular only when the LOS **directions** span rank < 3, i.e. when
   the tip plane passes through the origin (zero height, or collapsed
   azimuth spread — verified: shrinking azimuth span 100°→0.1° drives the
   condition number 1.8×10² → 2.5×10¹⁴). The coplanar-tips-singular
   intuition is the GNSS clock-column theorem and applies only if a
   common-phase column is added — which rule C.3-2 forbids. On the single
   ring, per-axis DOPs inflate (3.2/4.7 in x/z) but the **target-LOS DOP
   stays 0.59** for an on-cone target. The method paper's Part G row is
   corrected accordingly, and `dop_matrix` now raises explicitly on
   rank-deficient geometries (the review found `np.linalg.inv` silently
   returning NaN/garbage DOPs on an exactly-singular collapsed-azimuth
   case).
2. **Range-diverse ground anchors already give DOP(u_t) ≈ 0.46** — no
   structures needed for a good solve along the casualty LOS. Structures
   and more anchors help the cross-axes (irrelevant to the prediction) and
   availability.
3. **Implicit common-mode cancellation is real but partial and
   geometry-dependent**: the fraction of a common anchor-phase offset
   surviving at the target ranges 0.1 %–33 % across scenes. The review
   corrected the first interpretation: the survival factor 1 − u_tᵀg is
   *signed*, and its small values are **zero crossings** whose location is
   geometry-dependent (in the (e)-sweep the minimum sits at ~30° offset,
   not at the cluster centroid, where the leak is 6× larger). For a
   single-depression ring there is a verified closed form,
   CMR = |1 − sin(el_t)/sin(el_ring)| — which is why the on-cone target
   showed 0.001. Design consequence unchanged: measure it per scene, don't
   assume it — exp3 does, with step injections.

## Part D — Exp3: the end-to-end solve — budget verdict and the discovered cliff

Nominal point: 0.1° anchor angle error, 2 cm RMS/axis sway, −30 dB leakage,
50 µm/gap IMU, 50 mrad APLL steps, 9 anchors.

| Sweep | Key numbers (cardiac in-band residual, rad) | Verdict |
|---|---|---|
| **T3 angle error** 0 → 0.675° | 0.018 → 0.086; respiration-band error 2.7 → 100 µm, linear in σ_θ and in sway | T3 confirmed as the pacing term; **even half-bin quantization (0.675°) stays inside the 0.110 budget** in this configuration; residual concentrates in the respiration band, where the 4 mm signal dwarfs it |
| Sway 0 → 5 cm RMS | 0.019 → 0.037 | Linear coupling with T3, as the paper's Sensors-J-derived model predicts |
| **T8 leakage** 0 → −10 dB (all anchors) | 0.022 → 0.064 | −30 dB is invisible; even the "fatal" −10 dB case stays under budget *here* — but the model leaks identically into all anchors (partially common-mode, partially absorbed by the solve); the single-bad-anchor case is worse and remains for V2 |
| **IMU per-gap error** 20 µm → 950 µm | 0.023 (≤50 µm) → **0.58 at 200 µm — 15 failures, all target-track** → 2.2–2.4 at 500–950 µm (anchors now failing too: 459t+2423a at 500 µm) | **The discovered cliff, correctly attributed** (the review caught the first version's misattribution): anchors obey plain λ/4 Gaussian arithmetic (predicted 2491 vs observed 2423–2543 failures at 500 µm — 2 % agreement), but the **target track fails first because the chest's own motion across the 47 ms gap consumes up to 2.09 rad of the π margin at respiration-velocity peaks**, leaving only ~1.05 rad ≈ 317 µm for IMU error. Requirement: IMU ≤ ~100 µm per gap (accelerometer arithmetic says ~30 µm — a 3× margin), plus a **chest-velocity prior in the target-track integer fix** (extrapolate the respiration slope), which the anchors don't need. RAIM-style detection remains the mitigation to build — and the fixed shared-IMU model matters for it: one physical IMU error is common across all tracks, so failures co-occur rather than being independently excludable |
| Anchor count 4 → 16 | 0.027 → 0.020 | Mild ~1/√N; geometry saturates quickly, consistent with exp2 |
| **T6 APLL steps** (steps only, **high SNR** — the review showed the first version's CMRR was noise-floored at 3–12 dB and its mismatch sweep measured nothing) 50 → 1000 mrad | absolute residual 0.0009 → 0.0160 rad; **true CMRR 25.8–28.7 dB** | **Common-mode rejection works, with its ceiling identified**: the de-noised rejection is ~26–29 dB, set not by the beam-weight mismatch (0.1 vs 0.3 indistinguishable) but by the **geometric leak of the common step through the solve** — exp2's \|1 − u_tᵀg\| factor. Even 1 rad steps — 20× any plausible magnitude — leave 0.016 rad in band, 7× under budget. In the full sim, 200 mrad steps move the residual 0.0227 → 0.0243 |
| Estimator: LS vs angle-aware WLS; ± corner anchor | 0.038–0.042, all four cases | **Honest negative ×2**: WLS shows no gain when angle errors are *systematic per dwell* (biases don't average like the noise the weights model), and the corner anchor doesn't improve the solve (its benefit is availability and leakage margin, not DOP). Stöckel's 26–36 % WLS gains were not reproduced under this error model — the bench must establish which error model is real before investing in the estimator |
| **Combined worst-realistic** (0.3° + 5 cm + −30 dB + 200 mrad) | **0.095 ± (worst seed inside budget)** | **The budget closes with ~13 % margin** at the worst plausible corner of every modelled term simultaneously |

Noise floor: with every error zeroed the cardiac residual is 0.018 rad —
set by the target's 15 dB per-chirp SNR, giving the budget 6× headroom over
the floor. Respiration-band error never exceeded 112 µm in any case against
a 4 mm signal.

---

## Part E — What this changes in the method paper

1. **F.3 caveat resolved with a nuance**: at matched variance,
   per-chip-correlated errors do not raise the leakage table or the mean
   floor — but a fixed chip map makes **discrete spurs at predictable
   angles** (sin θ = k/8 for the default map's block structure, ~−44 dB,
   ~23 dB above the median excess). Anchor admission gains a
   spur-angle-avoidance clause; the measured cal vector settles real
   levels.
2. **T8 rule sharpened**: separation ≥ 3° *at any steering* —
   mainlobe-limited, worse off-boresight (2° is −12 dB at a 40°-steered
   anchor); heavier tapers do not pay below 3°.
3. **Part G flat-ground row corrected**: singularity requires the LOS
   *directions* to span rank < 3 (zero height or collapsed azimuth), not
   coplanar LOS tips — that intuition is the GNSS clock-column theorem and
   applies only with the common-phase column rule C.3-2 forbids. A single
   ring is ill-conditioned cross-sector but DOP(u_t) ≈ 0.6 on-cone.
4. **C.1 unwrap requirement quantified, and re-attributed**: the fragile
   seam is the **target track** — chest velocity consumes up to ~2/3 of
   the π margin over a 47 ms gap — so the target integer fix needs a
   chest-velocity prior on top of the ≤ ~100 µm IMU requirement; anchors
   obey plain λ/4 Gaussian arithmetic (2 % agreement with prediction).
   RAIM-style detection promoted to required engineering, with the caveat
   that IMU error is common across tracks, so failures co-occur.
5. **T6 verdict upgraded from analysis to simulation, with the ceiling
   named**: true rejection is 26–29 dB, set by the geometric \|1 − u_tᵀg\|
   leak, insensitive to beam-weight mismatch; even 20×-plausible steps stay
   7× under budget. E10's remaining role is measuring the *actual* step
   size and the per-chip component.
6. **Estimator guidance revised**: do not assume WLS or the corner anchor
   buy accuracy — the review confirmed the WLS implementation wins exactly
   where theory says it should (per-frame-random errors + heterogeneous
   anchors: 8/8 seeds), so the shipped no-gain result is a true statement
   about the *bias-error regime*, not a bug. "Characterise whether anchor
   angle errors are systematic or frame-random" is now a V2 bench goal —
   it decides the estimator.

## Part E2 — Exp4: mitigations and leakage topologies (follow-ups 1–2, executed)

**The real chip map** (follow-up 5, done): TIDUEN5A Figures 5–6 were read
visually from the primary PDF — azimuth TX at λ/2-positions {0,4,…,32}
(slaves only; the master's three TX are elevation), RX blocks at {0–3}
(device 4), {11–14} (master), {46–49} (device 3), {50–53} (device 2),
spanning exactly the annotated 26.5 λ and tiling virtual positions 0–85.
On the real map the chip-error spur moves to **2.6° off boresight**
(−40.9 dB at 0.8° chip error) — into the anchor-separation zone — costing
2–4 dB of p90 leakage margin at exactly 3° separation (−35.7 → −33.4/−31.6
dB) and fading by 4–5°. **Rule refined: ≥ 3° acceptable, ≥ 4° preferred.**

**The seam mitigations, measured** (at the 200 µm/gap cliff onset; 8 seeds):

| Mitigation | Target failures | Cardiac residual |
|---|---|---|
| bare | 15 | 0.579 rad — FAIL |
| chest-velocity prior (anti-cascade, median-of-wrapped-diffs) | 11 | 0.379 — FAIL |
| **seam-RAIM (anchor-consensus IMU correction)** | **0** | **0.0228 — nominal** |
| prior + RAIM | 8 | 0.355 — FAIL (the prior's noise *hurts* RAIM) |

Three findings, one of them the design answer:

1. **Seam-RAIM eliminates the cliff at its onset.** Anchors carry the same
   shared IMU error but no chest term and don't fail at 200 µm — so their
   per-seam sub-integer innovations LS-solve the common IMU error, and
   correcting the target's prediction with it removes the ambiguity
   entirely. The IMU requirement relaxes from ≲100 µm to **≲300 µm per
   gap** (the hard wall becomes the anchors' own λ/4 Gaussian limit at
   ~450 µm, where everything fails together). Margin over the ~30 µm
   achievable: ~10×, restored.
2. **Post-hoc slip repair cannot work at the cliff, by construction** —
   instrumentation showed the failing seams' residuals land at ~±3.1 rad,
   exactly the ambiguity zone where a slip and a legitimate excursion are
   indistinguishable from one track. (It still cleans up gross multi-slip
   cases: 12 repairs cut a 0.36 rad residual to 0.16.)
3. **Honest negative #3: the chest-velocity prior is dropped.** Even
   implemented anti-cascade (the naive version cascaded catastrophically —
   one slip poisoned its memory for the rest of the dwell), it removes only
   ~a quarter of failures and its noise degrades RAIM when combined. The
   method paper's C.1 recommendation becomes seam-RAIM, not the prior.

**Leakage topologies** (follow-up 1): a single contaminated anchor is
*milder* than all-anchors at the same ratio (0.026 vs 0.064 rad at −10 dB —
the solve averages one bad equation down by ~1/N). The binding topology is
the **reverse direction**: a strong anchor echo entering the *target* beam
(anchors are ~10 dB above the chest echo) — at an effective amplitude ratio
of 0.316 the cardiac residual hits 0.113 rad, the one modelled case that
fails budget outside the unwrap cliff. Design consequence: the target
beam's isolation *toward the anchors* matters as much as the reverse, which
strengthens the ≥ 3–4° separation rule (it is symmetric) and is the
concrete case for an LCMV null on the strongest anchor in the target beam.

## Part F — Verification status and follow-ups

The stack was self-tested (Parseval, tone RMS, K·λ/4 = π, taper SLLs against
textbook values) and then adversarially reviewed by three independent
reviewers who re-derived every formula and ran their own numerical checks.
What they confirmed: the GDOP covariance to 0.4 % (200k-trial MC), the
common-mode-leak formula to machine precision (plus a closed form for
single-depression rings), sign conventions end-to-end (a sign error would
have left 141 µm of uncancelled cardiac signal vs the observed 7 µm), the
Parseval-exact band RMS, and that the WLS implementation wins exactly where
weighting theory predicts. What they found (all fixed before the numbers in
this document were finalized): a missing taper-efficiency term and a
−2.5 dB dB-averaging bias in exp1's floor comparator; a confounded
variance-matching in exp1's case table; a per-draw (rather than fixed)
permutation that made the spur ablation structurally blind — with the spurs
themselves then demonstrated at sin θ = k/8; silent NaN DOPs on exactly
singular geometries; the wrong geometric reason attached to two
conclusions (coplanar-tips vs rank-deficient-directions; the CM-leak "best
at centroid" misreading of a sign-change zero crossing); the integer-fix
cliff misattributed to the IMU alone when 19/20 onset failures were
chest-velocity-eroded target seams; a noise-floored CMRR metric whose
mismatch sweep measured nothing; per-track-independent IMU errors breaking
the cross-anchor failure correlation; and a mis-sampled diagnostic. Known
remaining model gaps flagged by the review, beyond §A's list: anchor→target
beam leakage (the reverse direction — anchors are stronger, ~0.1 rad
cardiac-scale worst case) is unmodelled, and the TX/RX residuals of one
MMIC are drawn independently. Follow-ups, in order:

1. ~~Leakage topologies~~ **done** (exp4, §E2): single-anchor milder;
   reverse anchor→target is the binding case → LCMV null candidate.
2. ~~Seam mitigations~~ **done** (exp4, §E2): seam-RAIM is the answer;
   chest prior dropped; post-hoc repair shown structurally unable at the
   cliff. Remaining engineering: RAIM outlier-robustness when an anchor
   itself slips (currently plain LS over all anchors).
3. Replace the synthetic sway with a measured PX4 hover PSD — **ready to
   execute**: the pilot has offered hover flights, and the capture kit is
   built — flight card in
   [`tools/phase10/rbec/HOVER_CAPTURE.md`](../../tools/phase10/rbec/HOVER_CAPTURE.md)
   (`SDLOG_PROFILE = 17`, Loiter/Position hovers ≥ 2 min per segment),
   ingestion via `hover_ingest.py` (auto-detects hover segments, reports
   per-axis band RMS, writes the replay npz), and the simulator replays it
   through `SimConfig(motion_npz=...)`. Pipeline smoke-tested end-to-end on
   the upstream boat-test sample (which, at 5–12 m of travelling motion,
   also served as an unplanned stress case: 0.20 rad residual, zero unwrap
   failures). Once the real logs land, exp3/exp4 re-run on measured
   trajectories re-derives T3, the seam rates and the budget verdict.
4. Frame-random vs per-dwell-systematic angle-error characterisation (V2
   bench question that decides the WLS story).
4b. **Track 1 — the RBEC solve on real cascade data: kit built, data
   pending.** `coloradar_bridge.py` parses ColoRadar's 4×AWR2243 raw ADC +
   calibration (format taken from the dataset's own dev-kit code — which
   independently confirms the TIDUEN5A chip-row ordering) and passes a
   synthetic-fixture round-trip (angle exact, phase step within 1.6 %);
   `exp5_coloradar.py` scores the anchor solve against ground-truth pose
   increments plus a held-out static-cell residual that needs no ground
   truth. Honest scoping: at ColoRadar's cascade frame rate and walking
   speed, inter-frame motion is tens of wraps, so integers are GT/IMU-
   seeded — the experiment validates the LS geometry and sub-wavelength
   residual on real data, not blind unwrapping. Blocked only on data
   acquisition: every anonymous download route is dead (SharePoint removed,
   2021 GDrive links revoked, Radatron ships heatmaps only); the live route
   is the ColoRadar+ Globus collection (free login). The real cascade
   calibration is already in hand via a public vendored copy (fetch script
   committed) and verified through the bridge — 86 azimuth positions
   spanning 0–85, λ = 3.831 mm, coupling matrix applied — so only one
   sequence archive is needed. ASPEN sequences carry **Vicon mm-class
   ground truth**; best target `2_24_2021_aspen_run9` (83 s).
5. ~~The real chip map~~ **done** (TIDUEN5A Fig. 5/6 read visually, §E2):
   real spur at 2.6°; separation rule refined to ≥ 3° / ≥ 4° preferred.
