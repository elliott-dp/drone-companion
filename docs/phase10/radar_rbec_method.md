# RBEC — reference-beam ego-motion cancellation: a method proposal

> **Status: method proposal. No code, no measurements.** Companion to
> [`radar_dsp_ml_survey.md`](radar_dsp_ml_survey.md) §B.10, which ranks
> ego-motion compensation strategies and flags "a *beam* can serve as the
> second channel at no hardware cost" as the one experiment this aperture
> uniquely enables. This document develops that idea into a specified,
> falsifiable method, against primary sources read in the 2026-08 research
> passes (sourcing record in §K; tags as in the survey — **[prim]** primary
> document read · **[calc]** arithmetic here · **[open]** unresolved).

**The idea in one paragraph.** Each 20 Hz frame of the 4×AWR2243 cascade is a
192-virtual-channel datacube. From that *same* cube, digitally synthesize
(a) a target beam on the casualty and (b) several reference beams on static
scene anchors — ground patches, structures, a deployed corner reflector.
Solve the platform's frame-to-frame displacement from the anchor phases by
weighted least squares (the deterministic path), and/or cancel the residual
adaptively with the anchor phases as reference inputs (the adaptive path).
Because target and references share one LO, one APLL, one calibration state,
one clock and one instant of observation, instrument artifacts — including
the documented, non-disableable 1 Hz APLL/VCO recalibration that sits inside
the cardiac band — appear as common mode and largely cancel, and the platform
motion that is 10³–10⁴× larger than the cardiac signal is subtracted with the
scene itself as the reference. Nothing about the recorder changes: RBEC is a
pipeline attached to the harness, and every beam weight, anchor definition
and calibration event it uses is recorded so any live result is reproducible
offline.

---

## Part A — What is being combined, and what is actually new

RBEC is deliberately a *combination*: every ingredient exists, most are
flight-proven, and none has been assembled this way.

| Ingredient | Source lineage (primary-read) | What it contributes |
|---|---|---|
| Scene-anchor phase least squares, flown at 77 GHz | Stöckel/Fraunhofer FHR line: TRS 2024 (rotating radar under a hovering UAV, ~98 % motion removal, 1.33 % respiration error), Sensors J 2024 (angle-error-aware WLS, 0.643 mm flight residual), TAES 2023 (per-DoF bench), 2025 thesis **[prim]** | The estimator: per-frame LS/WLS on anchor phases, anchor detection/tracking machinery, the proof that anchor *angle* error dominates (up to 94 % of anchor error), the second-derivative unwrapper, and the IMU-spectrum anchor veto |
| Reference-channel adaptive cancellation, flown at 24 GHz | Nakata IMS/JETCAS 2016-18 (26 dB SIR airborne, deterministic); Islam EuMC 2020 (RLS bench); Ishmael TMTT 2025 (dual-radar, sub-mm displacement under >100 mm platform motion) **[prim]** | The ANC formulation, reference-conditioning discipline (full arctangent demodulation + IQ correction + robust unwrap *before* cancellation), and the honest precedent that deterministic subtraction matched or beat ANC in every flown test |
| SAR autofocus | PGA (Wahl/Eichel/Jakowatz Sandia reports **[prim]**): the LUMV phase-gradient kernel, four-step architecture, W³ noise law, threshold effect; space-variant generalizations (Boeing patent, 2D-SVMDA, FGA); NRL 94 GHz vibrometry (PGA recovering 8–120 µm vibrations from an accelerating car) **[prim]** | The optimal-estimation backbone and its 35-year-old failure catalogue; proof that autofocus-class processing reaches micrometre micro-motion on moving platforms |
| Range-correlation physics | Budge & Burt 1993 via Droitcour 2004 and two open transcriptions **[prim: relation verified; originals paywalled]**; TI SWRA574B + SPRACF4C + AWR2243 datasheet **[prim]** | Why the same-datacube reference is special: all shared-LO artifacts scale with the *range difference* between target and anchor, and the cascade's 1 Hz APLL/VCO recalibration lives in the master's shared LO path |
| Beam-space vitals on this exact hardware | Ren et al. arXiv:2411.09201 (five simultaneous chest-point beams from one MMWCAS datacube, SCG-validated ρ = 0.84–0.88) **[prim]**; Han & Hong EuRAD 2022 (differential beams cancel common-mode motion ~18 dB); Lai et al. 2026 (LCMV-with-nulls recipe) **[prim]** | Proof that simultaneous multi-beam phase extraction at 20 Hz is routine on this cube — and empirical documentation of the cross-beam leakage RBEC must engineer against |
| Anchor-quality theory | PS-InSAR amplitude dispersion (Ferretti 2001, D_A < 0.25) **[prim]**; AutoCalib 2025 (natural anchors on this same 86-antenna cascade reach 96 % of corner-reflector performance) **[prim]**; GDOP theory (Langley 1999) **[prim]** | Quantitative anchor admission, weighting, and geometry rules |

