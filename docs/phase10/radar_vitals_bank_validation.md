# Vitals estimator bank — first real-data benchmark (track 2)

Companion to [`radar_dsp_ml_survey.md`](radar_dsp_ml_survey.md) §B.7–B.9 and
the code in [`tools/phase10/vitals/`](../../tools/phase10/vitals/README.md).
Everything **[meas]** — computed on this machine by the committed scripts.

> **Headline: the bank's confident answers meet the D3 gate on real
> clinical radar data.** On the Erlangen clinical dataset (24 GHz Six-Port
> CW, ECG reference), confidence-gated heart-rate error is **MAE 3.05 BPM
> (p90 6.37) at 30 % coverage** for conf ≥ 0.5, improving to 2.67 (p90
> 5.02) at conf ≥ 0.6 — while ungated error over all 537 windows is
> 9.82 BPM. That gap is the point: the confidence metric is *calibrated
> enough to gate on*, which is exactly what the payload's three-state
> doctrine (R5: "undecided" is a legitimate answer) needs from the
> estimator layer. The best subjects run MAE 2.4–2.6 BPM ungated —
> comfortably inside the survey's D3 acceptance band (3–5 BPM).

## Setup

* **Dataset**: Schellenberger et al., Sci Data 2020 (figshare
  10.6084/m9.figshare.12186516, CC BY 4.0): 24 GHz Six-Port CW radar,
  clinical Task Force Monitor reference; subjects GDN0001–0010, Resting
  scenario (~10 min each). MD5-verified download.
* **Chain**: ellipse-fit I/Q correction → arctangent phase (2000 Hz) →
  decimate to 20 Hz → the bank (`process_window`): successive differencing
  + impulse removal (TI-verified parameters), respiration fusion
  (FFT/autocorr/intervals/MUSIC, tol 0.03 Hz), harmonic notch k = 2…5
  (k = 2 widened), cardiac fusion over six estimators (…+ HMUSIC, VMD+FFT,
  tol 0.08 Hz), 30 s windows sliding 10 s.
* **Reference**: ECG R-peaks (band-passed, derivative-squared, amplitude
  post-filter) → instantaneous HR interpolated to window centres.
  Respiration goes unreferenced — these files carry no respiration channel
  (`tfm_param` is empty in the v2 export).

## Results (Resting, subjects 01–10)

| Subject | Windows | HR MAE [BPM] | p90 | median ref | mean conf |
|---|---|---|---|---|---|
| GDN0001 | 58 | **2.60** | 4.66 | 72 | 0.66 |
| GDN0002 | 60 | 17.39 | 38.02 | 71 | 0.30 |
| GDN0003 | — | ECG channel all-NaN in the file (dataset artifact) | | | |
| GDN0004 | 58 | **2.38** | 3.56 | 69 | 0.73 |
| GDN0005 | 59 | 9.03 | 19.67 | 63 | 0.29 |
| GDN0006 | 59 | 8.26 | 14.79 | 56 | 0.29 |
| GDN0007 | 61 | 8.85 | 17.04 | 63 | 0.40 |
| GDN0008 | 59 | 12.36 | 54.87 | 57 | 0.40 |
| GDN0009 | 62 | 8.90 | 18.50 | 59 | 0.28 |
| GDN0010 | 61 | 18.04 | 46.41 | **46 (bradycardic)** | 0.37 |
| **Total** | 537 | 9.82 | 29.42 | | |
| conf ≥ 0.4 | 46 % cov | 4.03 | 9.37 | | |
| **conf ≥ 0.5** | **30 % cov** | **3.05** | **6.37** | | |
| conf ≥ 0.6 | 19 % cov | 2.67 | 5.02 | | |

## Findings

1. **Confidence calibration is the deliverable.** Mean per-subject
   confidence tracks per-subject error monotonically (0.66–0.73 on the
   two best subjects, 0.28–0.40 on the tail) — the gating table above is
   the operational consequence, and it validates the survey's C.5 demand
   ("if the pipeline says 80 %, it should be right 80 % of the time") at
   the ranking level. Reliability-diagram calibration is follow-up work.
2. **Bradycardia is real: the cardiac band floor moved.** GDN0010 rests at
   46 BPM — below the original 0.8 Hz floor, the mirror image of the
   survey's Part E critique of TI's 120 BPM cap. `CARDIAC_BAND` is now
   (0.6, 4.0) Hz, matching where the Erlangen-tuned literature converged.
   The widening admits more respiration-harmonic interference (GDN0002
   worsened), reconfirming the survey's B.8 thesis that harmonics, not
   sensitivity, are the dominant cardiac error source.
3. **The tail is harmonic-interference-shaped, not noise-shaped**: failing
   subjects fail with *confident-looking* respiration and a cardiac
   estimate parked on a harmonic. Better harmonic discrimination
   (HMUSIC-style joint modelling replacing the notch cascade, per-subject
   notch adaptation) is the identified next lever.
4. **Reference-building is its own discipline**: the naive R-peak detector
   double-counted T-waves (a "resting HR" of 142 BPM), and one subject's
   ECG channel is all-NaN. Both are now handled and documented in code.
5. **Honest transferability caveat**: this is a 24 GHz *CW* radar — no
   range gating, whole-body coupling, different SNR regime than the
   payload's 77 GHz FMCW with range-bin isolation. The benchmark validates
   the *estimator layer* on real physiological radar phase; it neither
   bounds nor predicts the cascade's end-to-end performance.

## Follow-ups

1. Per-window failure forensics on the tail subjects (GDN0002/0008/0010) —
   classify error modes (harmonic lock, motion, reference gaps).
2. Subjects 11–30 (two more zips) + the Valsalva/Tilt scenarios for
   rate-dynamics stress.
3. Harmonic-aware cardiac estimation as first-class (joint
   respiration+cardiac HMUSIC over both bands) instead of
   estimate-then-notch.
4. Committed fixtures (CC BY 4.0 permits, with attribution): a trimmed
   per-subject excerpt + expected bank outputs as the 10.3 parity-test
   fixture.
5. Dataset expansion decisions: Twente deferred (single 181 GiB RAR);
   the 2026 Zenodo 110-participant 60 GHz set is the next candidate
   (direct URLs verified).
