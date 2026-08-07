# Signal-processing and ML survey: what could work, what to test, what to skip

Companion to [`phase10_radar_harness.md`](phase10_radar_harness.md).

**This document deliberately implements nothing.** It maps the design space so
that pipelines built on the harness start from the state of the art rather than
from scratch, and so that effort goes where it can actually pay. Read §E ("what
not to test") before §D — the fastest way to waste this payload is to work on the
wrong stage.

Tags: **[calc]** arithmetic here · **[std]** standard, textbook or
universally-used practice · **[corrob]** multiple independent search extracts,
primary paper not read here · **[unver]** single source or inference ·
**[open]** genuinely unresolved.

---

## Part A — The five numbers that constrain every algorithm

| # | Fact | Consequence |
|---|---|---|
| A1 | 4π/λ = **3.311 rad/mm** at 79 GHz; phase wraps every λ/2 = **1.897 mm**; one-sided unambiguous λ/4 = **0.949 mm** **[calc]** | Respiration (1–12 mm → 3.3–39.7 rad) *always* wraps; cardiac (0.1–0.5 mm → 0.33–1.66 rad) *never* does. Unwrapping is a respiration problem only. |
| A2 | Unwrapping is valid only while true inter-frame motion < λ/4, i.e. chest-wall velocity < **λ·f_frame/4 = 19 mm/s at 20 Hz** **[calc]** | A dropped frame injects an unrecoverable 2πk error. **Segment on gaps; never bridge them.** |
| A3 | Rate resolution = 1/T: **2.34 BPM at 25.6 s**, 2.0 at 30 s, 1.0 at 60 s **[calc]** | A heart-rate estimate needs a ~26–30 s gap-free coherent dwell. Everything about the concept of operation follows from this. |
| A4 | σ_d = (λ/4π)/√(2·SNR) = 302 µm/√(2·SNR), growing as R² **[calc]** | Sensitivity is SNR-determined, not a hardware constant. Rescued by slow-time integration: 600 samples = **27.8 dB** **[calc]**. |
| A5 | Hover station-keeping is 331 rad (RTK) to 4967 rad (GNSS) of phase; lever arm 20 cm × 0.5° = 5.78 rad; rotor vibration at 30 m/s² is 0.05–0.25 rad and falls as 1/f² **[calc]** | Platform motion exceeds the cardiac signal by ~10³–10⁴. **Ego-motion compensation is the whole game**, and it is a *low-frequency* problem, not a vibration problem. |

Everything below is either a way to exploit A1–A4 or a way to survive A5.

---

## Part B — The canonical chain, stage by stage, with the real alternatives

```
raw ADC → [calibration] → range FFT → [MIMO demux + Doppler-phase correction]
   → [clutter/static removal] → [localisation: CFAR + angle] → [cell selection]
   → phase extraction → [ego-motion compensation] → band separation
   → rate estimation (multi-estimator) → confidence fusion → report
```

### B.1 Calibration (do this first or nothing else means anything)

A 4-chip cascade is only an aperture after per-channel amplitude/phase calibration;
TI ships a calibration procedure and a calibration matrix for exactly this
**[corrob]**. Three distinct corrections, often conflated:

1. **Inter-channel (TX×RX) phase/gain** — from a calibration capture of a known
   point target; without it beamforming produces grating lobes and biased angles.
2. **Frequency-dependent** correction across the sweep (the calibration is valid
   for a specific profile — **so a profile change invalidates it**, which is why the
   harness hashes both together and refuses a mismatch).
3. **Temperature drift** — phase versus die temperature, and TI documents a
   periodic APLL/VCO recalibration **[unver]**. If that recalibration lands a step
   in the slow-time phase it sits *inside* the cardiac band (test E10 in the
   transport document). This is the single most under-appreciated risk in the whole
   chain.

**Test, don't assume:** capture a corner reflector at a surveyed position at the
start and end of every session, and store the residual. The residual over a session
*is* the calibration-stability measurement.

### B.2 Range processing

Standard **[std]**: DC removal per chirp, window (Hann/Blackman-Harris — the
sidelobe level matters much more than usual here, because a 0.33 rad cardiac
signal can hide under the sidelobes of a huge clutter return), zero-padded FFT for
finer bin interpolation, then complex output retained. Choices worth deliberating:

* **Window choice is a clutter-leakage decision.** With ~35 dB of clutter-to-target
  ratio in a ground-looking geometry, a −31 dB-sidelobe Hann may not be enough;
  Blackman–Harris (−92 dB) costs resolution and is usually the right trade for
  vitals **[std]**.
* **Range resolution is nearly irrelevant to the vitals measurement itself** (phase,
  not range, carries the signal) but is *very* relevant to separating the subject
  from ground clutter and from another body. Choose bandwidth for separation.
* Keep the complex spectrum; magnitude-only destroys everything downstream.

### B.3 MIMO demux and the TDM Doppler trap

With 12-TX TDM each transmitter is sampled at a *different instant* within the
frame, so a target with radial velocity imprints a progressive phase across the TX
dimension that masquerades as an angle **[std]**. Every cascade imaging pipeline
must apply Doppler-phase compensation before angle estimation, and the correction
requires a velocity estimate — which is itself ambiguous beyond ±2.0 m/s at 12-TX
TDM with 40 µs chirps **[calc]**. Under platform motion that ambiguity is routinely
exceeded.

Implications:
* Record the **MIMO scheme and chirp order** per capture — a cube cannot be
  demuxed without it.
* For *vitals*, prefer the small-TX modes: the trap largely disappears and the data
  rate collapses (see the capture-mode table in the transport document).
* DDMA is sometimes proposed as the fix. It is not: at equal chirp period and TX
  count it has the **same** unambiguous velocity as TDM, and buys ~10.8 dB of
  integration/power instead **[calc]**. Choose it for SNR, not for ambiguity.

### B.4 Clutter and static-return removal

| Method | Notes |
|---|---|
| **Mean subtraction along slow-time**, per range bin | The community default; removes static background while retaining motion **[corrob]**. Also — usefully — exactly the transform that compresses best (see the dataset document). |
| High-pass / MTI along slow-time | Same family; explicit cut-off control. Beware: the respiration band starts at 0.1 Hz, so the cut-off must be *below* it. |
| **SVD / PCA subspace removal** | Drop the dominant singular components (usually clutter). Powerful, but it can eat the respiration component when the subject is the strongest thing in the scene — validate per geometry **[corrob]**. |
| DC-offset compensation on I/Q | Needed before arctangent demodulation; classic circle-fitting on the I/Q locus **[corrob]**. |
| **The catch** | Under random body movement (and platform movement) the "DC" term is *time-varying*, so no static calibration recovers it **[corrob]**. This is why ego-motion compensation must precede or be joint with phase extraction. |

### B.5 Localisation

* **CFAR** family (CA-, OS-, GO-/SO-CFAR) for detection; ordered-statistic variants
  are the usual choice with extended targets and clutter edges **[std]**.
* **Angle estimation**: FFT beamforming (cheap, resolution ≈ 1.35° with this
  aperture), Capon/MVDR (better resolution and null-steering at higher cost),
  MUSIC/ESPRIT (super-resolution, needs a source-count estimate and good
  calibration), compressive/sparse methods (best resolution, most fragile).
  Verified geometry: 42.5λ horizontal aperture → ~1.35° azimuth; 3λ vertical →
  ~19° elevation **[code, from a published cascade dataset's own parameters]**.
* **Elevation is ~2 cells across the FoV** **[calc]** — the array cannot separate a
  supine casualty from the ground plane. Height needs platform-motion synthetic
  aperture (requiring cm-class pose per chirp) or an assumed ground surface.
  Treat elevation as a nuisance parameter, not an observable.

### B.6 Cell selection for vitals (a bigger deal than it looks)

Candidate criteria, and none is obviously right:

| Criterion | Fails when |
|---|---|
| Max energy in range | The strongest bin is clutter, not the subject |
| Max phase variance | Platform motion and multipath fading also maximise it |
| Max spectral energy in the respiration band | Wind-moved vegetation and tarps also peak at 0.1–0.5 Hz |
| Harmonic-structure score | Weak when the subject is barely resolved |
| **Multi-cell fusion** (weighted combination across neighbouring range-angle cells) | More robust and now standard practice **[corrob]**; the right default |

Two harness-level consequences:
* **Do not select cells in flight.** Record all bins over a coarse beam set, and
  select offline where the decision can be revisited. This is why the live tier is
  explicitly not the record.
* A spatial-coherence requirement (the vital signature must be localised to the
  target cell and *absent* from neighbouring clutter cells) is one of the strongest
  available false-alarm guards.

### B.7 Phase extraction

| Method | Character |
|---|---|
| `atan2(Q, I)` + unwrap | The baseline **[std]**. Sensitive to DC offset and to the wrap-per-frame condition (A2). |
| **DACM** (differentiate-and-cross-multiply) / extended DACM | Avoids explicit arctangent branch discontinuities; differentiates first, integrates after **[corrob]**. Often more robust in practice. |
| Complex-signal demodulation | Handles the DC/null-point problem differently; worth benchmarking. |
| Successive phase differencing | Kills DC and slow drift and pre-emphasises the cardiac band (a differentiator has +6 dB/octave) — TI's own chain uses it **[corrob]**. Cheap and effective; note it also amplifies high-frequency noise, so it pairs with the band-pass. |
| Impulse-noise removal (forward/backward difference threshold + interpolate) | TI's chain does this **[corrob]**. **Distinguish carefully** from a missing frame: interpolating a *spike within received data* is fine; interpolating across a *dropped frame* is not (A2). |

### B.8 Band separation and the harmonic problem

Respiration at 0.2–0.5 Hz has harmonics; the 3rd–5th land at 0.6–2.5 Hz — i.e.
**directly on top of the cardiac band** and often stronger than the true cardiac
line, since respiration is 8–60× larger in amplitude **[calc]**. This, not
sensitivity, is the dominant error source for heart rate on a *stationary* subject.

Approaches, roughly in order of increasing sophistication:
* Cascaded band-pass IIR (TI's chain: separate breathing and cardiac biquads;
  TI's shipped cardiac band is **0.8–2.0 Hz = 48–120 BPM** **[corrob]** — too narrow
  for a tachycardic casualty, so widen deliberately).
* Explicit harmonic notching at k·f_resp once the respiration rate is known.
* **VMD** (variational mode decomposition) — extracts cardiac modes and avoids the
  mode-mixing that plagues EMD **[corrob]**.
* **EEMD/CEEMDAN** — adaptive decomposition, but documented mode mixing degrades
  heart-rate accuracy precisely because of respiratory harmonics **[corrob]**.
* Wavelet/DWT and CWT-based decomposition for clutter and noise suppression
  **[corrob]**.

### B.9 Rate estimation, and why to run several

| Estimator | Strength | Weakness |
|---|---|---|
| FFT peak | Simple, resolution 1/T | Harmonics, leakage, needs a long window |
| Autocorrelation | Robust to spectral leakage; reported ~93 % accuracy for both rates in one study **[corrob]** | Ambiguous at harmonics/subharmonics |
| Inter-peak interval (time domain) | Gives beat-to-beat variability | Very sensitive to impulse noise |
| **MUSIC / harmonic MUSIC (HMUSIC)** | Super-resolution; HMUSIC exploits the *harmonic structure* of the vital signal and is reported at 89th-percentile respiration error < 3 rpm and 88th-percentile heart error < 5 BPM **[corrob]** | Needs model order; sensitive to colouring |
| Chirp-Z / zoom FFT | Fine frequency grid without a longer window | Does not add true resolution (A3) |
| Kalman/UKF rate tracking | Smooths, enforces physiological continuity | Can mask real change; must not fabricate during gaps |

**Run three or more and fuse with an explicit confidence metric** — that is what
TI's chain does and what the better literature does **[corrob]**. Cross-estimator
agreement is itself the single most useful confidence feature, and it is exactly
the kind of quantity the existing `HealthFinding` culture in this repo already
knows how to express.

### B.10 Ego-motion compensation (the decisive stage for airborne work)

Ranked by expected value on this platform:

1. **Scene-referenced (static-clutter) phase reference.** Subtract the phase
   history of a static anchor cell (corner reflector, ground patch, structure) from
   the target cell. Common-mode platform motion cancels. A published finding that
   anchor-based compensation matches or beats IMU-based compensation supports this
   as the primary method **[unver]**, and A5 explains why: the lever-arm term alone
   needs 0.0086° attitude knowledge to reach 0.1 rad, which no multirotor EKF
   delivers **[calc]**.
2. **Range-migration compensation via inter-frame correlation**, then blind source
   separation (ICA) to split platform motion from physiology — the demonstrated
   airborne approach, at sub-6 GHz **[unver]**. Range migration is not optional at
   30 s: a 1 m drift crosses several 30 cm range bins.
3. **Reference-channel / dual-radar adaptive cancellation** — a second aperture (or
   beam) aimed at static ground provides the noise reference for an adaptive
   canceller **[unver]**. With 192 virtual channels, a *beam* can serve as the
   second channel at no hardware cost — an attractive, cheap experiment this
   aperture uniquely enables.
4. **IMU-aided, µs-aligned** — as a *coarse* pre-correction and a validity gate,
   not as the primary. Timing budget: −20 dB of a 100 Hz line needs 159 µs
   alignment; −30 dB needs 50 µs **[calc]**.
5. **Adaptive motion-artifact filtering (CWT-based)** for random *body* movement,
   which is a separate problem from platform movement **[corrob]**.

### B.11 Multiple subjects

Range-angle separation is the standard route: range FFT → angle FFT → extract phase
per range-angle cell **[corrob]**. Difference beamforming reportedly separates
subjects closer than the nominal angular resolution **[corrob]**. This aperture is
unusually well suited (1.35° azimuth ⇒ 24 cm cross-range at 10 m **[calc]**, i.e.
better than body width). But: **do not build this before single-subject works
landed** (§E).

### B.12 Detection of people, as distinct from vital signs

Two regimes, and they need different pipelines:

* **Moving person** — micro-Doppler gait signatures are well established across
  many datasets and bands **[corrob]**; a spectrogram + classifier is a solved-ish
  problem, and the cascade's aperture makes it easier than most published work.
  This is the *cueing* function and it should be the first working capability.
* **Motionless person** — this is the hard, valuable case, and it reduces to the
  vital-signs problem plus a presence decision. A body-shaped RCS with a
  physiological modulation is the only discriminator; hence the mannequin dwells in
  the dataset (a body without physiology) as a mandatory control.

Aspect matters more than people expect: a person's RCS varies by ~20 dB with
aspect **[corrob]**, so a single-threshold detector will flicker. Record per-detection
SNR/RCS and treat detection as probabilistic.

---

## Part C — Machine learning: where it pays, where it is a trap

### C.1 The landscape

| Approach | Relevance here |
|---|---|
| **Learning directly from raw ADC** (e.g. ADCNet-style, with a learnable/differentiable signal-processing front end, range FFT and windowing optimised jointly with the task) **[corrob]** | Directly applicable, and it is the strongest argument for archiving *raw* rather than products: only raw lets the front end be learned. |
| **Denoise-then-classify cascades** (a self-supervised denoiser feeding a classifier, trained end to end) **[corrob]** | The closest match to the stated interest in ML denoising. |
| **Self-supervised / contrastive pretraining** on unlabelled radar **[corrob]** | Very attractive here: the harness will produce far more unlabelled dwells than labelled ones. |
| **Cross-modal supervision** (a camera or reference sensor supervises the radar) **[corrob]** | The cheapest label source available: the published cascade datasets do exactly this with synchronised cameras **[code]**. Add a camera to the payload for labels even if the product never uses it. |
| Generative/GAN and diffusion methods for high-resolution imaging from degraded input **[unver]** | Interesting for imaging, high risk for a measurement whose output is a *number a medic will act on*. |
| Hardware-accelerated DSP front ends (automotive platforms put range/Doppler/angle FFTs in fixed-function hardware and leave the GPU for perception) **[corrob]** | Not available to us: **the Orin Nano has no DLA and no PVA** **[corrob]** — everything shares one 1024-core GPU. |

### C.2 Where ML sits in the pipeline, and the latency that makes it affordable

ML is **in** the chain, not beside it. The pipeline's terminal stage is a decision:
*is a human present?* — and only if that asserts do vital signs get released. The
enabling insight is that **this decision's deadline is the window or the dwell, not
the frame**:

```
20 Hz  ── range FFT · clutter removal · phase extraction · tracking     (classical)
1–2 s  ── sliding-window features → presence classifier                 (ML, soft)
~30 s  ── dwell commit: presence decision + vital signs + confidences   (ML, soft)
```

Published radar networks are far from 20 Hz — one self-supervised denoise+classify
network reports **0.26 s per sample**, a hybrid method **~1.7 s on a desktop GTX
1080 Ti** **[corrob]** — and that is fine here, because at 1 Hz the first costs
26 % of the GPU and at dwell cadence under 1 %. A model too slow for frame-rate
inference can be entirely appropriate for a decision the operator waits 30 seconds
for anyway. The full cadence arithmetic is in
[`radar_realtime_budget.md`](radar_realtime_budget.md) §C.3.

What that buys, and what it costs:

* **Buys:** big models are usable *online*, not just offline. A window-cadence
  classifier and a dwell-cadence committer can both be real networks.
* **Costs nothing in record integrity**, provided three things hold: the recorder
  never waits for the model; a late invocation is skipped and reported
  (`decision_age_ms`, `CC_VF_ML_STALE`) rather than answered stale; and the model
  hash is recorded so any live decision can be reproduced from the dataset.
* **Requires a three-state output.** "Undecided" must be expressible, because a
  classifier that has not yet seen 30 s of coherent data has genuinely not answered
  — and reporting that as "no human" is the failure mode that gets people missed.

### C.3 Where ML most plausibly pays off here — ranked

1. **Ego-motion / clutter suppression learned from paired data.** The harness gives
   something rare: the *same subject* recorded landed and airborne, in the same
   geometry (that is what the landed control is for). That is a supervised pair —
   airborne input, landed target — and it is a far better-posed learning problem
   than generic denoising. **This is the highest-value ML idea in this document.**
2. **Presence/absence classification** on short windows, cross-modally supervised
   by camera + mannequin/empty controls. **This is the pipeline's terminal stage**
   (§C.2) and it is robust to the fact that rate estimation is hard: knowing
   *someone is alive there* is most of the operational value, and it gates whether
   vital signs are released at all.
3. **Quality/confidence prediction** — learn to predict whether a dwell will yield
   a trustworthy rate, from the first few seconds. Operationally this is huge: it
   tells the pilot to keep hovering or move on, and it is a much easier target than
   the rate itself.
4. **Rate refinement** as a *residual* on top of a classical estimator, never as a
   replacement — so failures degrade to the classical answer.
5. Super-resolution / elevation inference — attractive, but see §E.

### C.4 Where ML is likely a trap

* **A black box that outputs a heart rate directly**, trained on tens of subjects.
  It will learn the dataset's geometry and subjects, and the failure mode is a
  confident wrong number. The repo's existing "no black-box ML; every finding
  traceable to a formula" doctrine exists for good reasons; radar rates should
  carry the same burden of traceability.
* **Learning to "enhance" phase.** Any generative model in the phase path can
  synthesise a plausible periodicity that was never there. If a denoiser touches
  the vital band, the empty-dwell false-alarm test is the only thing standing
  between it and a fabricated heartbeat — and it must be run on *every* model
  revision.
* **Training on simulation alone.** Useful for pretraining and for coverage of rare
  geometries, but the clutter and platform-motion statistics are exactly what a
  simulator gets wrong.

### C.5 Evaluation discipline (non-negotiable for either classical or ML)

* **Subject-wise splits.** Frame- or dwell-wise splits leak identity and produce
  fantasy accuracy.
* **Empty and mannequin dwells in the test set**, and report false-alarm rate as a
  first-class metric alongside error.
* **Report per-geometry**, not pooled: standoff × depression angle × posture.
  A pooled MAE hides that the system only works at 2 m nadir.
* **Calibrated confidence**: if the pipeline says 80 %, it should be right 80 % of
  the time. Reliability diagrams, not vibes.
* **Landed vs airborne, always paired.** The single most informative number this
  programme can produce is the *degradation* between them.

---

## Part D — What to test, in order

Ordered so that each step can kill the next one cheaply. Steps 1–4 need no aircraft.

| # | Test | Pass criterion |
|---|---|---|
| D1 | Static corner reflector, 60 s: phase noise floor and drift; look for the ~1 Hz APLL step | σ_d measured; no unexplained periodic step in 0.8–3 Hz, or the step characterised |
| D2 | Calibration stability across a session and across temperature | Residual phase drift quantified in rad and µm |
| D3 | Seated subject, tripod, 1 m, chest strap reference — the reproduction baseline | Respiration within ~1–2 rpm and heart within ~3–5 BPM of reference, matching what published work achieves **[corrob]** |
| D4 | Same, sweeping standoff 1 → 3 → 5 → 8 → 10 m, and angle/posture | The range at which heart rate fails, measured rather than argued. **This single curve determines whether the concept is viable.** |
| D5 | Empty dwells and mannequin dwells, same geometries | False-alarm rate measured; mannequin yields detection but no vitals |
| D6 | Vegetation/tarp/water dwells in wind | Characterise the 0.1–0.5 Hz confounders; verify the spatial-coherence guard rejects them |
| D7 | Rotor-on, aircraft on the ground, subject at 5 m | Isolates vibration + downwash from platform translation |
| D8 | Downwash on a mannequin, blanket, dust, foliage at 3/5/10 m | If apparent displacement exceeds ~0.5 mm, airborne vitals is in serious doubt regardless of compensation |
| D9 | Tethered hover, subject at 5 m, landed control in the same session | The degradation number; heartbeat likely survives only in hover **[unver]** |
| D10 | Ego-motion compensation A/B: scene-referenced vs IMU-aided vs both | Residual phase in rad, and rate error, per method |
| D11 | Multi-subject separation at 1 m and 0.5 m apart | Only after D3/D4 pass |
| D12 | Compression acceptance: re-quantise D3/D4 dwells offline at each k | The k at which rate estimates change measurably — sets the lossy policy |

---

## Part E — What *not* to test (and why)

Explicit non-goals. Each has cost someone a research programme.

| Don't | Because |
|---|---|
| **Run ML inference at frame rate (20 Hz)** | Not because ML is unaffordable — because it is unnecessary. The decision is a window/dwell product; frame-rate inference costs 20–100× more for an answer nobody can use faster (§C.2). |
| **Buried-victim detection at 77–81 GHz** | Physics. Concrete runs tens of dB/m and every fielded rubble/avalanche system uses 150 MHz–4 GHz **[corrob]**. This payload is a surface/line-of-sight sensor. |
| **Chasing sensitivity** — more virtual channels, more averaging, lower noise figure | At 5–30 m nothing is SNR-limited; the limits are clutter, platform motion and geometry (A4/A5). Spend the budget on uniform timing and clutter rejection. |
| **Heart rate from a translating platform as an early milestone** | Even at sub-6 GHz, published airborne work recovered breathing in motion but heartbeat only in hover **[unver]**; at 79 GHz the same motion produces 3.3–10.8× more phase **[calc]**. Hover first, or don't bother. |
| **TDA2-side signal processing to "offload the Jetson"** | The Orin's DSP cost is ~2 % of its GPU **[calc]**; the TDA2 route costs an EOL SDK, a firmware project, and the ability to change the algorithm **[corrob]**. |
| **Full-cube 3D deep networks in real time** | 2–10 TFLOP/s at 20 Hz on a device with no DLA/PVA **[calc]**. Offline only. |
| **Magnitude-only or pre-selected products as the archive** | Destroys the phase and bakes in in-flight decisions that ego-motion will have made wrong. |
| **DDMA expecting a velocity-ambiguity fix** | Same ambiguity as TDM at equal chirp period and TX count; it is a power/SNR trade **[calc]**. |
| **Elevation/SAR height estimation before pose accuracy is proven** | Needs cm-class pose *per chirp*; the aperture gives <2 elevation cells **[calc]**. |
| **Inheriting TI's 0.8–2.0 Hz cardiac band** | 48–120 BPM **[corrob]** excludes a tachycardic casualty — precisely the person who matters. |
| **Multi-person separation before single-person works landed** | Harder problem, same failure modes, no diagnostic value. |
| **Tuning thresholds on synthetic traces alone** | The clutter and motion statistics are what simulators get wrong; the repo already learned this lesson with the health algorithms' false-positive audit. |
| **Interpolating across dropped frames** | A2. It silently manufactures a phase error that looks like signal. |

---

## Part F — False-alarm catalogue (what will fake a heartbeat)

| Mechanism | Why it is dangerous | Structural guard |
|---|---|---|
| **Two-ray multipath fading** | λ/4 = 0.95 mm of platform motion sweeps a null; the fading is indistinguishable from breathing **[calc]** | Spatial-coherence requirement; anchor-referenced phase; empty-dwell FAR |
| Wind-moved vegetation, tarps, water | Sit squarely at 0.1–0.5 Hz | D6 dwells; spatial extent test (physiology is compact, foliage is not) |
| **Rotor downwash on fabric/foliage** | Correlated with the platform, so naive common-mode rejection *will not* remove it | D8; require a harmonic/cardiac structure, not just band energy |
| Aliased vibration lines | Land at `f_vib mod f_frame` | Record rotor state; predict and notch |
| Respiration harmonics | Land on 0.8–2.0 Hz and are often stronger than the cardiac line **[calc]** | Explicit harmonic cancellation; HMUSIC; cross-estimator agreement |
| Aircraft's own structure/props in near bins | Constant, strong | Range gating; static-anchor identification |
| A generative denoiser | Can synthesise periodicity | Empty-dwell FAR re-run on every model revision |

---

## Part G — Metrics to report (so results are comparable)

Respiration/heart MAE and 90th-percentile error against reference; per-geometry
breakdown; **false-alarm rate on empty dwells** and detection rate on mannequin
dwells; usable-dwell fraction (how often a 30 s coherent window was achieved);
time-to-first-estimate; confidence calibration; and the **landed→airborne
degradation** for every metric. Publish the failures: the range at which heart rate
stops working is the most useful number in the whole programme.

---

## Part H — Open questions worth a literature pass on an unblocked network

1. Any peer-reviewed mmWave vital-sign result beyond ~3 m, and what SNR/window it
   needed **[open]** — this is the single most decisive unknown for the concept.
2. Measured σ⁰ for grass, soil, gravel, rubble at 76–81 GHz across 20–90°
   depression, and human RCS *viewed from above* in standing/supine/prone poses.
   Both appear unmeasured in the accessible literature, and both are exactly the
   geometry this payload flies **[open]**.
3. AWR2243 phase noise and cascade coherency over a 30 s window — sets the
   achievable floor when everything else is perfect **[open]**.
4. Whether rotor downwash moves a clothed casualty enough to swamp respiration
   **[open]** — no literature found; D8 answers it.
5. The airborne compensation methods' portability from 7.3–24 GHz to 79 GHz
   **[open]**.