**The narrowed novelty claim** — each neighbour was checked, and the claim is
stated against them, not into a vacuum **[prim, 2026-08 sweep]**:

1. *Digitally synthesized reference beams from one datacube, used to solve and
   cancel UAV ego-motion for vital signs.* Stöckel's Sensors J 2024 §V-C
   explicitly names MIMO radars as the successor architecture but no
   publication implements it; Han & Hong difference beams on *one chest*, not
   scene anchors; Cardillo 2021 references same-radar clutter but by *range
   bin*, deterministically, on a static radar; the Raytheon patent
   (US 7,978,124) uses a single stationary-object range bin with no LS over
   multiple anchors and no co-range choice. Same-array primary/reference
   cancellation is the Griffiths-Jim GSC lineage — cited, with its
   signal-cancellation failure mode inherited and addressed (§D, §G).
2. *Co-range anchor selection as an instrument-artifact suppressor* — the
   two-cell residual of every shared-LO artifact scales with (R_target −
   R_anchor), a knob only the digital-beam variant can turn (§E).
3. *The shared-LO/APLL common-mode analysis and its in-band experimental
   verification via TI's per-event calibration reports* — no prior work
   models or measures instrument artifacts in this context (§E).
4. *A 3-D translation solve from a single 2-D-topology cascade in flight* —
   the flown prior art is 2-D + yaw (TRS 2024); the 6-DoF paper is a tripod
   emulation. Flying 3-D + attitude re-steering would itself be a first.
5. *The validation ladder's two unpublished elements*: per-cell phase
   remodulation of recorded real datacubes, and IMU-trajectory replay on a
   hexapod under a radar (§H).

---

## Part B — Signal model and the error taxonomy

Geometry: platform displacement **d**(t) ∈ ℝ³ (antenna phase centre, relative
to dwell start), anchor k at LOS unit vector **u**_k and range R_k, casualty
at **u**_t, R_t. λ = 3.79 mm at 79 GHz; 4π/λ = 3.311 rad/mm.

Anchor-beam phase (after removing the static geometric phase):

```
φ_k(t) = (4π/λ) uₖ·d(t) + ψ_LO(t)·h(τ_k) + b_k + ε_k(t)          (B.1)
```

— the projection of platform motion on the anchor LOS, plus the shared-LO
artifact ψ seen through the delay response h(τ_k), plus a static near-field
bias b_k, plus noise. The target beam obeys the same equation *plus the chest
displacement* (4π/λ)·x_chest(t)·(chest-normal projection). RBEC's output is
the target phase minus the prediction (4π/λ) **u**_t·**d̂**(t), with the
adaptive stage (§D) operating on the residual.

The error taxonomy, each term with its magnitude and its fate:

| # | Term | Magnitude | Fate under RBEC |
|---|---|---|---|
| T1 | Platform translation | 331–4967 rad over a dwell (survey A5) **[calc]** | The thing being solved; residual set by T3/T4 |
| T2 | Platform rotation | Enters anchor phases only through the lever arm (rotation about the phase centre changes no far-anchor range). Gyro noise over one 50 ms frame ≈ 13 µrad → **2.6 µm** through a 20 cm lever arm — 40× below cardiac **[calc]** | Take rotation from the FC gyro as a *prior*; re-steer beams per frame (Stöckel's spectral shift). Do **not** solve rotation from anchors — with sector-clustered anchors it is badly conditioned and unnecessary. The lever-arm displacement is absorbed into the 3-DoF translation solve automatically |
| T3 | Anchor angle error × platform excursion | **The dominant term** (Stöckel: up to 94 % of anchor error; 3.2 mm at 10 cm sway with 1.8° error). At our 1.35° bins and 20 cm sway: ~2.4 mm — 24× cardiac **[prim + calc]** | Attacked three ways: 1.35° aperture (vs their 3.6°), super-resolved/off-grid anchor DoA over 192 channels, and a surveyed corner reflector. Must appear in the budget as the pacing term |
| T4 | Phase-noise floor per anchor | σ_φ = 1/√(2·SNR); anchors are the strongest returns in a ground-looking scene, so SNR-rich by construction | Sets the WLS weights (SNR-scaled, per Stöckel Eq. 49); negligible against T3 |
| T5 | Shared-LO phase noise | Range-correlation-filtered: 4 sin²(πfτ) ≈ −92 to −134 dB across 5–30 m, 1–20 Hz **[prim + calc]** | Already negligible before differencing; differencing improves it by a further (ΔR/R)² |
| T6 | APLL/VCO 1 Hz recalibration step | Master-chip cal is in the **shared LO path** → common-mode across all 192 channels; a pure LO phase step cancels in dechirp after a ~ns transient; a frequency step δf leaves 4π·δf·ΔR/c in the beam difference (0.021 rad even for a pessimistic 100 kHz step at ΔR = 5 m) **[prim + calc]** | Neutralized for the master path; **slave** APLL steps touch that chip's ADC clocks only — per-chip, bounded ≈ 5 mrad in the difference for 0.55 ns sync uncertainty at ΔR = 2 m. Claim: "neutralizes the shared-LO component, bounds the per-chip component" — §E |
| T7 | Per-channel calibration drift | < ±2° INL after cal, ~0.01–0.03°/°C — quasi-DC; enters beams as *pattern* (pointing/sidelobe) perturbation, second-order for the phase **[prim]** | Matched weight-fractions-per-chip in target and reference beams cancel per-MMIC common steps exactly; residual = weight dissimilarity × step |
| T8 | Sidelobe leakage of the casualty into a reference beam | With a −30 dB taper and *equal* chest/anchor echoes: parasitic phasor ≈ 0.032 → ~31.6 mrad ≈ 9.8 µm — 10 % of cardiac, concentrated in the respiration band. **If the anchor echo is 20 dB weaker than the chest: ~0.3 rad — equal to the entire cardiac signal. Fatal.** **[calc on prim formulas]** | The method's central risk (Widrow's leakage law, §D); governed by the anchor-selection rule in §F.3 |
| T9 | Near-field curvature | 2D²/λ ≈ 14.3 m for the 42.5 λ aperture — casualty *and* anchors sit in the Fresnel region at operating standoff **[prim, Ren]** | Ren-style cosine-law per-channel corrections; the *static* part of the curvature bias cancels in frame-to-frame differencing, but it is anchor-geometry-dependent and does **not** cancel in the LS solve while the platform translates — must be modelled, not ignored |
| T10 | Chirp-start jitter | 0/1.1 ns bimodal → 0.4° per chirp at 1 MHz IF **[prim, SPRACV2]** | Averages across chirps; favour low IF for vitals cells (survey B.1) |
| T11 | Downwash-driven *ground* motion | Anchors on downwash-agitated surfaces move coherently with rotor state — common across anchors, **indistinguishable from platform motion by geometry alone** | A distinct error class anchor geometry cannot fix; mitigations: rigid anchors (structures, reflector), the D8 measurement, rotor-state regressors — §G |

