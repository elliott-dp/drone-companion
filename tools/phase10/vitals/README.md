# Vitals estimator bank (track 2)

The survey's B.7–B.9 stages made executable, NumPy-only: phase conditioning
(successive differencing + TI-style impulse removal, both `[prim]`-verified
against TI's shipped chain), band separation with harmonic cancellation
(k = 2…5, vs TI's k = 2 only), the estimator set (windowed/zoomed FFT peak,
autocorrelation, inter-peak intervals, decimated single-channel MUSIC), the
TI-style per-estimator confidence, and cross-estimator cluster fusion with
band-appropriate tolerances.

```bash
python3 -m tools.phase10.vitals.selftest
```

The self-test is the survey's hard case: respiration harmonics through the
5th overlaying a 30× weaker cardiac line. Gates: respiration < 1/min,
cardiac < 3/min; the no-notch ablation is printed to show harmonic
cancellation is load-bearing (~25/min error without it). Three design bugs
were caught by this test and are documented in the code: integration
random-walk biasing the fundamental, a fusion tolerance loose enough for a
bad estimate to drag the cluster, and sub-period MUSIC aperture in the
respiration band.

`bank.py` assembles the full chain (`process_window` / `sliding`), now
including VMD (spec-faithful ADMM per Dragomiretskiy-Zosso) and HMUSIC
(per arXiv:2408.01951, with a declared sum-of-reciprocals deviation), plus
`cw.py` ellipse-fit I/Q correction for CW radars.

Benchmark on the Erlangen clinical dataset (24 GHz Six-Port CW, clinical
ECG reference, CC BY 4.0 — figshare DOI 10.6084/m9.figshare.12186516):

```bash
python3 -m tools.phase10.vitals.exp_erlangen <subjects_zip> --probe   # once
python3 -m tools.phase10.vitals.exp_erlangen <subjects_zip> --scenario Resting
```

Dataset decisions from the 2026-08 scout: Erlangen first (direct HTTPS,
per-10-subject zips ~2 GB); the Twente Polar-H10 set is deferred — it
ships as a SINGLE ~181 GiB RAR with no per-subject access; the 2026
Zenodo 110-participant 60 GHz set is a noted candidate. Fixture
redistribution: Erlangen is CC BY 4.0 (fixtures OK with attribution);
Twente is CC BY-NC-SA.
