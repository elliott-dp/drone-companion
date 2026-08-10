"""Benchmark the estimator bank on the Erlangen clinical radar dataset
(Schellenberger et al., Sci Data 2020; 24 GHz Six-Port CW, 30 subjects,
Task Force Monitor reference; figshare DOI 10.6084/m9.figshare.12186516,
CC BY 4.0).

Usage:
    python3 -m tools.phase10.vitals.exp_erlangen <zip-or-dir> \
        [--scenario Resting] [--subjects GDN0001,GDN0002] [--probe]

--probe prints the variable names/shapes of the first .mat found (the
dataset's field-name case is unverified; probe before trusting the loader).

Chain per recording: radar I/Q (2000 Hz) -> ellipse correction ->
arctangent phase -> decimate to 20 Hz -> sliding 30 s windows through the
bank -> HR vs ECG-derived reference (R-peak detector on tfm_ecg), RR
reported without reference unless a respiration channel is present.
"""

from __future__ import annotations

import argparse
import io
import os
import zipfile

import numpy as np

from .bank import sliding
from .cw import cw_phase
from .dsp import bandpass

FS_RADAR = 2000.0
FS_BANK = 20.0


def load_mat(buf: bytes) -> dict:
    """Load a .mat from bytes, handling both pre-7.3 (scipy) and 7.3
    (h5py) formats; returns {name: np.ndarray}."""
    try:
        import scipy.io as sio
        d = sio.loadmat(io.BytesIO(buf), squeeze_me=True,
                        struct_as_record=False)
        return {k: v for k, v in d.items() if not k.startswith("__")}
    except NotImplementedError:
        import h5py
        out = {}
        with h5py.File(io.BytesIO(buf), "r") as f:
            def visit(name, obj):
                if isinstance(obj, h5py.Dataset):
                    out[name] = np.array(obj)
            f.visititems(visit)
        return out


def iter_mats(root: str, scenario: str, subjects: list[str] | None):
    """Yield (label, bytes) for matching .mat files in a zip or dir."""
    if os.path.isdir(root):
        for dirp, _, files in os.walk(root):
            for fn in sorted(files):
                if fn.endswith(".mat") and scenario.lower() in fn.lower():
                    sub = os.path.basename(dirp)
                    if subjects and not any(s in dirp + fn for s in subjects):
                        continue
                    yield f"{sub}/{fn}", open(os.path.join(dirp, fn),
                                              "rb").read()
    else:
        with zipfile.ZipFile(root) as z:
            for n in sorted(z.namelist()):
                if n.endswith(".mat") and scenario.lower() in n.lower():
                    if subjects and not any(s in n for s in subjects):
                        continue
                    yield n, z.read(n)


def pick(d: dict, *cands: str) -> np.ndarray | None:
    for k in d:
        base = k.split("/")[-1].lower()
        if base in [c.lower() for c in cands]:
            return np.ravel(np.asarray(d[k], dtype=float))
    return None