---

## Part C — The deterministic estimator (the primary method)

RBEC's deterministic path is **space-variant PGA**: anchors play PGA's
redundant range bins, the frame index plays aperture position, and the scalar
common-phase estimate becomes a vector displacement solve. The mapping keeps
PGA's proven parts and replaces the one assumption RBEC violates.

**C.1 Phase tracking at chirp rate, not frame rate.** The published failure
mode is unambiguous **[prim]**: at 40 Hz anchor sampling Stöckel saw ~10 mm
inter-sample motion against a λ/4 = 0.97 mm unwrap limit and needed a
dedicated second-derivative unwrapping algorithm; his outdoor system went to
500 Hz frames "to avoid unwrapping issues". The cascade samples **every beam
on every chirp**: at a 4 kHz chirp rate, 0.5 m/s sway moves 0.125 mm per
chirp — comfortably inside λ/4. RBEC therefore tracks anchor and target
phases chirp-to-chirp *within* each frame and stitches across frames,
reducing the unwrap crisis to the inter-frame seam. That seam is handled as
a GNSS-style integer problem: seed with integrated Doppler
(Δφ ≈ −[D(t)+D(t−1)]·Δt/2) *plus* the IMU short-horizon prior (µm-class over
50 ms), round when the residual < λ/4, and use the N-anchor redundancy for
RAIM-style outlier exclusion (a wrong integer on one anchor is a gross LS
residual) or joint integer LS (LAMBDA lineage). Honest requirement: 20 Hz
Doppler resolution alone predicts inter-frame displacement only to ~1.5 mm >
λ/4, so **the IMU prior is mandatory, not optional** **[prim + calc]**.
Stöckel's second-derivative unwrapper (integer-counter correction of
first-derivative jumps, IMU-seeded initialisation, 30·2π divergence reset)
is the documented fallback for frame-rate-only processing.

**C.2 The solve.** Per frame (or per chirp-block):

```
φ_k = (4π/λ) uₖᵀ d + ε_k ,  k = 1…N     →     d̂ = argmin Σ w_k (φ_k − (4π/λ)uₖᵀd)²
```

with the covariance (λ/4π)² (UᵀWU)⁻¹ — a GDOP problem, reported as such.
Adopt Stöckel's angle-error-aware WLS verbatim as the starting point
**[prim, Sensors J 2024 Eq. 31/49]**: W from the azimuth-error covariance
projected through the *previous* displacement estimate, plus SNR-scaled
per-anchor phase noise — it cut the motion-parallel error component by 83 %
and flight residual by 36 % over plain LS. At 20 Hz, implement his Kalman
variant (constant-acceleration model, adaptive measurement noise): he reports
it outperforms WLS below 10 Hz and matches it elsewhere; 20 Hz with large
sway is near that regime. The PGA correspondence fixes the estimator details:
estimate in the phase-*difference* domain and integrate (sidesteps absolute
unwrap), weight by anchor power (the LUMV kernel is amplitude²-weighted),
keep slow-time processing bandwidth minimal (phase-derivative noise grows as
W³), and iterate estimate→apply→re-estimate with a stopping criterion in
radians RMS of anchor residuals (PGA converges in ≤3–4 iterations
empirically; RBEC's stop must be tighter, e.g. < 0.05 rad, since the cardiac
signal is 0.33 rad) **[prim, Sandia reports]**.

