"""exp8 (thesis P2): the z-aliasing gain alpha carries its own error bar.

Closes the one open caveat on contribution C4 (thesis_plan §4 P2;
radar_rbec_validation_exp67.md D.1): alpha is scene-computable, but its
inputs — anchor DoA (az from the 2.5-deg grid, el unmeasured on wall
scenes) and the CASUALTY's elevation, measured by the same weak elevation
aperture exp6 showed reaches only 1.75 deg — are uncertain, so the gate
test "|alpha| < 0.02 => keep the 2-D solve" needs alpha's uncertainty
propagated before it can certify anything.

Method: central-difference Jacobian of alpha w.r.t. every anchor
elevation, every anchor azimuth, and the target's (az, el), validated
against seeded Monte-Carlo in the small-bound regime; full MC (uniform
bounds for elevations, the grid-quantization sigma for azimuths) for the
p95 |alpha| that the per-dwell gate actually consumes. Scenes: exp6's
three synthetic geometries (true elevations known) and the five real
ASPEN still-window anchor sets from the committed exp5b guard-8 bundles
(elevations unknown -> bound-only, exactly as exp5b reports them).

Questions answered, each a figure panel / bundle key:
  1. **Certification map**: per scene, the largest anchor-elevation
     bound (on the EL_BOUNDS_DEG grid, requiring every smaller grid
     bound to pass too) that keeps p95 |alpha| under the 0.02 gate, for
     target elevation known exactly / to 0.5 deg / to the
     elevation-aperture's 1.75 deg. p95 is the gate statistic
     (exp5b-consistent); certified cells still carry 2-3.6 % of draws
     above the gate at the marginal cells (p99 up to ~0.024) — quoted,
     never hidden.
  2. **The irreducible term**: d alpha / d el_target =
     -cos(el_t) - sin(el_t) (u_hat . g) — measured -1.000 at el_t = 0,
     -1.17/-1.34 on the hover scenes (the second term is 35-55 % of the
     first there, geometry-signed). Target-elevation uncertainty maps
     >= 1:1 into alpha and does NOT average down with anchor count: the
     elevation aperture alone (1.75 deg = 0.031 rad -> p95 |alpha| ~
     1.96 sigma ~ 0.060) busts the 0.02 gate with no help from the
     anchors.
  3. **N-scaling**: the anchor-elevation part of alpha's spread shrinks
     ~1/sqrt(N) (it is a weighted mean of independent sin(el_k) draws);
     measured over N = 6..36.
  4. **Azimuth is negligible**: the grid-quantization sigma (0.72 deg)
     contributes < a few percent of the variance everywhere (measured,
     not assumed).

Everything [meas]/[calc]; deterministic (seeded); the committed bundle
``docs/phase10/results/exp8.json`` is regenerated exactly by ``--check``
(exp67_report comparator, rtol 1e-9).

Usage:
    python3 -m tools.phase10.rbec.exp8_alpha_budget            # write bundle
    python3 -m tools.phase10.rbec.exp8_alpha_budget --check
    python3 -m tools.phase10.rbec.exp8_alpha_budget --self-test
"""

from __future__ import annotations

import json
import os
import sys

import numpy as np

from .core import los_from_azel
from .exp5b_upgrade import ALPHA_GATE, SIGMA_THETA_DEG, alpha_gain, _compare

RESULTS_DIR = os.path.normpath(os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "..", "..", "..", "docs", "phase10", "results"))

# real ASPEN still-window anchor azimuths (deg), from the committed
# exp5b_*_gt-rot_*_g8.json bundles (guard 8, upgraded picker); elevations
# are unmeasured on these scenes and the target is the el=0 scoring
# direction the exp5b per-dwell report uses
REAL_WINDOWS = {
    "aspen_run0_1-42": [-5.0, -37.5, -17.5, 2.5, -45.0, -52.5, 30.0,
                        17.5, 22.5],
    "aspen_run1_408-458": [-5.0, -40.0, -50.0, 12.5, 20.0, -45.0, 27.5,
                           -25.0, 32.5],
    "aspen_run2_411-476": [-7.5, -2.5, -42.5, -52.5, 17.5, -47.5, 25.0,
                           10.0, 32.5],
    "aspen_run3_533-584": [-5.0, 0.0, -42.5, -55.0, 12.5, 17.5, 27.5,
                           -25.0, 22.5],
    "aspen_run10_631-694": [-10.0, -42.5, -55.0, -50.0, 27.5, -32.5,
                            -37.5, 37.5, 32.5],
}