def ecg_hr_reference(ecg: np.ndarray, fs: float,
                     t_grid: np.ndarray) -> np.ndarray:
    """Windowless HR reference: R-peak detection (band-passed,
    derivative-squared, adaptive threshold), instantaneous HR from RR
    intervals, interpolated onto ``t_grid`` [s] -> BPM."""
    y = bandpass(ecg, fs, (5.0, 25.0))
    e = y ** 2
    thr = 6.0 * np.median(e)
    min_dist = int(0.3 * fs)
    peaks = []
    last = -min_dist
    for i in range(1, e.size - 1):
        if e[i] > thr and e[i] >= e[i - 1] and e[i] >= e[i + 1]:
            if i - last >= min_dist:
                peaks.append(i)
                last = i
            elif e[i] > e[peaks[-1]]:
                peaks[-1] = i
                last = i
    if len(peaks) < 5:
        return np.full_like(t_grid, np.nan)
    # amplitude post-filter: the first pass catches T-waves as well as
    # R-peaks (alternating strong/weak heights — found on GDN0001, where
    # the raw detector reported 142 "bpm" at rest); keep only peaks near
    # the strong-peak level, then re-enforce the refractory period
    peaks = np.array(peaks)
    h = e[peaks]
    keep = h >= 0.35 * np.percentile(h, 80)
    peaks = peaks[keep]
    filtered = [int(peaks[0])]
    for p in peaks[1:]:
        if p - filtered[-1] >= int(0.35 * fs):
            filtered.append(int(p))
        elif e[p] > e[filtered[-1]]:
            filtered[-1] = int(p)
    peaks = filtered
    if len(peaks) < 5:
        return np.full_like(t_grid, np.nan)
    tp = np.array(peaks) / fs
    rr = np.diff(tp)
    ok = (rr > 0.3) & (rr < 2.0)
    hr_t = tp[1:][ok]
    hr = 60.0 / rr[ok]
    return np.interp(t_grid, hr_t, hr)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("root")
    ap.add_argument("--scenario", default="Resting")
    ap.add_argument("--subjects", default=None)
    ap.add_argument("--probe", action="store_true")
    ap.add_argument("--max-recordings", type=int, default=20)
    args = ap.parse_args()
    subjects = args.subjects.split(",") if args.subjects else None

    hr_errs, confs, rows = [], [], []
    for label, buf in iter_mats(args.root, args.scenario, subjects):
        d = load_mat(buf)
        if args.probe:
            for k, v in d.items():
                shape = getattr(v, "shape", "?")
                print(f"  {k}: {shape}")
            return
        i_ch = pick(d, "radar_i", "radar_I")
        q_ch = pick(d, "radar_q", "radar_Q")
        ecg = pick(d, "tfm_ecg1", "tfm_ecg2", "ecg1", "ecg2")
        if i_ch is None or q_ch is None or ecg is None:
            print(f"{label}: missing channels "
                  f"(have {sorted(d.keys())[:8]}...) — skipped")
            continue
        fs_ecg = FS_RADAR * ecg.size / i_ch.size   # aligned grids
        phase = cw_phase(i_ch, q_ch)
        dec = int(FS_RADAR / FS_BANK)
        ph = bandpass(phase, FS_RADAR, (0.05, 8.0))[::dec]

        # reference computed ONCE per recording, then sampled per window
        t_dense = np.arange(0.0, i_ch.size / FS_RADAR, 1.0)
        hr_series = ecg_hr_reference(ecg, fs_ecg, t_dense)

        errs = []
        for t_c, resp, card in sliding(ph, FS_BANK, 30.0, 10.0):
            hr_ref = float(np.interp(t_c, t_dense, hr_series))
            if np.isnan(hr_ref) or card.rate_hz <= 0:
                continue
            errs.append((abs(card.rate_per_min - hr_ref), hr_ref,
                         card.rate_per_min, resp.rate_per_min,
                         card.confidence))
        if not errs:
            print(f"{label}: no valid windows")
            continue
        e = np.array([x[0] for x in errs])
        rows.append((label, e))
        hr_errs.append(e)
        confs.append(np.array([x[4] for x in errs]))
        print(f"{label}: {e.size} windows, HR MAE {e.mean():.2f} BPM, "
              f"p90 {np.percentile(e, 90):.2f}, median ref "
              f"{np.median([x[1] for x in errs]):.0f} BPM, mean conf "
              f"{np.mean([x[4] for x in errs]):.2f}")
        if len(rows) >= args.max_recordings:
            break

    if hr_errs:
        all_e = np.concatenate(hr_errs)
        all_c = np.concatenate(confs)
        print(f"\nTOTAL: {all_e.size} windows over {len(rows)} recordings — "
              f"HR MAE {all_e.mean():.2f} BPM, median {np.median(all_e):.2f}, "
              f"p90 {np.percentile(all_e, 90):.2f}")
        # confidence-gated view: the payload's three-state doctrine means a
        # low-confidence window is reported "undecided", not as a number —
        # so the operational metric is (coverage, error-when-confident)
        for gate in (0.4, 0.5, 0.6):
            sel = all_c >= gate
            if sel.any():
                print(f"  conf>={gate}: coverage {sel.mean()*100:4.0f} %, "
                      f"MAE {all_e[sel].mean():5.2f}, "
                      f"p90 {np.percentile(all_e[sel], 90):5.2f}")


if __name__ == "__main__":
    main()