**C.3 Geometry, quantified.** Numeric GDOP for the cascade FoV **[calc,
pillar analysis]**: 12 anchors uniform in ±60° az × ±30° el give boresight
DOP ≈ 0.41, cross-azimuth ≈ 0.5, cross-elevation ≈ 0.8–1.0 — sub-millimetre
solves are geometrically possible. Confining anchors to ±15° elevation
degrades cross-elevation DOP to 2.4–3.2 (the GNSS one-sided-sky VDOP effect,
Langley). Three rules follow:

1. **Bracket the target.** What matters is the projection of Cov(d) onto
   **u**_t; choosing reference beams that angularly surround the target beam
   makes the poorly-observed cross-sector component project weakly onto the
   prediction. Report cancellation as a function of target–anchor angular
   separation, never as one number.
2. **Do not add a common-phase nuisance column** to the solve: with
   sector-clustered anchors it is nearly collinear with boresight translation
   (boresight DOP inflates 0.41 → 2.36 **[calc]**). Common instrument phase
   is handled by the differencing (§E), which needs no extra unknown.
3. **Coplanarity is the operational limit**: anchors that are all ground
   patches at one depression angle have coplanar LOS tips → singular solve.
   A hovering radar over flat featureless ground is RBEC's worst case; the
   mitigation is structures, a deployed corner reflector (pair-GDOP
   √2/|sin γ| quantifies its placement value; γ → 90° reaches the floor √2),
   or accepting compensation only along observed axes, declared in the
   output quality flags.

**C.4 Anchor management.** Composite admission/weighting rule, every part
from a primary-read source:

* **Detection**: CFAR on the time-integrated energy map S(r,θ) = Σ_t |s|²
  (Stöckel); r⁴-normalized power within 5–15 dB of scene max; 20–50 anchors
  for a 3-unknown solve (the automotive-SAR GCP rule, Tagliaferri).
* **Stability admission**: PS-InSAR amplitude dispersion D_A = σ_A/m_A over a
  sliding slow-time window, admit at D_A < 0.25, weight by ~1/D_A² (valid
  only for strong scatterers — D_A saturates at the Rayleigh 0.5 for weak
  clutter, so weak patches cannot be certified by D_A alone) **[prim,
  Ferretti]**.
* **Anti-casualty veto**: Stöckel's TAES rule — argmin ‖w_k − w_IMU‖² +
  µ‖v_k‖² with the FC IMU spectrum as the platform-motion template and v_k
  the anchor's energy in the respiration/cardiac bands. This is the direct
  answer to "what if a reference beam lands on a survivor".
* **Pointness discipline** (PPP): anchors must be point-like within a
  resolution cell — validate by PSF width and amplitude variance; the corner
  reflector satisfies this by construction. AutoCalib's result on this exact
  86-antenna cascade — natural anchors reaching 96 % of corner-reflector
  phase-calibration performance, ranked by template match + boresight-favoring
  geometric score — is direct evidence adequate natural anchors exist in
  real scenes **[prim]**.
* **Migration tracking**: an anchor is valid while platform motion stays
  within one range bin (3.75 cm at 4 GHz) *and* the azimuth PSF
  (r·tan 1.35° — ~3× tighter per metre of range than the prior art's 3.6°
  system). Track each anchor's peak bin per frame and apply PPP-style
  frequency+phase (range-shift) correction rather than reading a fixed bin;
  re-associate on migration; anchor dropouts are an availability event
  (Doppler dead-reckoning at the demonstrated ~1.6 cm/s noise floor burns
  the entire cardiac budget in ~10 ms **[prim + calc]**).

---

## Part D — The adaptive variant (the evaluated alternative)

The multi-reference ANC path uses the anchor-beam phases as reference inputs
to an adaptive canceller on the target phase. Its theory is 50 years old and
brutally clear about the risks **[prim, Widrow 1975/1976]**:

* **The leakage law is exact**: output signal-to-noise density ratio =
  1/ρ_ref at every frequency, where ρ_ref is the signal-to-noise density
  ratio *in the reference*. Every dB of casualty leakage into an anchor beam
  directly caps the output vital-band SNR, with distortion D ≈ ρ_ref/ρ_pri.
  This is why §F.3's leakage budget is computed *before* any flight.
* **The notch failure mode**: a quasi-periodic respiration leak converges the
  canceller to a notch at f_resp with bandwidth µC²Ω/π, and fast adaptation
  adds nonlinear cancellation beyond the Wiener prediction. Mitigations:
  slow adaptation, leaky/constrained updates, and excluding the vital band
  from the adaptation error.