EL_BOUNDS_DEG = [0.25, 0.5, 1.0, 2.0, 3.0, 5.0]
TARGET_EL_SIGMA_DEG = [0.0, 0.5, 1.75]     # known / aided / el-aperture
N_SCALING = [6, 9, 12, 18, 24, 36]
N_MC = 4096


def _alpha(az: np.ndarray, el: np.ndarray, t_az: float,
           t_el: float) -> float:
    U = np.array([los_from_azel(a, e) for a, e in zip(az, el)])
    return alpha_gain(U, los_from_azel(t_az, t_el))


def alpha_jacobian(az: np.ndarray, el: np.ndarray, t_az: float,
                   t_el: float, h: float = 1e-6) -> dict:
    """Central-difference sensitivities of alpha [1/rad]."""
    az = np.asarray(az, dtype=float)
    el = np.asarray(el, dtype=float)
    j_el = np.empty(az.size)
    j_az = np.empty(az.size)
    for k in range(az.size):
        e1, e2 = el.copy(), el.copy()
        e1[k] -= h
        e2[k] += h
        j_el[k] = (_alpha(az, e2, t_az, t_el)
                   - _alpha(az, e1, t_az, t_el)) / (2 * h)
        a1, a2 = az.copy(), az.copy()
        a1[k] -= h
        a2[k] += h
        j_az[k] = (_alpha(a2, el, t_az, t_el)
                   - _alpha(a1, el, t_az, t_el)) / (2 * h)
    j_tel = (_alpha(az, el, t_az, t_el + h)
             - _alpha(az, el, t_az, t_el - h)) / (2 * h)
    j_taz = (_alpha(az, el, t_az + h, t_el)
             - _alpha(az, el, t_az - h, t_el)) / (2 * h)
    return {"el": j_el, "az": j_az, "t_el": j_tel, "t_az": j_taz}


def linear_sigma(jac: dict, b_el_rad: float, sig_az_rad: float,
                 sig_tel_rad: float) -> dict:
    """First-order sigma_alpha from uniform anchor-el bounds (+-b ->
    sigma b/sqrt(3)), Gaussian az sigma, Gaussian target-el sigma; plus
    the per-source variance shares."""
    s_el = b_el_rad / np.sqrt(3.0)
    v_el = float(np.sum((jac["el"] * s_el) ** 2))
    v_az = float(np.sum((jac["az"] * sig_az_rad) ** 2))
    v_tel = float((jac["t_el"] * sig_tel_rad) ** 2)
    v = v_el + v_az + v_tel
    return {"sigma": float(np.sqrt(v)),
            "share_el": v_el / v if v > 0 else 0.0,
            "share_az": v_az / v if v > 0 else 0.0,
            "share_t_el": v_tel / v if v > 0 else 0.0}


def mc_alpha(az: np.ndarray, el0: np.ndarray, t_az: float, t_el0: float,
             b_el_rad: float, sig_az_rad: float, sig_tel_rad: float,
             n: int = N_MC, seed: int = 0) -> dict:
    """Seeded MC of alpha under anchor-el uniform bounds around el0,
    Gaussian anchor-az error, Gaussian target-el error. Returns the
    stats the gate consumes."""
    rng = np.random.default_rng(seed)
    az = np.asarray(az, dtype=float)
    el0 = np.asarray(el0, dtype=float)
    draws = np.empty(n)
    for i in range(n):
        el = el0 + rng.uniform(-b_el_rad, b_el_rad, az.size)
        a = az + sig_az_rad * rng.standard_normal(az.size)
        te = t_el0 + sig_tel_rad * rng.standard_normal()
        draws[i] = _alpha(a, el, t_az, te)
    absd = np.abs(draws)
    return {"mean": float(draws.mean()), "std": float(draws.std()),
            "abs_p95": float(np.percentile(absd, 95)),
            "abs_max": float(absd.max()),
            "within_gate": bool(np.percentile(absd, 95) < ALPHA_GATE)}


