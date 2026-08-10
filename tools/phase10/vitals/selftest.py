"""Synthetic self-test for the estimator bank.

The hard case from the survey (B.8): respiration with harmonics whose 5th
lands on the cardiac band and exceeds the cardiac line in amplitude; the
bank must still recover both rates, and harmonic cancellation must be the
thing that makes cardiac recovery possible.
"""

from __future__ import annotations

import numpy as np

from .dsp import (CARDIAC_BAND, RESP_BAND, bandpass, estimate_then_notch,
                  remove_impulses, successive_diff)
from .estimators import autocorr, fft_peak, fuse, music, peak_intervals

K_DISP = 4 * np.pi / 3.794e-3      # rad per metre at 79 GHz


def synth(fs: float = 20.0, dur: float = 30.0, resp_hz: float = 0.27,
          card_hz: float = 1.23, seed: int = 1):
    rng = np.random.default_rng(seed)
    t = np.arange(0, dur, 1.0 / fs)
    resp_m = 4e-3 * (np.sin(2 * np.pi * resp_hz * t)
                     + 0.35 * np.sin(2 * np.pi * 2 * resp_hz * t + 0.6)
                     + 0.20 * np.sin(2 * np.pi * 3 * resp_hz * t + 1.2)
                     + 0.10 * np.sin(2 * np.pi * 4 * resp_hz * t + 0.3)
                     + 0.06 * np.sin(2 * np.pi * 5 * resp_hz * t + 2.0))
    card_m = 1e-4 * (np.sin(2 * np.pi * card_hz * t)
                     + 0.4 * np.sin(2 * np.pi * 2 * card_hz * t + 0.4))
    phase = K_DISP * (resp_m + card_m)
    phase += 0.02 * rng.standard_normal(t.size)          # phase noise
    # a few impulses (the TI chain's motivating artefact)
    for i in rng.integers(10, t.size - 10, 4):
        phase[i] += rng.choice([-1, 1]) * 3.0
    return t, phase, resp_hz, card_hz


def run_bank(phase: np.ndarray, fs: float):
    clean = remove_impulses(successive_diff(phase), thresh=1.5)
    # integrate the differenced signal back (cumsum) for band work, then
    # kill the random-walk tail the integration reintroduces — without
    # this the walk spectrum inside the respiration band biases the
    # fundamental estimate low, which mislocates the harmonic notches
    # (found by this very self-test)
    x = bandpass(np.cumsum(clean), fs, (0.05, 8.0))
    resp_est = [fft_peak(x, fs, RESP_BAND),
                autocorr(x, fs, RESP_BAND),
                peak_intervals(x, fs, RESP_BAND),
                music(x, fs, RESP_BAND)]
    resp = fuse(resp_est, tol_hz=0.03)
    card_sig = estimate_then_notch(x, fs, resp.rate_hz)
    card_est = [fft_peak(card_sig, fs, CARDIAC_BAND),
                autocorr(card_sig, fs, CARDIAC_BAND),
                peak_intervals(card_sig, fs, CARDIAC_BAND),
                music(card_sig, fs, CARDIAC_BAND)]
    card = fuse(card_est, tol_hz=0.08)
    return resp, card, resp_est, card_est


def main() -> None:
    ok = True
    for seed in range(1, 6):
        fs = 20.0
        _, phase, resp_hz, card_hz = synth(seed=seed)
        resp, card, resp_est, card_est = run_bank(phase, fs)
        r_err = abs(resp.rate_per_min - resp_hz * 60)
        c_err = abs(card.rate_per_min - card_hz * 60)
        # no-notch ablation: cardiac estimation on the plain band-passed
        # signal, to show the harmonic cancellation is load-bearing
        x = np.cumsum(remove_impulses(successive_diff(phase)))
        card_nn = fuse([fft_peak(bandpass(x, fs, CARDIAC_BAND), fs,
                                 CARDIAC_BAND)])
        nn_err = abs(card_nn.rate_per_min - card_hz * 60)
        print(f"seed {seed}: resp {resp.rate_per_min:5.2f}/min "
              f"(err {r_err:4.2f}, conf {resp.confidence:.2f}) | "
              f"cardiac {card.rate_per_min:6.2f}/min "
              f"(err {c_err:4.2f}, conf {card.confidence:.2f}) | "
              f"no-notch err {nn_err:5.2f}")
        ok &= (r_err < 1.0) and (c_err < 3.0)
    print("SELFTEST", "OK" if ok else "FAIL",
          "(gates: resp <1/min, cardiac <3/min)")
    raise SystemExit(0 if ok else 1)


if __name__ == "__main__":
    main()
