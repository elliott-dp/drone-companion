#!/usr/bin/env python3
"""Compressibility of TI cascade raw ADC frames, and the price of lossy modes
expressed as displacement noise in micrometres at 79 GHz.

Synthesises physically-shaped cascade ADC frames (complex int16, fast-time x
chirp x RX), then measures lossless codecs over several reversible transforms,
plus controlled-loss requantisation with its error mapped into the vital-signs
displacement budget.

Run: python3 radar_compress_bench.py
"""
import time
import zlib
import lzma

import numpy as np
import zstandard as zstd

C = 299_792_458.0
F0 = 79e9
LAMBDA = C / F0
RAD_PER_MM = 4 * np.pi / (LAMBDA * 1e3)   # 3.311 rad/mm

NS, NCHIRP, NRX = 256, 64, 16             # fast-time, slow-time, RX (one device group)
RNG = np.random.default_rng(7)


def synth_frame(peak_bits, clutter_db=35.0, n_targets=4, noise_sigma=1.0):
    """One frame of complex baseband, scaled so the peak sits at `peak_bits`.

    Structure that matters for compressibility:
      * complex Gaussian thermal noise (incompressible floor)
      * a very strong zero-Doppler clutter return (ground/leakage) — dominates
        dynamic range and is *identical chirp to chirp*, so slow-time delta
        should erase it
      * a few moving/breathing targets with per-chirp Doppler phase and per-RX
        spatial phase
    """
    s = RNG.normal(0, noise_sigma, (NCHIRP, NRX, NS)) + 1j * RNG.normal(
        0, noise_sigma, (NCHIRP, NRX, NS))

    n = np.arange(NS)
    ch = np.arange(NCHIRP)[:, None, None]
    rx = np.arange(NRX)[None, :, None]

    # static clutter + TX->RX leakage near DC: huge, perfectly repeatable
    a_cl = noise_sigma * 10 ** (clutter_db / 20)
    for fb, ph in ((3.0, 0.0), (0.7, 1.1)):
        s += a_cl * np.exp(1j * (2 * np.pi * fb * n / NS + ph))

    # targets: range beat freq, Doppler phase per chirp, spatial phase per RX
    for k in range(n_targets):
        a = noise_sigma * 10 ** (RNG.uniform(10, 25) / 20)
        fb = RNG.uniform(8, 100)
        fd = RNG.uniform(-0.05, 0.05)        # rad/chirp
        fs = RNG.uniform(-0.9, 0.9)          # rad/rx
        s += a * np.exp(1j * (2 * np.pi * fb * n / NS + fd * ch + fs * rx))

    peak = np.max(np.abs(np.concatenate([s.real.ravel(), s.imag.ravel()])))
    full = 2 ** (peak_bits - 1) - 1
    s *= full / peak
    iq = np.empty((NCHIRP, NRX, NS, 2), dtype=np.int16)
    iq[..., 0] = np.round(s.real)
    iq[..., 1] = np.round(s.imag)
    return iq


def used_bits(a):
    m = int(np.max(np.abs(a.astype(np.int32))))
    return int(np.ceil(np.log2(m + 1))) + 1          # +1 for sign


# ---- reversible transforms ------------------------------------------------
def t_identity(a):
    return a.tobytes()


def t_byteplane(a):
    v = a.astype(np.int16).ravel().view(np.uint16)
    return np.concatenate([(v >> 8).astype(np.uint8),
                           (v & 0xFF).astype(np.uint8)]).tobytes()


def t_delta_slow(a):
    """Difference along the chirp axis: cancels anything static."""
    d = a.astype(np.int32).copy()
    d[1:] -= a.astype(np.int32)[:-1]
    return d.astype(np.int16).tobytes()


def t_delta_slow_byteplane(a):
    d = a.astype(np.int32).copy()
    d[1:] -= a.astype(np.int32)[:-1]
    return t_byteplane(d.astype(np.int16))


def t_delta_fast(a):
    d = a.astype(np.int32).copy()
    d[..., 1:, :] -= a.astype(np.int32)[..., :-1, :]
    return d.astype(np.int16).tobytes()


def t_bitpack(a):
    """Pack to the actual used bit-width (exact; MSBs were sign extension)."""
    nb = used_bits(a)
    v = (a.astype(np.int32).ravel() & ((1 << nb) - 1)).astype(np.uint64)
    bits = np.zeros(v.size * nb, dtype=np.uint8)
    for i in range(nb):
        bits[i::nb] = ((v >> np.uint64(i)) & np.uint64(1)).astype(np.uint8)
    return np.packbits(bits).tobytes()