def scenes() -> dict:
    """name -> (az_rad, el0_rad, t_az, t_el, el_known: bool)."""
    from .exp6_zaxis import SCENES as E6
    out = {}
    for name, sc in E6.items():
        az = np.arctan2(sc.U[:, 1], sc.U[:, 0])
        el = np.arcsin(np.clip(sc.U[:, 2], -1, 1))
        t_az = float(np.arctan2(sc.u_t[1], sc.u_t[0]))
        t_el = float(np.arcsin(np.clip(sc.u_t[2], -1, 1)))
        out[f"e6_{name}"] = (az, el, t_az, t_el, True)
    for name, az_deg in REAL_WINDOWS.items():
        az = np.deg2rad(np.array(az_deg))
        out[name] = (az, np.zeros(az.size), 0.0, 0.0, False)
    return out


def certification_bound(az, el0, t_az, t_el, sig_az_rad, sig_tel_rad,
                        seed: int) -> float:
    """Largest anchor-el bound (deg, on the EL_BOUNDS_DEG grid, 0.0 if
    none) whose p95 |alpha| stays under the gate."""
    best = 0.0
    for b in EL_BOUNDS_DEG:
        r = mc_alpha(az, el0, t_az, t_el, np.deg2rad(b), sig_az_rad,
                     sig_tel_rad, seed=seed)
        if not r["within_gate"]:
            break            # certificate = contiguous passes from below
        best = b
    return float(best)


def build() -> dict:
    sig_az = np.deg2rad(SIGMA_THETA_DEG)
    out = {"gate": ALPHA_GATE, "sigma_az_deg": SIGMA_THETA_DEG,
           "el_bounds_deg": EL_BOUNDS_DEG,
           "target_el_sigma_deg": TARGET_EL_SIGMA_DEG}

    # per-scene: jacobian summary, linearization check, certification map
    sc_out = {}
    for i, (name, (az, el0, t_az, t_el, el_known)) in \
            enumerate(sorted(scenes().items())):
        jac = alpha_jacobian(az, el0, t_az, t_el)
        # linearization check at a 0.1 deg bound, no target error
        b_small = np.deg2rad(0.1)
        lin = linear_sigma(jac, b_small, 0.0, 0.0)
        mc = mc_alpha(az, el0, t_az, t_el, b_small, 0.0, 0.0,
                      seed=100 + i)
        # variance decomposition at the reference config (5 deg bound,
        # grid az sigma, el-aperture target)
        dec = linear_sigma(jac, np.deg2rad(5.0), sig_az,
                           np.deg2rad(1.75))
        cert = {}
        for st in TARGET_EL_SIGMA_DEG:
            cert[f"tel_{st}"] = certification_bound(
                az, el0, t_az, t_el, sig_az, np.deg2rad(st),
                seed=200 + i)
        p95_map = {}
        for b in EL_BOUNDS_DEG:
            for st in TARGET_EL_SIGMA_DEG:
                r = mc_alpha(az, el0, t_az, t_el, np.deg2rad(b), sig_az,
                             np.deg2rad(st), seed=300 + i)
                p95_map[f"b{b}_tel{st}"] = r["abs_p95"]
        sc_out[name] = {
            "n_anchors": int(az.size), "el_known": bool(el_known),
            "alpha_nominal": _alpha(az, el0, t_az, t_el),
            "j_t_el": float(jac["t_el"]),
            "j_el_rms": float(np.sqrt(np.mean(jac["el"] ** 2))),
            "j_az_rms": float(np.sqrt(np.mean(jac["az"] ** 2))),
            "lin_sigma_0p1deg": lin["sigma"],
            "mc_sigma_0p1deg": mc["std"],
            "share_el": dec["share_el"], "share_az": dec["share_az"],
            "share_t_el": dec["share_t_el"],
            "cert_bound_deg": cert,
            "p95": p95_map,
        }
    out["scenes"] = sc_out

    # N-scaling: hallway-like fan of N anchors, 5-deg bound, known target
    ns = {}
    for j, n in enumerate(N_SCALING):
        az = np.deg2rad(np.linspace(-50, 50, n))
        r = mc_alpha(az, np.zeros(n), 0.0, 0.0, np.deg2rad(5.0), sig_az,
                     0.0, seed=400 + j)
        ns[str(n)] = {"abs_p95": r["abs_p95"], "std": r["std"]}
    out["n_scaling"] = ns
    return out


