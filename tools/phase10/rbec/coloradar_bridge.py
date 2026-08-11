"""ColoRadar cascade bridge: parse + calibrate + beamform real MMWCAS data.

Loads the ColoRadar dataset's cascade (4x AWR2243) raw ADC frames, applies
the dataset's phase/frequency calibration, and synthesizes steered azimuth
beams — the front end for running the RBEC anchor solve against real
cascade recordings with ground-truth poses (track 1 of the pipeline
research; see radar_rbec_validation.md follow-ups).

Format authority (no dataset needed to develop against): the ColoRadar dev
kit as mirrored in azinke/coloradar (core/radar.py, core/calibration.py,
dataset/dataset.json) and arpg/coloradar-library. Layout:

    <root>/calib/cascade/{waveform_cfg,antenna_cfg,coupling_calib}.txt,
                          phase_frequency_calib.txt (JSON)
    <root>/kitti/<sequence>/cascade/adc_samples/data/frame_<n>.bin
                            cascade/adc_samples/timestamps.txt
                            groundtruth/{groundtruth_poses,timestamps}.txt

Raw frame binary: int16, shape (num_tx, num_rx, num_chirps, num_samples, 2)
with I/Q last. Antenna config rows are 'tx|rx idx az el' in half-wavelength
units at f_design, row order [dev4, dev1, dev3, dev2].

Everything here is NumPy-only and validated by ``self_test()``, which writes
a synthetic mini-dataset in this exact format and verifies target recovery
through the full chain (parse -> calibrate -> range FFT -> virtual array ->
beamform -> phase).
"""

from __future__ import annotations

import json
import os

import numpy as np

C0 = 2.998e8


# --------------------------------------------------------------------------
# calibration / config parsing
# --------------------------------------------------------------------------

def _read_kv(path: str) -> dict:
    out = {}
    with open(path) as fh:
        for line in fh:
            if line.startswith("#") or not line.strip():
                continue
            parts = line.strip().replace(":", " ").split()
            key = parts[0].lower()
            # values may be whitespace- or comma-separated (coupling data)
            vals = [v for tok in parts[1:] for v in tok.split(",") if v]
            if key in ("rx", "tx"):
                out.setdefault(key + "l", []).append([int(x) for x in vals])
            elif len(vals) == 1:
                out[key] = float(vals[0])
            else:
                out[key] = [float(x) for x in vals]
    return out