* **Conditioning**: all anchors observe the same rank-3 motion subspace, so
  the multi-reference covariance is near rank-3 with extreme eigenvalue
  spread — whiten/PCA the anchor phases down to the motion subspace before
  the canceller and report the condition number (Widrow's Appendix C
  requires *linearly independent* references).
* **Precedent says deterministic first**: Nakata's deterministic compensation
  beat his LMS by 2.8 dB and produced the only flown 26 dB SIR figure;
  Ishmael's deterministic NIC matched his RLS-ANC in every reported test.
  RBEC therefore runs ANC as a **residual stage on the LS output** (small
  filters, 1–5 taps at 20 Hz — the beams share one clock so coupling is
  essentially instantaneous; step size from the Widrow 1976
  gradient-noise/lag tradeoff, swept and reported since the field publishes
  no recipe), and as an ablation against the deterministic stage — not as
  the headline method.
* **Reference conditioning** follows Ishmael's flown discipline: full
  arctangent demodulation, IQ-imbalance (Gram-Schmidt) and DC-offset
  correction, robust unwrapping on *every* beam before any cancellation —
  this is what upgraded the field from rate-only recovery to sub-mm
  displacement recovery under >100 mm platform motion.
* An LCMV variant of the reference beams (unit gain on the anchor, explicit
  **null on the casualty's steering vector**; diagonal loading + sector
  constraints per Lai 2026) is the beamforming-domain cure for leakage —
  evaluated as an option, with the known adaptive-beam self-nulling risk
  under nonstationary covariance flagged (§G).

---

## Part E — Why same-cube references cancel what dual-radar cannot

The physics rests on one verified transfer function and one verified hardware
fact **[prim]**.

**E.1 Range correlation.** Mixing an echo against its own LO filters LO phase
noise by 4 sin²(πfτ) ≈ (2πfτ)², τ = 2R/c (Budge & Burt 1993; vital-signs
derivation Droitcour 2004, verified to ~5 dB experimentally). Worked numbers
**[calc]**: τ = 33–200 ns for 5–30 m; suppression −134 dB (5 m, 1 Hz) to
−92 dB (30 m, 20 Hz). Oscillator phase *noise* is therefore already ~90+ dB
below the signal across the entire vital band before RBEC does anything —
the common-mode argument is about deterministic *steps* and drift, not noise
floor.

**E.2 The two-cell difference.** The target-minus-anchor phase sees every
shared-LO artifact through |H₁−H₂|² = 4 sin²(πf(τ₁−τ₂)) — dependence on the
**range difference only**. A pure LO phase step cancels in dechirp after a
~ns transient; a frequency step δf leaves 4π·δf·(R_t−R_a)/c (0.021 rad for a
pessimistic 100 kHz step at ΔR = 5 m; 2×10⁻⁴ rad at 1 kHz) **[calc]**.
Design rule that no physical-reference scheme can copy: **choose anchors
co-range with the casualty** — same range ring, different azimuth — and the
residual of every shared-LO artifact class collapses further. (Realistic
scenes may not offer co-range anchors; the budget in §F is evaluated at
ΔR of several metres, not the best case.)

**E.3 Where the APLL actually sits.** In the cascade, only the **master**
generates the 19–20.25 GHz chirp-modulated LO, distributed delay-matched to
all four chips; the APLL is the clean-up PLL feeding that synthesizer
**[prim, SWRA574B + datasheet §8.3.1.1]**. Therefore: the master's
non-disableable 1 Hz APLL/VCO recalibration is **common-mode across all 192
virtual channels** for its LO component. What does *not* cancel: slave-chip
APLL steps (they clock that chip's ADCs — per-chip, bounded ≈ 5 mrad in the
beam difference for the 0.55 ns sync-quantisation worst case at ΔR = 2 m
**[calc]**), and LO-distribution buffer bias steps (asymmetric across
cross-chip virtual TX/RX pairs — a channel-domain effect suppressed by using
**identical weight magnitudes per chip** in target and reference beams, since
TI documents calibration steps as common across all chains of one MMIC).
The honest claim, stated once and repeated in the abstract: *RBEC neutralizes
the shared-LO component of the E10 risk and bounds the per-chip remainder —
it does not blanket-cancel instrument artifacts.* Amplitude steps (2 dB per
RX gain code) obey none of this algebra; frames adjacent to calibration
events are flagged regardless.

**E.4 The configuration baseline** is TI's own cascade-coherence playbook
(survey B.1): factory RF-INIT with save/restore, all optional runtime cals
disabled, one-time-calibration with host-scheduled temperature-index
overrides — leaving the APLL/VCO pair as the only involuntary event, which
is exactly the case RBEC handles. Frame timing must leave the ICD-required
inter-frame idle (≥1 ms blank per CALIB_MON_TIME_UNIT for APLL+SYNTH) or
the artifact gets worse, not better.

