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

Next (pending the dataset scout): loaders + benchmarks on the public
Twente (77 GHz FMCW, Polar H10) and Erlangen (24 GHz CW, clinical TFM)
datasets, VMD and HMUSIC per their primary specifications, and fixture
export for the 10.3 parity tests.
