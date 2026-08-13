"""Experiment 6: is a 3-D anchor solve the right upgrade for exp5?

exp5 solves 2-D because elevation steering is not wired, and the upgrade list
records "3-D LOS" as the fix. This experiment asks whether that is the right
fix *and at what elevation-accuracy cost*, because the cascade's elevation
aperture is a few positions against 86 in azimuth: elevation DoA error is
degrees where azimuth is fractions of a degree. Adding a third unknown whose
sensitivity vector is badly known can inflate the variance of the only
quantity that matters -- the prediction along the casualty LOS.

Three estimators, scored on the same in-band metric as exp3:

  oracle3d : 3 unknowns, exact anchor LOS               (floor)
  radar3d  : 3 unknowns, anchor LOS perturbed in az AND el ("wire it")
  drop_z   : 2 unknowns (dx, dy); true dz unmodelled     (exp5 today)

The mechanism is closed-form. With A2 = K*U[:, :2], the 2-unknown solve
returns d_xy + pinv(U2) U_z dz, so the LOS prediction error from unmodelled
vertical motion is exactly

    alpha * K * dz(t),   alpha = u_t,xy . pinv(U2) U_z  -  u_t,z          (1)

a single scene-computable scalar (verified to machine precision in
``_self_test``). alpha is the *z-aliasing gain*: it decides whether dropping z
is free or fatal, and it is the quantity that should gate the upgrade.

Usage:
    python3 -m tools.phase10.rbec.exp6_zaxis
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np

from .core import (CARDIAC_BAND, CARDIAC_RESIDUAL_BUDGET_RAD, K_DISP,
                   RESP_BAND, band_rms, los_from_azel, shaped_noise)
from .geometry import target_dop

FS = 20.0
DWELL_S = 30.0


# --------------------------------------------------------------- scenes ---

@dataclass
class Scene:
    name: str
    U: np.ndarray          # (N,3) true anchor LOS
    u_t: np.ndarray        # (3,) casualty LOS
    label: str


def scene_hover_ground(h: float = 10.0) -> Scene:
    """UAV hover at h; ground-ring anchors at 6/12/25 m ground range;
    casualty on the ground at el = -30 deg (exp2's configuration)."""
    us = []
    for rg in (6.0, 12.0, 25.0):
        el = -np.arctan2(h, rg)
        for az in np.deg2rad(np.linspace(-60, 60, 4)):
            us.append(los_from_azel(az, el))
    return Scene("hover_ground", np.array(us),
                 los_from_azel(0.0, np.deg2rad(-30.0)),
                 "hover over ground, h=10 m")


def scene_hallway(seed: int = 7) -> Scene:
    """ColoRadar ec_hallways-like: a cart-borne sensor with wall/corner
    anchors at near-horizontal elevation -- elevation spread is small by
    construction, which is the geometry exp5 actually ran on."""
    rng = np.random.default_rng(seed)
    az = np.deg2rad(np.linspace(-60, 60, 12))
    el = np.deg2rad(rng.uniform(-5.0, 5.0, 12))
    U = np.array([los_from_azel(a, e) for a, e in zip(az, el)])
    return Scene("hallway", U, los_from_azel(0.0, 0.0),
                 "hallway, near-horizontal anchors")


def scene_hover_elevated(h: float = 10.0) -> Scene:
    """Hover with a mix of ground anchors and elevated structure anchors --
    the exp2 'rings + structures' case, best-conditioned in elevation."""
    us = []
    for rg in (6.0, 12.0, 25.0):
        el = -np.arctan2(h, rg)
        for az in np.deg2rad(np.linspace(-60, 60, 3)):
            us.append(los_from_azel(az, el))
    for az, el in ((-40.0, -5.0), (0.0, -8.0), (45.0, -3.0)):
        us.append(los_from_azel(np.deg2rad(az), np.deg2rad(el)))
    return Scene("hover_elevated", np.array(us),
                 los_from_azel(0.0, np.deg2rad(-30.0)),
                 "hover, ground rings + structures")


SCENES = {s.name: s for s in (scene_hover_ground(), scene_hallway(),
                              scene_hover_elevated())}


def z_alias_gain(sc: Scene) -> float:
    """alpha of Eq. (1): LOS prediction error per metre of unmodelled dz."""
    U2 = sc.U[:, :2]
    g = np.linalg.pinv(U2) @ sc.U[:, 2]
    return float(sc.u_t[:2] @ g - sc.u_t[2])


# ----------------------------------------------------------------- solve ---

def run_dwell(sc: Scene, sigma_el_deg: float, seed: int,
              sigma_az_deg: float = 0.1, sway_rms_m: float = 0.02,
              sigma_phi: float = 0.0115) -> dict:
    """One 30 s dwell. Returns estimator -> LOS prediction error (rad).

    Angle errors are fixed per dwell (systematic), matching exp3's error
    model and the review's finding that this is the regime that matters.
    sigma_phi is the per-frame anchor phase noise (25 dB/chirp, 12 chirps).
    """
    rng = np.random.default_rng(seed + 10_000)
    n = int(DWELL_S * FS)
    N = sc.U.shape[0]

    az = np.arctan2(sc.U[:, 1], sc.U[:, 0])
    el = np.arcsin(np.clip(sc.U[:, 2], -1, 1))
    U_est = np.array([
        los_from_azel(a + np.deg2rad(sigma_az_deg) * rng.standard_normal(),
                      e + np.deg2rad(sigma_el_deg) * rng.standard_normal())
        for a, e in zip(az, el)])

    d = np.stack([shaped_noise(n, FS, 0.3, sway_rms_m, rng) for _ in range(3)],
                 axis=1)                                  # (n,3) metres
    phi = K_DISP * (d @ sc.U.T).T + sigma_phi * rng.standard_normal((N, n))
    los_true = d @ sc.u_t

    out = {}
    for tag, U in (("oracle3d", sc.U), ("radar3d", U_est)):
        d_hat = np.linalg.pinv(K_DISP * U) @ phi          # (3,n)
        out[tag] = K_DISP * (sc.u_t @ d_hat - los_true)
    d2 = np.linalg.pinv(K_DISP * U_est[:, :2]) @ phi
    d_dropz = np.vstack([d2, np.zeros(n)])
    out["drop_z"] = K_DISP * (sc.u_t @ d_dropz - los_true)
    return out


def score(sc: Scene, sigma_el_deg: float, seeds=range(8), **kw) -> dict:
    acc: dict[str, list] = {}
    for s in seeds:
        for tag, e in run_dwell(sc, sigma_el_deg, s, **kw).items():
            acc.setdefault(tag, []).append(
                (band_rms(e, FS, CARDIAC_BAND), band_rms(e, FS, RESP_BAND)))
    return {t: np.array(v) for t, v in acc.items()}


def crossover_sigma_el(sc: Scene, lo=0.05, hi=45.0, **kw) -> float:
    """sigma_el at which radar3d's cardiac residual equals drop_z's.
    Below it, solving 3-D wins; above it, dropping z wins."""
    f = lambda s: (score(sc, s, seeds=range(4), **kw)["radar3d"][:, 0].mean()
                   - score(sc, s, seeds=range(4), **kw)["drop_z"][:, 0].mean())
    if f(lo) > 0:
        return float("nan")            # 3-D never wins
    if f(hi) < 0:
        return float("inf")            # 3-D always wins in range
    for _ in range(24):
        mid = np.sqrt(lo * hi)
        if f(mid) < 0:
            lo = mid
        else:
            hi = mid
    return float(np.sqrt(lo * hi))


def budget_sigma_el(sc: Scene, lo=0.01, hi=45.0, **kw) -> float:
    """Largest sigma_el at which radar3d still meets the cardiac budget."""
    g = lambda s: (score(sc, s, seeds=range(4), **kw)["radar3d"][:, 0].mean()
                   - CARDIAC_RESIDUAL_BUDGET_RAD)
    if g(lo) > 0:
        return float("nan")
    if g(hi) < 0:
        return float("inf")
    for _ in range(24):
        mid = np.sqrt(lo * hi)
        if g(mid) < 0:
            lo = mid
        else:
            hi = mid
    return float(np.sqrt(lo * hi))


# --------------------------------------------- what the hardware can do ---

# TIDUEN5A virtual-array elevation structure (as transcribed in
# coloradar_bridge.write_fixture, which the ColoRadar dev kit independently
# confirms): only the master's three TX sit off the elevation axis, at
# half-wavelength heights 1, 4, 6; every RX is at elevation 0. So the
# elevation aperture is 6 half-wavelengths = 3 lambda, against 86 half-
# wavelengths = 43 lambda in azimuth -- a 14x shorter baseline.
TX_EL_HALFLAMBDA = np.array([0, 0, 0, 1, 4, 6, 0, 0, 0, 0, 0, 0], dtype=float)
RX_EL_HALFLAMBDA = np.zeros(16)
TX_AZ_HALFLAMBDA = np.array([0, 4, 8, 9, 10, 11, 12, 16, 20, 24, 28, 32],
                            dtype=float)
RX_AZ_HALFLAMBDA = np.array(list(range(0, 4)) + list(range(11, 15))
                            + list(range(46, 50)) + list(range(50, 54)),
                            dtype=float)


def virtual_positions() -> tuple[np.ndarray, np.ndarray]:
    """(az, el) virtual-element positions in half-wavelengths."""
    az = (TX_AZ_HALFLAMBDA[:, None] + RX_AZ_HALFLAMBDA[None, :]).ravel()
    el = (TX_EL_HALFLAMBDA[:, None] + RX_EL_HALFLAMBDA[None, :]).ravel()
    return az, el


def doa_crb_deg(snr_db: float, n_snapshots: int = 12,
                axis: str = "el", theta_deg: float = 0.0) -> float:
    """Deterministic (conditional) CRB on DoA for the full 2-D virtual array.

    For a single source with unknown complex amplitude, the CRB on the
    electrical angle parameter is the standard array-processing result

        var(theta) = 1 / (2 * N_snap * SNR_lin * var_spatial(d_eff))

    where d_eff = pi * pos * cos(theta) is the derivative of the steering
    phase w.r.t. theta and var_spatial is taken about the array centroid
    (the centroid subtraction is the unknown-phase nuisance term). Positions
    are in half-wavelengths, so the steering phase is pi*pos*sin(theta).
    """
    az_pos, el_pos = virtual_positions()
    pos = el_pos if axis == "el" else az_pos
    th = np.deg2rad(theta_deg)
    dphi = np.pi * pos * np.cos(th)             # d(phase)/d(theta)
    spread = np.var(dphi)                       # about the centroid
    snr = 10 ** (snr_db / 10)
    var = 1.0 / (2 * n_snapshots * snr * spread * pos.size)
    return float(np.rad2deg(np.sqrt(var)))


def doa_cal_error_deg(cal_sigma_deg: float = 2.0, axis: str = "el",
                      theta_deg: float = 0.0, snr_db: float = 25.0,
                      n_snapshots: int = 12, n_draws: int = 400,
                      seed: int = 11) -> tuple[float, float]:
    """DoA error from per-channel CALIBRATION phase residual, by Monte Carlo.

    The CRB above is thermal-noise-only and wildly optimistic for elevation:
    with four distinct elevation positions the binding term is TI's +-2-3 deg
    post-calibration per-channel phase residual (SPRACV2), which is *fixed
    per dwell* -- exactly the systematic regime exp3 found matters. Each draw
    perturbs the 192 virtual-channel phases, forms the conventional
    beamformer response of a single unit-amplitude source, and takes the
    parabolically-interpolated peak. Returns (bias_deg, rms_deg).
    """
    az_pos, el_pos = virtual_positions()
    pos = el_pos if axis == "el" else az_pos
    rng = np.random.default_rng(seed)
    th0 = np.deg2rad(theta_deg)
    bw = beamwidth_deg(axis)
    grid = np.deg2rad(np.linspace(theta_deg - 1.5 * bw, theta_deg + 1.5 * bw,
                                  241))
    Amat = np.exp(1j * np.pi * pos[None, :] * np.sin(grid)[:, None])
    sig_n = 1.0 / np.sqrt(2 * 10 ** (snr_db / 10) * n_snapshots)
    errs = []
    for _ in range(n_draws):
        cal = np.deg2rad(cal_sigma_deg) * rng.standard_normal(pos.size)
        x = np.exp(1j * (np.pi * pos * np.sin(th0) + cal))
        x = x + sig_n * (rng.standard_normal(pos.size)
                         + 1j * rng.standard_normal(pos.size))
        resp = np.abs(Amat.conj() @ x)
        i = int(np.argmax(resp))
        if 0 < i < resp.size - 1:      # parabolic sub-grid refinement
            y0, y1, y2 = resp[i - 1], resp[i], resp[i + 1]
            denom = y0 - 2 * y1 + y2
            off = 0.5 * (y0 - y2) / denom if denom != 0 else 0.0
        else:
            off = 0.0
        dg = grid[1] - grid[0]
        errs.append(np.rad2deg(grid[i] + off * dg - th0))
    e = np.asarray(errs)
    return float(e.mean()), float(np.sqrt(np.mean(e ** 2)))


def doa_two_ray_bias_deg(sep_deg: float, rel_amp: float = 0.5,
                         axis: str = "el", theta_deg: float = -20.0,
                         cal_sigma_deg: float = 2.0, snr_db: float = 25.0,
                         n_snapshots: int = 12, n_draws: int = 200,
                         seed: int = 23) -> tuple[float, float]:
    """DoA error when a SECOND unresolved scatterer sits ``sep_deg`` away.

    This is the elevation aperture's real exposure: the 16.9 deg elevation
    beamwidth cannot resolve a ground-bounce/multipath companion, and two
    unresolved sources pull the beamformer peak to an amplitude- and
    phase-weighted position between them. The relative phase is random per
    draw (it depends on the path-length difference at 3.8 mm), so the pull is
    a per-dwell systematic error of unpredictable sign -- precisely the error
    class exp3 showed WLS cannot average down. Returns (bias, rms) in deg.
    """
    az_pos, el_pos = virtual_positions()
    pos = el_pos if axis == "el" else az_pos
    rng = np.random.default_rng(seed)
    th0 = np.deg2rad(theta_deg)
    th1 = np.deg2rad(theta_deg + sep_deg)
    bw = beamwidth_deg(axis)
    grid = np.deg2rad(np.linspace(theta_deg - 1.5 * bw, theta_deg + 1.5 * bw,
                                  481))
    Amat = np.exp(1j * np.pi * pos[None, :] * np.sin(grid)[:, None])
    sig_n = 1.0 / np.sqrt(2 * 10 ** (snr_db / 10) * n_snapshots)
    errs = []
    for _ in range(n_draws):
        cal = np.deg2rad(cal_sigma_deg) * rng.standard_normal(pos.size)
        psi = rng.uniform(0, 2 * np.pi)          # random path-length phase
        x = (np.exp(1j * (np.pi * pos * np.sin(th0)))
             + rel_amp * np.exp(1j * (np.pi * pos * np.sin(th1) + psi)))
        x = x * np.exp(1j * cal)
        x = x + sig_n * (rng.standard_normal(pos.size)
                         + 1j * rng.standard_normal(pos.size))
        resp = np.abs(Amat.conj() @ x)
        i = int(np.argmax(resp))
        if 0 < i < resp.size - 1:
            y0, y1, y2 = resp[i - 1], resp[i], resp[i + 1]
            den = y0 - 2 * y1 + y2
            off = 0.5 * (y0 - y2) / den if den != 0 else 0.0
        else:
            off = 0.0
        dg = grid[1] - grid[0]
        errs.append(np.rad2deg(grid[i] + off * dg - th0))
    e = np.asarray(errs)
    return float(e.mean()), float(np.sqrt(np.mean(e ** 2)))


def beamwidth_deg(axis: str = "el") -> float:
    """Rayleigh beamwidth 0.886*lambda/D for the aperture on that axis."""
    az_pos, el_pos = virtual_positions()
    pos = el_pos if axis == "el" else az_pos
    d_lambda = (pos.max() - pos.min()) / 2.0    # half-wavelengths -> lambda
    if d_lambda <= 0:
        return float("inf")
    return float(np.rad2deg(0.886 / d_lambda))


# ------------------------------------------------------------ self tests ---

def _self_test() -> str:
    # Eq. (1) exactly predicts the drop_z error series.
    for sc in SCENES.values():
        alpha = z_alias_gain(sc)
        rng = np.random.default_rng(3)
        n = int(DWELL_S * FS)
        d = np.zeros((n, 3))
        d[:, 2] = shaped_noise(n, FS, 0.3, 0.02, rng)     # z-only motion
        U2 = sc.U[:, :2]
        phi = K_DISP * (d @ sc.U.T).T
        d2 = np.linalg.pinv(K_DISP * U2) @ phi
        obs = K_DISP * (sc.u_t[:2] @ d2 - d @ sc.u_t)
        pred = alpha * K_DISP * d[:, 2]
        assert np.max(np.abs(pred - obs)) < 1e-9, (sc.name,
                                                   np.max(np.abs(pred - obs)))
    # zero-error oracle is exact
    r = run_dwell(SCENES["hover_ground"], 0.0, 1, sigma_az_deg=0.0,
                  sigma_phi=0.0)
    assert np.max(np.abs(r["oracle3d"])) < 1e-9
    # a purely horizontal-anchor scene with an on-boresight target has
    # alpha ~ 0: dropping z is free there, by construction
    assert abs(z_alias_gain(SCENES["hallway"])) < 0.05
    # hover-over-ground aliases strongly
    assert abs(z_alias_gain(SCENES["hover_ground"])) > 0.2
    return "exp6 self-tests pass"


def hardware_elevation_error(cal_sigma_deg: float = 2.0) -> dict:
    """What elevation accuracy the cascade can actually deliver, by regime."""
    return {
        "beamwidth_el_deg": beamwidth_deg("el"),
        "beamwidth_az_deg": beamwidth_deg("az"),
        "crb_el_deg": doa_crb_deg(25.0, axis="el"),
        "cal_el_rms_deg": doa_cal_error_deg(cal_sigma_deg, axis="el")[1],
        "cal_az_rms_deg": doa_cal_error_deg(cal_sigma_deg, axis="az")[1],
        "tworay_el_rms_deg": doa_two_ray_bias_deg(5.0, 0.5, axis="el")[1],
        "tworay_az_rms_deg": doa_two_ray_bias_deg(5.0, 0.5, axis="az")[1],
    }


def main() -> None:
    print(_self_test(), "\n")
    print(f"cardiac budget {CARDIAC_RESIDUAL_BUDGET_RAD:.4f} rad\n")

    hw = hardware_elevation_error()
    print("--- what the cascade's elevation aperture can deliver ---")
    print(f"  beamwidth            az {hw['beamwidth_az_deg']:6.2f} deg   "
          f"el {hw['beamwidth_el_deg']:6.2f} deg  (14x shorter baseline)")
    print(f"  CRB, thermal only    el {hw['crb_el_deg']:6.3f} deg")
    print(f"  + 2 deg cal residual az {hw['cal_az_rms_deg']:6.3f} deg   "
          f"el {hw['cal_el_rms_deg']:6.3f} deg")
    print(f"  + unresolved 2nd ray az {hw['tworay_az_rms_deg']:6.3f} deg   "
          f"el {hw['tworay_el_rms_deg']:6.3f} deg  <-- binding term")

    print("\n--- per-scene decision ---")
    print(f"{'scene':16s} {'N':>3s} {'el span':>15s} {'DOP(u_t)':>9s} "
          f"{'alpha':>7s} {'sig_el*':>8s} {'sig_el_bud':>11s} {'verdict':>10s}")
    for sc in SCENES.values():
        el = np.rad2deg(np.arcsin(np.clip(sc.U[:, 2], -1, 1)))
        xo, bud = crossover_sigma_el(sc), budget_sigma_el(sc)
        achievable = hw["tworay_el_rms_deg"]
        if bud == bud and achievable > bud:      # bud not NaN
            verdict = "DROP Z"
        elif achievable < min(xo, bud):
            verdict = "solve 3-D"
        else:
            verdict = "DROP Z"
        print(f"{sc.name:16s} {sc.U.shape[0]:3d} "
              f"{el.min():+6.1f}..{el.max():+6.1f} "
              f"{target_dop(sc.U, sc.u_t):9.3f} {z_alias_gain(sc):+7.3f} "
              f"{xo:8.2f} {bud:11.2f} {verdict:>10s}")
    print("\n  alpha      = z-aliasing gain, Eq. (1): LOS error per unit "
          "unmodelled dz")
    print("  sig_el*    = elevation error where 3-D stops beating drop-z")
    print("  sig_el_bud = elevation error where 3-D leaves cardiac budget")
    print("  verdict compares those against the ACHIEVABLE elevation error "
          f"({hw['tworay_el_rms_deg']:.2f} deg)")

    print("\ncardiac in-band residual [rad], mean of 8 seeds")
    sweep = [0.0, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0]
    print(f"{'scene':16s} {'sig_el':>7s} {'oracle3d':>9s} {'radar3d':>9s} "
          f"{'drop_z':>9s}")
    for sc in SCENES.values():
        for s in sweep:
            r = score(sc, s)
            print(f"{sc.name:16s} {s:7.2f} "
                  f"{r['oracle3d'][:,0].mean():9.4f} "
                  f"{r['radar3d'][:,0].mean():9.4f} "
                  f"{r['drop_z'][:,0].mean():9.4f}")


if __name__ == "__main__":
    main()