**E.5 The centerpiece experiment.** Subscribe to
`AWR_RUN_TIME_CALIB_SUMMARY_REPORT_AE_SB` (per-event timestamp, die
temperature, hardware-updated flag — already a Phase 10 `device_state`
requirement) and demonstrate: single-beam phase shows the 1 Hz line/steps;
the RBEC beam difference removes it; the removal correlates event-by-event
with the logged calibration reports. This turns E10 from a risk measurement
into a *method validation* — and it is cheap, static, and ground-based.

---

## Part F — Budgets

**F.1 The requirement, stated in-band.** The cardiac target is 0.1 mm
(0.33 rad) **residual in the 0.8–3 Hz band**, and respiration 1–12 mm in
0.1–0.5 Hz — not broadband RMSE. Published sub-mm figures are broadband over
10–20 s; an in-band spec is both more honest and easier to meet, since
anchor-error energy concentrates where platform motion has energy.

**F.2 Against the published floor.** The best flown deterministic residual is
0.87 mm RMS (indoor, wall anchors, angle-aware WLS); best simulation
0.459 mm **[prim, Stöckel]**. The cardiac target exceeds the state of the
art by ~5–9×. RBEC's paper-level case for closing that gap, term by term:
(i) T3, the dominant term, shrinks with the 1.35° aperture (2.7× on bin
width), off-grid anchor DoA over 192 channels, and a corner reflector that
collapses both the angle and SNR error terms at once; (ii) chirp-rate
tracking removes the unwrap-reset data losses; (iii) the in-band spec
discounts out-of-band residual; (iv) the ANC residual stage attacks what the
LS leaves; (v) common-mode rejection removes the instrument's own in-band
line. **This is a target with a budget, not an assumption** — and the
respiration-grade claim (beat 1.33 % rate error and 0.87 mm residual) is the
near-term, defensible one. No airborne cardiac recovery exists at any band
above 7 GHz UWB ≤ 5 m (survey Part H); RBEC does not pretend otherwise.

**F.3 The leakage budget (T8), computed before flight.** Array theory with
the SPRACV2 ±2–3° post-cal residual **[prim + calc]**: the random-error
sidelobe floor of the 86-element array is −48.5 dB (2°) to −45 dB (3°) mean,
−40 to −37 dB peak at 90 % yield — so the **chosen taper dominates**: a
−30 dB Chebyshev/Taylor design is realizable, and tapering pays down to
about −35/−40 dB and no further. The rule that follows: every reference
anchor must differ from the casualty in **both** angle (≥ first-null
separation, ≥30 dB) and range bin (range-window sidelobes multiply — Hann
first sidelobe −31.5 dB → combined ≥60 dB), **or** carry an anchor echo
≥20 dB above the chest echo (the corner reflector again). With equal echoes
and angle-only separation the leaked respiration is ~10 % of cardiac
amplitude — quantified, tolerable, and reported; with a 20 dB-weaker anchor
it equals the cardiac signal and the method self-cancels — prohibited by the
admission rule. Caveat carried in the paper: the σ²/N floor assumes i.i.d.
per-channel errors; the cascade's residual is partly *correlated* per chip,
which can produce discrete spurious lobes above the floor — a Monte Carlo
with the measured calibration vector is required before trusting the floor
(§H, V1).

**F.4 Compute.** One steered beam at one range-Doppler cell = 192 complex
MACs. Ten beams × ~4 range bins at 20 Hz ≈ 1.2 MFLOP/s — 0.07 % of the
already-trivial 1.8 GFLOP/s classic DSP chain and strictly cheaper than the
angle FFT already budgeted; the LS solve and a 9-reference, few-tap ANC are
sub-kFLOP/s **[calc]**. RBEC adds no compute risk; one paragraph in the
paper, then move on.

**F.5 Why not odometry.** Doppler/scan-matching ego-motion integrates
velocity: best per-scan σ ≈ 1.6 cm/s, best drift 0.51–0.61 % of distance —
centimetres over 30 s. The equivalent velocity bias for 0.1 mm over 30 s is
3.3 µm/s, ~5000× below the demonstrated noise **[prim + calc]**. Anchor
interferometry is position-referenced and drift-free by construction; that
one line kills the odometry route and belongs in the paper's related work.

---

## Part G — Failure modes (what the paper must say out loud)