def make_figure(data: dict, path: str) -> None:
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    gate = data["gate"]
    fig, axs = plt.subplots(1, 3, figsize=(16.5, 4.8))
    a, b, c = axs

    # (a) p95 |alpha| vs anchor-el bound, per target-el sigma, real scenes
    bs = data["el_bounds_deg"]
    for st, color in zip(data["target_el_sigma_deg"],
                         ["tab:blue", "tab:orange", "tab:red"]):
        for name, sc in data["scenes"].items():
            if sc["el_known"]:
                continue
            y = [sc["p95"][f"b{bb}_tel{st}"] for bb in bs]
            a.plot(bs, y, "-", color=color, alpha=0.5, lw=1.2)
        a.plot([], [], "-", color=color,
               label=f"target el sigma {st} deg")
    a.axhline(gate, ls="--", color="dimgray")
    a.text(bs[0], gate * 1.06, f"gate {gate}", color="dimgray")
    a.set_xscale("log")
    a.set_yscale("log")
    a.set_xlabel("anchor elevation bound [deg]")
    a.set_ylabel("p95 |alpha|")
    a.set_title("Certification: the target-el term, not the\n"
                "anchors, decides the gate (5 real windows)")
    a.legend(fontsize=9)

    # (b) variance decomposition per scene at the reference config
    names = list(data["scenes"].keys())
    x = np.arange(len(names))
    for key, color, lab in [("share_el", "tab:blue", "anchor el"),
                            ("share_t_el", "tab:red", "target el"),
                            ("share_az", "gray", "az (grid)")]:
        bot = np.zeros(len(names))
        if key == "share_t_el":
            bot = np.array([data["scenes"][n]["share_el"]
                            for n in names])
        elif key == "share_az":
            bot = np.array([data["scenes"][n]["share_el"]
                            + data["scenes"][n]["share_t_el"]
                            for n in names])
        b.bar(x, [data["scenes"][n][key] for n in names], 0.6,
              bottom=bot, color=color, label=lab)
    b.set_xticks(x)
    b.set_xticklabels([n.replace("aspen_", "").replace("e6_", "")
                       for n in names], rotation=45, ha="right",
                      fontsize=8)
    b.set_ylabel("variance share")
    b.set_title("Where alpha's variance comes from\n"
                "(5 deg el bound, 0.72 deg az, 1.75 deg target el)")
    b.legend(fontsize=9)

    # (c) N-scaling of the anchor part
    ns = data["n_scaling"]
    nn = sorted(int(k) for k in ns)
    y = [ns[str(n)]["abs_p95"] for n in nn]
    c.loglog(nn, y, "o-", color="tab:blue", label="measured p95 |alpha|")
    ref = y[0] * np.sqrt(nn[0]) / np.sqrt(np.array(nn, dtype=float))
    c.loglog(nn, ref, "--", color="dimgray", label="1/sqrt(N)")
    c.axhline(gate, ls=":", color="dimgray")
    c.set_xlabel("anchor count N")
    c.set_ylabel("p95 |alpha| (target el known)")
    c.set_title("The anchor-el part averages down ~1/sqrt(N);\n"
                "the target-el part does not")
    c.legend(fontsize=9)

    fig.tight_layout()
    fig.savefig(path, dpi=130)
    plt.close(fig)