class CascadeCalib:
    def __init__(self, calib_dir: str):
        wf = _read_kv(os.path.join(calib_dir, "cascade", "waveform_cfg.txt"))
        self.num_tx = int(wf["num_tx"])
        self.num_rx = int(wf["num_rx"])
        self.num_chirps = int(wf["num_chirps_per_frame"])
        self.num_samples = int(wf["num_adc_samples_per_chirp"])
        self.fs = float(wf["adc_sample_frequency"])
        self.f0 = float(wf["start_frequency"])
        self.slope = float(wf["frequency_slope"])
        self.chirp_time = (float(wf.get("idle_time", 0.0))
                           + float(wf.get("ramp_end_time", 0.0)))

        ant = _read_kv(os.path.join(calib_dir, "cascade", "antenna_cfg.txt"))
        fd = float(ant["f_design"]) if "f_design" in ant else None
        if fd is not None and fd < 1e3:   # dataset ships GHz (e.g. 76.8)
            fd *= 1e9
        self.f_design = fd
        self.txl = np.array(ant["txl"])   # rows: [idx, az, el] in lambda/2
        self.rxl = np.array(ant["rxl"])

        # coupling calibration: one complex value per (tx, rx, pos-range-bin),
        # subtracted from the range FFT (dataset paper Eq. 3)
        self.coupling = None
        cpath = os.path.join(calib_dir, "cascade", "coupling_calib.txt")
        if os.path.exists(cpath):
            cc = _read_kv(cpath)
            if "data" in cc:
                d = np.asarray(cc["data"], dtype=float)
                d = d[::2] + 1j * d[1::2]
                nrb = int(cc.get("num_range_bins", d.size
                                 // (self.num_tx * self.num_rx)))
                self.coupling = d.reshape(self.num_tx, self.num_rx, 1, nrb)

        with open(os.path.join(calib_dir, "cascade",
                               "phase_frequency_calib.txt")) as fh:
            pc = json.load(fh)["antennaCalib"]
        pm = np.array(pc["phaseCalibrationMatrix"], dtype=float)
        pm = pm[::2] + 1j * pm[1::2]
        self.phase_cal = (pm[0] / pm).reshape(self.num_tx, self.num_rx, 1, 1)
        fcal = np.array(pc["frequencyCalibrationMatrix"], dtype=float)
        dp = fcal - fcal[0]
        ramp = (2 * np.pi * dp
                * (self.slope / pc["frequencySlope"])
                * (pc["samplingRate"] / self.fs)
                / self.num_samples)
        self.freq_cal = np.exp(
            -1j * ramp[:, None] * np.arange(self.num_samples)[None, :]
        ).reshape(self.num_tx, self.num_rx, 1, self.num_samples)

        # carrier at mid-sweep, wavelength, and the virtual azimuth array
        stime = self.num_samples / self.fs
        self.fc = self.f0 + self.slope * stime / 2
        self.lam = C0 / self.fc
        # element spacing is lambda/2 at f_design; express positions in
        # units of the OPERATING half-wavelength via the d-scale factor
        self.d_scale = (self.fc / self.f_design) if self.f_design else 1.0

        tx_az = self.txl[:, 1][:, None]
        rx_az = self.rxl[:, 1][None, :]
        self.virt_az = (tx_az + rx_az).astype(float)          # (tx, rx)
        tx_el = self.txl[:, 2][:, None]
        rx_el = self.rxl[:, 2][None, :]
        self.virt_el = (tx_el + rx_el).astype(float)

    def range_axis(self) -> np.ndarray:
        rres = C0 * self.fs / (2 * self.slope * self.num_samples)
        return np.arange(self.num_samples) * rres


# --------------------------------------------------------------------------
# sequence access
# --------------------------------------------------------------------------

class CascadeSequence:
    def __init__(self, seq_dir: str, calib: CascadeCalib):
        self.calib = calib
        self.dir = os.path.join(seq_dir, "cascade", "adc_samples")
        self.times = np.loadtxt(os.path.join(self.dir, "timestamps.txt"))
        # Prefer Vicon ground truth when present (ASPEN sequences: mm-class
        # mocap) over the lidar-inertial pose graph (accuracy unquantified).
        vicon_dir = os.path.join(seq_dir, "vicon")
        if os.path.isdir(vicon_dir):
            self.gt_times = np.loadtxt(
                os.path.join(vicon_dir, "timestamps.txt"))
            self.gt_poses = np.loadtxt(
                os.path.join(vicon_dir, "vicon_poses.txt"))
            self.gt_source = "vicon"
        else:
            gt_dir = os.path.join(seq_dir, "groundtruth")
            self.gt_times = np.loadtxt(os.path.join(gt_dir, "timestamps.txt"))
            # groundtruth_poses: x y z qx qy qz qw per row
            self.gt_poses = np.loadtxt(
                os.path.join(gt_dir, "groundtruth_poses.txt"))
            self.gt_source = "pose-graph"

    def n_frames(self) -> int:
        return self.times.size

    def frame(self, i: int) -> np.ndarray:
        """Complex cube (tx, rx, chirps, samples), calibrated. ``i`` is
        0-based against ``times``; ColoRadar+ ships 1-indexed filenames
        (frame_1.bin first), detected and mapped automatically."""
        c = self.calib
        if not hasattr(self, "_one_indexed"):
            self._one_indexed = (not os.path.exists(
                os.path.join(self.dir, "data", "frame_0.bin"))
                and os.path.exists(
                    os.path.join(self.dir, "data", "frame_1.bin")))
        path = os.path.join(self.dir, "data",
                            f"frame_{i + 1 if self._one_indexed else i}.bin")
        raw = np.fromfile(path, dtype=np.int16).reshape(
            c.num_tx, c.num_rx, c.num_chirps, c.num_samples, 2)
        cube = raw[..., 0].astype(np.float32) \
            + 1j * raw[..., 1].astype(np.float32)
        return cube * c.phase_cal * c.freq_cal


# --------------------------------------------------------------------------
# the cube chain
# --------------------------------------------------------------------------

def range_fft(cube: np.ndarray, window: bool = True,
              calib: CascadeCalib | None = None) -> np.ndarray:
    """Range FFT; if ``calib`` with coupling data is given, subtracts the
    dataset's antenna-coupling calibration from the positive range bins."""
    n = cube.shape[-1]
    w = np.blackman(n) if window else np.ones(n)
    rf = np.fft.fft(cube * w, axis=-1)
    if calib is not None and calib.coupling is not None:
        nrb = calib.coupling.shape[-1]
        rf[..., :nrb] = rf[..., :nrb] - calib.coupling
    return rf


def steer_beam(rng_fft: np.ndarray, calib: CascadeCalib, az: float,
               rbin: int, chirp_mean: bool = True,
               elevation_row_only: bool = True) -> complex | np.ndarray:
    """Steered azimuth-beam output at one range bin.

    Sums the virtual channels with conjugate steering phases
    exp(-j*pi*d_scale*pos*sin(az)). By default restricts to the
    elevation-0 azimuth row of the virtual array (the 86-position ULA)
    and averages over chirps (each chirp = one TX in TDM, so summing over
    the tx axis after per-chirp extraction is the TDM demux for a static
    or slowly-moving scene; Doppler-phase correction across TX is left to
    the caller for fast scenes).
    """
    c = calib
    sel = (c.virt_el == c.virt_el.min()) if elevation_row_only \
        else np.ones_like(c.virt_el, dtype=bool)
    ph = np.exp(-1j * np.pi * c.d_scale * c.virt_az * np.sin(az))
    w = ph * sel
    w = w / np.abs(w).sum()
    # rng_fft shape (tx, rx, chirps, samples) -> pick bin
    v = rng_fft[..., rbin]                       # (tx, rx, chirps)
    beam = (v * w[:, :, None]).sum(axis=(0, 1))  # per chirp
    return beam.mean() if chirp_mean else beam


def energy_map(seq: CascadeSequence, frames: range, az_grid: np.ndarray,
               max_bin: int) -> np.ndarray:
    """Time-integrated |beam|^2 over (az, range-bin) — the anchor-detection
    map (Stöckel's S(r,theta), survey C.4)."""
    c = seq.calib
    acc = np.zeros((az_grid.size, max_bin))
    for fi in frames:
        rf = range_fft(seq.frame(fi), calib=c if "c" in dir() else seq.calib)
        for ai, az in enumerate(az_grid):
            for rb in range(max_bin):
                acc[ai, rb] += np.abs(steer_beam(rf, c, az, rb)) ** 2
    return acc


# --------------------------------------------------------------------------
# synthetic fixture + self test
# --------------------------------------------------------------------------

def write_fixture(root: str, n_frames: int = 8,
                  target_az_deg: float = 12.0, target_range: float = 6.0,
                  disp_per_frame: float = 0.0002) -> dict:
    """Write a synthetic mini-dataset in the exact ColoRadar layout: one
    point target at (az, range) whose range increases by ``disp_per_frame``
    each frame. Returns truth for the self test."""
    cal_dir = os.path.join(root, "calib", "cascade")
    os.makedirs(cal_dir, exist_ok=True)
    num_tx, num_rx, num_chirps, num_samples = 12, 16, 16, 256
    fs, f0, slope = 8e6, 77e9, 78.9861e12
    with open(os.path.join(cal_dir, "waveform_cfg.txt"), "w") as fh:
        fh.write(f"num_rx {num_rx}\nnum_tx {num_tx}\n"
                 f"num_adc_samples_per_chirp {num_samples}\n"
                 f"num_chirps_per_frame {num_chirps}\n"
                 f"adc_sample_frequency {fs}\nstart_frequency {f0}\n"
                 f"idle_time 5e-6\nadc_start_time 6e-6\n"
                 f"ramp_end_time 40e-6\nfrequency_slope {slope}\n")
    # antenna: TIDUEN5A azimuth map (matches the dataset's own convention;
    # row order [dev4, dev1, dev3, dev2])
    tx_az = [0, 4, 8] + [9, 10, 11] + [12, 16, 20] + [24, 28, 32]
    tx_el = [0, 0, 0] + [1, 4, 6] + [0, 0, 0] + [0, 0, 0]
    rx_az = list(range(0, 4)) + list(range(11, 15)) \
        + list(range(46, 50)) + list(range(50, 54))
    with open(os.path.join(cal_dir, "antenna_cfg.txt"), "w") as fh:
        fh.write("num_rx 16\nnum_tx 12\nF_design 77\n")
        for i, a in enumerate(rx_az):
            fh.write(f"rx {i} {a} 0\n")
        for i, (a, e) in enumerate(zip(tx_az, tx_el)):
            fh.write(f"tx {i} {a} {e}\n")
    ident = [x for _ in range(num_tx * num_rx) for x in (1.0, 0.0)]
    with open(os.path.join(cal_dir, "phase_frequency_calib.txt"), "w") as fh:
        json.dump({"antennaCalib": {
            "numRx": num_rx, "numTx": num_tx,
            "frequencySlope": slope / 1e12, "samplingRate": fs / 1e3,
            "frequencyCalibrationMatrix": [0.0] * (num_tx * num_rx),
            "phaseCalibrationMatrix": ident}}, fh)

    seq_dir = os.path.join(root, "kitti", "fixture_run0")
    data_dir = os.path.join(seq_dir, "cascade", "adc_samples", "data")
    gt_dir = os.path.join(seq_dir, "groundtruth")
    os.makedirs(data_dir, exist_ok=True)
    os.makedirs(gt_dir, exist_ok=True)

    calib = CascadeCalib(os.path.join(root, "calib"))
    az = np.deg2rad(target_az_deg)
    lam = calib.lam
    t_fast = np.arange(num_samples) / fs
    rng = np.random.default_rng(3)
    for fi in range(n_frames):
        r = target_range + fi * disp_per_frame
        fb = 2 * slope * r / C0
        phase_geo = 4 * np.pi * r / lam
        steer = np.exp(1j * np.pi * calib.d_scale * calib.virt_az
                       * np.sin(az))
        sig = np.exp(1j * (2 * np.pi * fb * t_fast + phase_geo))
        cube = (steer[:, :, None, None]
                * sig[None, None, None, :]
                * np.ones((1, 1, num_chirps, 1))) * 400.0
        cube += rng.normal(0, 2.0, cube.shape) \
            + 1j * rng.normal(0, 2.0, cube.shape)
        out = np.empty((num_tx, num_rx, num_chirps, num_samples, 2),
                       dtype=np.int16)
        out[..., 0] = np.round(cube.real)
        out[..., 1] = np.round(cube.imag)
        out.tofile(os.path.join(data_dir, f"frame_{fi}.bin"))
    np.savetxt(os.path.join(seq_dir, "cascade", "adc_samples",
                            "timestamps.txt"), np.arange(n_frames) * 0.1)
    np.savetxt(os.path.join(gt_dir, "timestamps.txt"),
               np.arange(n_frames) * 0.1)
    np.savetxt(os.path.join(gt_dir, "groundtruth_poses.txt"),
               np.column_stack([np.arange(n_frames) * disp_per_frame,
                                np.zeros((n_frames, 5)),
                                np.ones(n_frames)]))
    return {"az": az, "range": target_range,
            "disp_per_frame": disp_per_frame, "calib": calib,
            "seq_dir": seq_dir}


def self_test(tmp_root: str) -> None:
    """Round-trip: fixture -> parse -> chain -> verify angle, range and
    frame-to-frame phase recovery."""
    truth = write_fixture(tmp_root)
    calib = CascadeCalib(os.path.join(tmp_root, "calib"))
    seq = CascadeSequence(truth["seq_dir"], calib)

    rf = range_fft(seq.frame(0))
    rbin = int(np.argmin(np.abs(calib.range_axis() - truth["range"])))
    # angle recovery: scan beams, expect peak at the fixture azimuth
    az_grid = np.deg2rad(np.linspace(-30, 30, 121))
    resp = [np.abs(steer_beam(rf, calib, a, rbin)) for a in az_grid]
    az_hat = az_grid[int(np.argmax(resp))]
    assert abs(np.rad2deg(az_hat - truth["az"])) < 0.75, \
        f"angle recovery failed: {np.rad2deg(az_hat):.2f}"
    # phase tracking across frames: 4*pi/lambda * disp per frame
    ph = []
    for fi in range(seq.n_frames()):
        b = steer_beam(range_fft(seq.frame(fi)), calib, az_hat, rbin)
        ph.append(np.angle(b))
    dph = np.diff(np.unwrap(ph))
    expect = 4 * np.pi * truth["disp_per_frame"] / calib.lam
    err = abs(dph.mean() - expect) / expect
    assert err < 0.05, f"phase slope off by {err*100:.1f}%"
    print(f"coloradar_bridge self_test OK: az {np.rad2deg(az_hat):+.2f} deg "
          f"(truth {np.rad2deg(truth['az']):+.2f}), phase step "
          f"{dph.mean():.4f} rad (truth {expect:.4f}, {err*100:.1f}% err)")


if __name__ == "__main__":
    import tempfile
    with tempfile.TemporaryDirectory() as td:
        self_test(td)