| Failure | Mechanism | Mitigation / disclosure |
|---|---|---|
| Flat featureless ground | Coplanar anchor LOS tips → singular solve (infinite DOP) | Declared operational limit; deployed reflector; per-axis quality flags; degrade to observed-axes-only compensation |
| Downwash-driven ground motion (T11) | Coherent across anchors and correlated with the platform — **geometry cannot separate it from ego-motion**; the survey's D8 question, unmeasured anywhere | Rigid/elevated anchors weighted up; rotor-state regressors; D8 measures the magnitude; stated as the honest unknown it is |
| Casualty leakage into anchors | Widrow's exact 1/ρ_ref law; GSC signal-cancellation analog; the scene-dependent-lag failure that killed correlation autofocus (SAND-91-0106C) | §C.4 veto + §F.3 budget + optional LCMV casualty-null; leakage coherence measured per anchor in-band |
| Sub-degree attitude jitter | At 1.35° beams, tenths of a degree move anchors across beams — worse than the prior art's 3° system by construction | Mandatory gyro-driven beam re-steering per frame; bench rotation tests (not injection — injection cannot create this) |
| Anchor migration | 3.75 cm range bin; r·tan(1.35°) azimuth PSF | Peak tracking + PPP-style range-shift correction; re-association; availability accounting |
| Unwrap integer errors | A single missed cycle = 1.9 mm step = 20× cardiac | Chirp-rate tracking; IMU-seeded integer fixing; RAIM-style anchor outlier exclusion; declared IMU dependency |
| Low anchor SNR | PGA threshold effect — below threshold the estimate is "hopelessly garbled", not degraded | D_A admission; SNR floor published with the results |
| Adaptive-beam self-nulling | LCMV with signal in the sample covariance under nonstationary platform motion | Deterministic beams as default; diagonal loading; LCMV as evaluated option only |
| Correlated calibration errors | Per-chip correlated residuals break the σ²/N sidelobe floor | Monte Carlo with measured cal vectors (V1); matched per-chip weights |
| Two-ray multipath anchors | A ground-bounce anchor has a different sensitivity vector; fading near nulls | Anchor plausibility checks (reject sub-ground apparent ranges — Stöckel); D_A instability catches fading anchors |

---

## Part H — Validation ladder (each rung with published precedent, except the two that are the point)

| Rung | What | Precedent / novelty | Pass criterion |
|---|---|---|---|
| V1 | **Injection study** on recorded static dwells: per-cell phase remodulation φ_k += (4π/λ)uₖ·d_syn with the published error-waveform taxonomy (quadratic, Wiener-from-IMU, GPS-discontinuity sinusoids, measured hover spectra, 1 Hz APLL-step surrogate) | SAR-autofocus standard practice (Doerry; Sensors 2021) transplanted; per-cell remodulation of real MIMO datacubes is **unpublished** | RBEC recovers injected d(t) to budget; common-mode events injected identically into all beams leave the residual unchanged. Validity domain stated: phase-only exact for \|d\| ≪ 3.75 cm bin (envelope-shift above ~4 mm); per-channel spherical wavefronts inside the 14.3 m Fraunhofer distance; **cannot** test rotation, multipath, or beam-gain modulation |
| V2 | **Bench**: radar on an encoder-fed linear stage (Griffin LNS-100 class, ±2–10 µm calibrated) facing a laser-vibrometer-calibrated chest phantom (Marty precedent: 0.08 mm commanded / 0.079 mm measured) + corner reflector + wall anchors; separate pan/tilt stage for rotation runs | Lubecke-line topology upgraded with the only phantom class that reaches ~10 µm; amplitude ladder 1.25/5/10 mm p-p (Ishmael-comparable) **plus a 0.08–0.3 mm cardiac tier** | Displacement error vs stage ground truth; Nakata SIR dB + Ishmael displacement-% metrics; rotation runs quantify T2/beam-migration handling |
| V3 | **Bungee-suspended UAV**, rotors on | Ishmael's cheap intermediate — real airframe, real vibration, no crash risk | Same metrics under real rotor spectra; D7 alignment |
| V4 | **Hexapod IMU-replay HIL**: replay recorded PX4 ULog hover trajectories on a 6-DoF hexapod carrying the cascade (COTS: PI H-840 class); inertial shaker adds the high-frequency band no hexapod covers | **Unpublished** — nobody has replayed flight IMU under a radar; fidelity metric defined as replay-PSD coverage vs measured hover PSD against the λ/4 = 0.97 mm threshold | RBEC residual in-band vs the same trajectory in simulation (V1) — closes the sim-to-bench loop |
| V5 | Rotor-on ground + downwash surfaces | Extends D7/D8 | T11 magnitude measured; anchor-weighting rules validated |
| V6 | **Tethered hover A/B**, extending the survey's D10: RBEC vs single-anchor subtraction vs IMU-aided vs combinations, landed control in-session | Stöckel's protocol reproduced for comparability (his 0.87 mm / 1.33 % to beat) | Residual-motion RMS on a static test object; respiration error vs chest strap; **the residual spectrum in 0.8–3 Hz — which no paper in this line has ever published — reported regardless of outcome** |

Recording requirements land almost entirely on what the harness already
mandates (R11 provenance, `device_state` cal events, HTE time base, bracket
IMU); RBEC adds: per-dwell anchor-beam definitions (steering vectors,
weights, admission scores), per-frame anchor tracks, and the LS/ANC
diagnostics — all recorded so any live compensation is reproducible offline,
per the harness's non-negotiables.

---

## Part I — Relation to prior art, in one table