def _self_test() -> None:
    sig_az = np.deg2rad(SIGMA_THETA_DEG)
    sc = scenes()

    # (1) alpha_gain agrees with exp6's z_alias_gain on exp6 scenes
    from .exp6_zaxis import SCENES as E6, z_alias_gain
    for name, s6 in E6.items():
        az, el, t_az, t_el, _ = sc[f"e6_{name}"]
        assert abs(_alpha(az, el, t_az, t_el) - z_alias_gain(s6)) < 1e-12
    # (2) target-el sensitivity ~ -1 on a hallway geometry
    az, el, t_az, t_el, _ = sc["aspen_run1_408-458"]
    jac = alpha_jacobian(az, el, t_az, t_el)
    assert abs(jac["t_el"] + 1.0) < 0.1, jac["t_el"]
    # (3) linearization: MC sigma matches the Jacobian sigma at 0.1 deg
    lin = linear_sigma(jac, np.deg2rad(0.1), 0.0, 0.0)
    mc = mc_alpha(az, el, t_az, t_el, np.deg2rad(0.1), 0.0, 0.0, seed=1)
    assert abs(mc["std"] - lin["sigma"]) / lin["sigma"] < 0.08, \
        (mc["std"], lin["sigma"])
    # (4) the el-aperture target sigma alone busts the gate everywhere
    r = mc_alpha(az, el, t_az, t_el, 0.0, 0.0, np.deg2rad(1.75), seed=2)
    assert not r["within_gate"], r
    # (5) N-scaling is monotone decreasing and ~1/sqrt(N)
    p = []
    for j, n in enumerate([6, 24]):
        azn = np.deg2rad(np.linspace(-50, 50, n))
        p.append(mc_alpha(azn, np.zeros(n), 0.0, 0.0, np.deg2rad(5.0),
                          sig_az, 0.0, seed=10 + j)["abs_p95"])
    assert p[1] < p[0], p
    assert 0.35 < p[1] / p[0] < 0.75, p        # sqrt(6/24) = 0.5
    # (6) consistency with the committed exp5b per-dwell alpha bounds:
    # same construction (el-only draws, +-5 deg, target fixed) on the
    # same geometry must land in the same range (different rng streams)
    r5 = mc_alpha(az, el, t_az, t_el, np.deg2rad(5.0), 0.0, 0.0, seed=3)
    assert 0.02 < r5["abs_p95"] < 0.06, r5["abs_p95"]
    print("exp8 self_test OK: exp6 agreement exact, j_t_el "
          f"{jac['t_el']:+.3f}, lin-vs-MC {lin['sigma']:.2e}/"
          f"{mc['std']:.2e}, el-aperture target busts gate "
          f"(p95 {r['abs_p95']:.3f}), N-scaling {p[0]:.3f}->{p[1]:.3f}, "
          f"exp5b-consistent p95 {r5['abs_p95']:.3f}")


def main() -> None:
    check = "--check" in sys.argv
    if "--self-test" in sys.argv:
        _self_test()
        return
    os.makedirs(RESULTS_DIR, exist_ok=True)
    json_path = os.path.join(RESULTS_DIR, "exp8.json")
    fig_path = os.path.join(RESULTS_DIR, "fig_rbec_exp8.png")
    data = build()
    if check:
        with open(json_path) as fh:
            committed = json.load(fh)
        bad = _compare(data, committed)
        if bad:
            print(f"MISMATCH ({len(bad)} values):")
            for m in bad[:20]:
                print(" ", m)
            sys.exit(1)
        print(f"exp8_alpha_budget --check: regenerated grid matches "
              f"{os.path.relpath(json_path)} exactly")
        return
    with open(json_path, "w") as fh:
        json.dump(data, fh, indent=1)
    print(f"wrote {json_path}")
    try:
        make_figure(data, fig_path)
        print(f"wrote {fig_path}")
    except ImportError:
        print("matplotlib not available -- figure skipped, JSON written")


if __name__ == "__main__":
    main()
