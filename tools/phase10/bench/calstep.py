"""Test E10 / D1: detecting and quantifying the ~1 Hz APLL/VCO calibration step.

The problem, stated precisely. The AWR2243's APLL and synth-VCO runtime
calibrations run at 1 s periodicity, execute in the inter-frame idle, and
**cannot be disabled**. TI documents abrupt gain/phase steps from runtime
calibration updates but never quantifies this one, and 1 Hz sits inside the
0.8–3.0 Hz cardiac band. So the residual has to be measured, not assumed.

Detection model. A calibration event that shifts the RF phase by Δφ appears in
the slow-time phase record as a **step**, hence as a single-sample **impulse of
height Δφ in the first difference**. With the event times known (they are —
``ENABLE_CAL_REPORT`` delivers a timestamped report per event), the estimator is
a synchronous average of the differenced phase over the event epochs, which
beats a blind 1 Hz spectral search by √N and, more importantly, does not
confuse a real step with any other 1 Hz-periodic bench artefact.

Two estimators are provided:

* :func:`fold_at_events` — the preferred one, using logged cal-event timestamps.
* :func:`fold_at_period` — a fallback when timestamps are unavailable, folding
  at a hypothesised period. Weaker: any bench vibration at that period folds in
  identically, which is exactly the trap that makes a null result meaningless.

A third, assumption-free detector (:func:`page_hinkley`) locates unmodelled
change points, mirroring the change-point statistics already used by
``cc-ai-health``.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np

from ..rbec.core import K_DISP  # rad per metre of LOS displacement at 79 GHz

# Two-sided detection at alpha = 0.01 with 99 % power: z(0.995) + z(0.99).
Z_ALPHA_2S_001 = 2.5758
Z_POWER_099 = 2.3263


@dataclass
class FoldResult:
    n_epochs: int
    step_rad: float          # estimated Δφ at the event sample
    sem_rad: float           # standard error of that estimate
    sigma_diff_rad: float    # per-sample noise of the differenced phase
    profile: np.ndarray      # folded mean of the differenced phase
    lag_axis: np.ndarray     # sample offsets for `profile`
    event_bin: int           # index into `profile` of the event sample

    @property
    def snr(self) -> float:
        return abs(self.step_rad) / self.sem_rad if self.sem_rad > 0 else float("inf")

    @property
    def step_um(self) -> float:
        return self.step_rad / K_DISP * 1e6

    @property
    def detected(self) -> bool:
        """Two-sided significance at alpha = 0.01."""
        return self.snr >= Z_ALPHA_2S_001

    def summary(self) -> str:
        return (
            f"epochs {self.n_epochs}  step {self.step_rad:+.5f} rad "
            f"({self.step_um:+.2f} um)  sem {self.sem_rad:.5f} rad  "
            f"snr {self.snr:.2f}  detected={self.detected}"
        )


def min_detectable_step(
    sigma_diff_rad: float,
    n_epochs: int,
    *,
    z_alpha: float = Z_ALPHA_2S_001,
    z_power: float = Z_POWER_099,
) -> float:
    """Smallest Δφ detectable with the stated significance and power.

    ``(z_alpha + z_power) · σ / √N`` — the standard two-sample-free result for a
    mean shift with known variance. Use it to size the record *before* going to
    the bench: at 20 Hz, a 60 s record gives 60 epochs, a 300 s record gives 300.
    """
    if n_epochs <= 0:
        return float("inf")
    return (z_alpha + z_power) * sigma_diff_rad / np.sqrt(n_epochs)


def required_epochs(
    target_step_rad: float,
    sigma_diff_rad: float,
    *,
    z_alpha: float = Z_ALPHA_2S_001,
    z_power: float = Z_POWER_099,
) -> int:
    """Epochs needed to detect ``target_step_rad`` — i.e. record length / 1 s."""
    if target_step_rad <= 0:
        return 2**31 - 1
    return int(np.ceil(((z_alpha + z_power) * sigma_diff_rad / target_step_rad) ** 2))


def _fold(diff: np.ndarray, centres: np.ndarray, half: int) -> FoldResult:
    lags = np.arange(-half, half + 1)
    keep = [c for c in centres if c - half >= 0 and c + half < diff.size]
    if not keep:
        raise ValueError("no epoch fits inside the record")
    seg = np.stack([diff[c - half : c + half + 1] for c in keep])
    profile = seg.mean(axis=0)
    n = len(keep)
    event_bin = int(half)

    # Noise from the off-event lags of the folded segments, so a real step does
    # not inflate the noise estimate it is being compared against.
    off = np.ones(seg.shape[1], dtype=bool)
    off[event_bin] = False
    sigma = float(seg[:, off].std())
    return FoldResult(
        n_epochs=n,
        step_rad=float(profile[event_bin]),
        sem_rad=sigma / np.sqrt(n),
        sigma_diff_rad=sigma,
        profile=profile,
        lag_axis=lags,
        event_bin=event_bin,
    )


def fold_at_events(
    phase_rad: np.ndarray,
    fs_hz: float,
    event_times_s: np.ndarray,
    *,
    t0_s: float = 0.0,
    half_window: int = 5,
) -> FoldResult:
    """Synchronous average of the differenced phase at logged cal events.

    ``event_times_s`` are the reported calibration timestamps, in the same time
    base as the phase record (``t0_s`` is the record's start in that base).
    """
    ph = np.asarray(phase_rad, dtype=float)
    diff = np.diff(ph)
    centres = np.rint((np.asarray(event_times_s, dtype=float) - t0_s) * fs_hz).astype(int)
    # diff[k] spans samples k -> k+1, so a step at sample m lands in diff[m-1].
    centres = centres - 1
    return _fold(diff, centres, half_window)


def fold_at_period(
    phase_rad: np.ndarray,
    fs_hz: float,
    period_s: float,
    *,
    phase_offset_s: float = 0.0,
    half_window: int = 5,
) -> FoldResult:
    """Fallback fold at a hypothesised period, when event times are unavailable.

    Weaker than :func:`fold_at_events`: anything periodic at ``period_s`` folds
    in identically. Never report a positive from this alone.
    """
    ph = np.asarray(phase_rad, dtype=float)
    diff = np.diff(ph)
    dur = ph.size / fs_hz
    starts = np.arange(phase_offset_s, dur, period_s)
    centres = np.rint(starts * fs_hz).astype(int) - 1
    return _fold(diff, centres, half_window)


def page_hinkley(
    x: np.ndarray, *, delta: float, threshold: float
) -> list[int]:
    """Change-point indices via the Page–Hinkley cumulative-deviation test.

    ``delta`` is the magnitude of change deemed insignificant (a slack term) and
    ``threshold`` the detection level; both in the units of ``x``. Used here on
    the differenced phase to locate steps *without* assuming a period, so an
    unmodelled artefact cannot hide behind the 1 Hz hypothesis.
    """
    x = np.asarray(x, dtype=float)
    hits: list[int] = []
    mean = 0.0
    cum = 0.0
    cum_min = 0.0
    for i, v in enumerate(x):
        mean += (v - mean) / (i + 1)
        cum += v - mean - delta
        cum_min = min(cum_min, cum)
        if cum - cum_min > threshold:
            hits.append(i)
            mean, cum, cum_min = 0.0, 0.0, 0.0
    return hits


def band_report(phase_rad: np.ndarray, fs_hz: float) -> dict[str, float]:
    """D1's deliverable: band-limited RMS in the vital bands, in rad and µm.

    Uses the project's existing convention (band-RMS in the respiration and
    cardiac bands) rather than an Allan deviation, so bench numbers are directly
    comparable to the RBEC budget figures.
    """
    from ..rbec.core import CARDIAC_BAND, RESP_BAND, band_rms

    out: dict[str, float] = {}
    for name, band in (("resp", RESP_BAND), ("cardiac", CARDIAC_BAND)):
        r = band_rms(np.asarray(phase_rad, dtype=float), fs_hz, band)
        out[f"{name}_rms_rad"] = float(r)
        out[f"{name}_rms_um"] = float(r / K_DISP * 1e6)
    return out