| Work | What it did | What RBEC takes | What it lacks that RBEC adds |
|---|---|---|---|
| Stöckel et al. TRS 2024 / Sensors J 2024 / TAES 2023 / thesis 2025 **[prim]** | Flew wall-anchor phase LS at 77 GHz under a UAV (98 % removal, 0.87 mm residual); angle-aware WLS; per-DoF bench; named MIMO as future work | The estimator, anchor machinery, unwrapper, veto, protocol | Simultaneous digital beams (no 40 Hz mechanical revisit, no intra-frame timing skew), 1.35° aperture vs 3.6°, 3-D in flight, instrument common-mode analysis, cardiac-band residual reporting |
| Ishmael TMTT 2025 / Islam EuMC 2020 / Nakata 2016-18 **[prim]** | Dual-radar reference cancellation; flown sub-mm displacement under >100 mm motion; 26 dB SIR (deterministic) | Reference conditioning; metrics; the deterministic-first lesson | No second radar: reference beams from the same cube (shared LO/APLL/clock — dual radars share nothing); N anchors and a geometric solve instead of one reference |
| PGA lineage (Sandia) + NRL 94 GHz vibrometry **[prim]** | Optimal common-phase estimation; µm vibrometry from a moving car via autofocus cascade | The LUMV kernel, iteration architecture, noise laws | Space-variant vector solve with beams as the redundant channels; flying platform; vital-band spec |
| Han & Hong EuRAD 2022 | Differential beams on one chest cancel common-mode motion (~18 dB per a 2025 review) | The beam-differencing germ | Scene anchors, ego-motion solve, 192 channels, instrument common-mode. (Original 4-page text unread — paywalled; the 18 dB figure is second-hand and cited as such) |
| Cardillo TMTT 2021; Raytheon US 7,978,124 | Same-radar clutter referencing (range-bin, deterministic, static/handheld) | The same-instrument germ | Beams not bins; LS over N anchors; co-range selection; flight |
| PS-InSAR (Ferretti 2001); AutoCalib 2025 **[prim]** | Phase-stable scatterer selection; natural anchors ≈ 96 % of corner reflector on this cascade | D_A metric; anchor ranking | Applied to ego-motion cancellation at 20 Hz slow-time |
| GNSS (Langley GDOP; LAMBDA; Doppler cycle-slip repair) **[prim/partial]** | Geometry covariance; integer ambiguity machinery | The solve's error analysis and unwrap seeding | The radar transplant |

---

## Part J — What would falsify RBEC

1. **E10 measures a large per-chip (slave) APLL phase step** — the
   common-mode argument covers the master path only; if slave steps are
   µrad-claimed but mrad-measured, T6 re-enters the cardiac budget.
2. **D8 finds downwash-driven ground motion at mm scale over typical
   surfaces** — T11 is common across anchors and unremovable by geometry;
   RBEC degrades to rigid-anchor-only scenes.
3. **Anchor angle error resists super-resolution** — if off-grid DoA on
   ground clutter cannot beat ~0.3° effective error, T3 alone exceeds the
   cardiac budget at realistic sway and RBEC stays respiration-grade.
4. **Leakage coherence in-band exceeds the −30 dB budget** in real scenes
   (correlated calibration lobes, multipath) — the ANC stage then notches
   the vital band and the deterministic stage inherits a biased solve.
5. **The V4 HIL shows the 20 Hz + chirp-rate tracking still loses integers**
   under real hover spectra — the method needs a higher frame rate, which
   collides with the capture-mode table in the transport document.

Each of these is measurable on the ladder *before* the airborne phase — the
same design philosophy as the rest of Phase 10: the harness exists so that a
strong claim like RBEC can be killed cheaply, or earned.

---

## Part K — Sourcing disclosure

All claims tagged **[prim]** were verified against primary documents opened
in the 2026-08 research passes (per-pillar records preserved in the session
research archive; key documents: Stöckel TRS 2024 / Sensors J 2024 / TAES
2023 author PDFs + 2025 thesis (CC BY), Ishmael TMTT 2025 (NSF PAR open
copy), Widrow 1975/1976 (Stanford PDFs), Sandia PGA reports (OSTI),
Ferretti 2001, Langley 1999, Ren arXiv:2411.09201, Lai 2026, AutoCalib
arXiv:2506.23472, Doerry OSTI 919639, Marty arXiv:2309.08317, TI SWRA574B /
SPRACF4C / SPRACV2 / AWR2243 datasheet / mmWaveLink ICD). Known gaps, to
resolve before any camera-ready external version: Budge & Burt 1993 and
Droitcour 2004 verified via two independent open transcriptions (originals
paywalled — note there are *two* distinct Budge & Burt 1993 papers);
Han & Hong EuRAD 2022 and Stöckel EuCAP 2023 read at abstract level only;
Teunissen 1995 (LAMBDA) and Haykin's adaptive-filter results cited at
textbook level; Kaplan & Hegarty not directly opened (Langley used instead).
Citation hygiene: IEEE spells the Fraunhofer first author "Stockel".