TRANSFORMS = [
    ("raw int16", t_identity),
    ("byte-plane split", t_byteplane),
    ("delta slow-time", t_delta_slow),
    ("delta slow + byteplane", t_delta_slow_byteplane),
    ("delta fast-time", t_delta_fast),
    ("bit-pack to used bits", t_bitpack),
]


def codecs():
    yield "zstd-1", lambda b: zstd.ZstdCompressor(level=1).compress(b)
    yield "zstd-3", lambda b: zstd.ZstdCompressor(level=3).compress(b)
    yield "zstd-9", lambda b: zstd.ZstdCompressor(level=9).compress(b)
    yield "zstd-19", lambda b: zstd.ZstdCompressor(level=19).compress(b)
    yield "zlib-6", lambda b: zlib.compress(b, 6)
    yield "lzma-6", lambda b: lzma.compress(b, preset=6)


def bench(frame, tag):
    base = frame.nbytes
    print(f"\n=== {tag} — frame {base/2**20:.2f} MiB, "
          f"used bits/sample = {used_bits(frame)} of 16 ===")
    print(f"{'transform':<24}{'codec':<9}{'ratio':>7}{'MB/s in':>9}")
    for tname, tf in TRANSFORMS:
        buf = tf(frame)
        for cname, cf in codecs():
            if cname in ("zstd-19", "lzma-6") and tname not in (
                    "raw int16", "delta slow + byteplane"):
                continue                      # keep the slow ones to 2 rows
            t0 = time.perf_counter()
            out = cf(buf)
            dt = time.perf_counter() - t0
            print(f"{tname:<24}{cname:<9}{base/len(out):>7.2f}"
                  f"{base/dt/1e6:>9.0f}")


def lossy_budget(frame):
    """Drop k LSBs; report ratio, SNR cost, and displacement noise in um."""
    print("\n=== controlled-loss requantisation (drop k LSBs) ===")
    print(f"{'k':>2}{'kept bits':>10}{'ratio(zstd-3)':>15}"
          f"{'SNR loss dB':>13}{'sigma_phi rad':>15}{'sigma_d um':>12}")
    ref = frame.astype(np.float64)
    sig_pow = np.mean(ref ** 2)
    nb = used_bits(frame)
    cz = zstd.ZstdCompressor(level=3)
    for k in range(0, 9):
        q = (frame.astype(np.int32) >> k) << k
        err = ref - q
        err_pow = np.mean(err ** 2)
        # per-sample amplitude SNR after requantisation
        snr = 10 * np.log10(sig_pow / err_pow) if err_pow > 0 else float("inf")
        # phase-noise floor for a single look at this SNR, and the displacement
        # it implies at 79 GHz (sigma_phi = 1/sqrt(2*SNR_linear))
        snr_lin = sig_pow / err_pow if err_pow > 0 else np.inf
        sphi = 1 / np.sqrt(2 * snr_lin) if np.isfinite(snr_lin) else 0.0
        sd_um = sphi / RAD_PER_MM * 1000
        packed = cz.compress((q.astype(np.int32) >> k).astype(np.int16).tobytes())
        print(f"{k:>2}{nb-k:>10}{frame.nbytes/len(packed):>15.2f}"
              f"{snr:>13.1f}{sphi:>15.5f}{sd_um:>12.2f}")


if __name__ == "__main__":
    print(f"lambda = {LAMBDA*1e3:.4f} mm, 4pi/lambda = {RAD_PER_MM:.4f} rad/mm")
    print(f"cardiac target 0.1 mm = {0.1*RAD_PER_MM:.3f} rad; "
          f"0.5 mm = {0.5*RAD_PER_MM:.3f} rad")

    # (a) generous gain: signal fills most of int16
    f16 = synth_frame(peak_bits=16)
    bench(f16, "full-scale capture (peak at int16 limit)")

    # (b) typical: peak ~12 bits, i.e. 4 unused MSBs
    f12 = synth_frame(peak_bits=12)
    bench(f12, "typical capture (peak ~12 bits)")

    # (c) quiet scene / low gain
    f10 = synth_frame(peak_bits=10)
    bench(f10, "quiet scene (peak ~10 bits)")

    lossy_budget(f12)
