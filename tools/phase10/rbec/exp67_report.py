"""Regenerate the exp6/exp7 results bundle: exp67.json + fig_rbec_exp67.png.

Every number in ``docs/phase10/radar_rbec_validation_exp67.md`` comes from
the grid this module runs — the module mains print narrower slices (exp7's
estimator table there uses its N=9 default; the document's tables are the
N=12 grid). Deterministic end to end up to BLAS last-bit reordering: a
re-run reproduces the committed JSON to ~1e-12 relative, which is what
``--check`` verifies (rtol 1e-9 — far below any digit either document
reports).

Grid (documented defaults):
  el_sweep      exp6.score, 8 seeds, all three scenes, 14 sigma_el points
  tworay        exp6.doa_two_ray_bias_deg, el+az, companion amp 0.3/0.5/1.0
  ghost         exp7.run_dwell, N=12 anchors, offset 25 deg, 6 seeds,
                all five estimators, 0..6 ghosts
  ghost_offset  1 ghost of N=12, offset sweep, plain LS vs consensus

Usage:
    python3 -m tools.phase10.rbec.exp67_report            # write bundle
    python3 -m tools.phase10.rbec.exp67_report --check    # verify vs committed
"""

from __future__ import annotations

import json
import os
import sys

import numpy as np

from . import exp6_zaxis as e6
from . import exp7_ghost_anchors as e7
from .core import CARDIAC_RESIDUAL_BUDGET_RAD

RESULTS_DIR = os.path.normpath(os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "..", "..", "..", "docs", "phase10", "results"))

SIGMA_EL = [0.0, 0.1, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0, 7.5,
            10.0, 15.0, 20.0]
SEPS = [1.0, 2.0, 3.0, 5.0, 7.5, 10.0, 15.0, 20.0, 30.0]
AMPS = [0.3, 0.5, 1.0]
ESTIMATORS = ["none", "irls", "raim", "traim", "consensus"]
N_ANCHORS = 12
N_GHOST = range(0, 7)
OFFSETS = [0.5, 1.0, 2.0, 3.0, 5.0, 10.0, 25.0, 45.0]
SEEDS = range(6)


def build() -> dict:
    out: dict = {"budget": CARDIAC_RESIDUAL_BUDGET_RAD,
                 "hw": e6.hardware_elevation_error()}

    el: dict = {"sigma_el": SIGMA_EL, "scenes": {}}
    for sc in e6.SCENES.values():
        rows: dict = {t: [] for t in ("oracle3d", "radar3d", "drop_z")}
        for s in SIGMA_EL:
            r = e6.score(sc, s)
            for t in ("oracle3d", "radar3d", "drop_z"):
                rows[t].append(float(r[t][:, 0].mean()))
        rows["alpha"] = e6.z_alias_gain(sc)
        rows["crossover"] = e6.crossover_sigma_el(sc)
        rows["budget_lim"] = e6.budget_sigma_el(sc)
        el["scenes"][sc.name] = rows
    out["el_sweep"] = el

    tr: dict = {"sep": SEPS}
    for axis in ("el", "az"):
        for amp in AMPS:
            tr[f"{axis}_{amp}"] = [
                e6.doa_two_ray_bias_deg(s, amp, axis=axis)[1] for s in SEPS]
    out["tworay"] = tr

    gh: dict = {}
    for rob in ESTIMATORS:
        rows_l = []
        for ng in N_GHOST:
            rs = [e7.run_dwell(n_ghost=ng, n_anchors=N_ANCHORS, robust=rob,
                               seed=s) for s in SEEDS]
            rows_l.append({
                "n_ghost": ng,
                "cardiac": float(np.mean([r["cardiac_rad"] for r in rs])),
                "ghost_kept": (float(np.nanmean(
                    [r["ghost_retention"] for r in rs]))
                    if ng else float("nan")),
                "good_kept": float(np.mean([r["good_retention"]
                                            for r in rs]))})
        gh[rob] = rows_l
    out["ghost"] = gh

    go: dict = {"offset": OFFSETS}
    for rob in ("none", "consensus"):
        go[rob] = [float(np.mean(
            [e7.run_dwell(n_ghost=1, ghost_offset_deg=o, n_anchors=N_ANCHORS,
                          robust=rob, seed=s)["cardiac_rad"] for s in SEEDS]))
            for o in OFFSETS]
    out["ghost_offset"] = go
    return out


def make_figure(data: dict, path: str) -> None:
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    bud = data["budget"]
    fig, axs = plt.subplots(2, 3, figsize=(16.5, 11.0))
    (a, b, c), (d, e, f) = axs

    # (a) hover_ground elevation sweep
    sg = data["el_sweep"]["scenes"]["hover_ground"]
    x = data["el_sweep"]["sigma_el"][1:]          # skip 0 on the log axis
    a.loglog(x, sg["drop_z"][1:], "o-", color="tab:blue",
             label="drop z (exp5 today)")
    a.loglog(x, sg["radar3d"][1:], "o-", color="tab:red", label="solve 3-D")
    a.loglog(x, sg["oracle3d"][1:], "o-", color="gray",
             label="exact LOS (floor)")
    lo = min(data["tworay"]["el_0.3"][3], data["tworay"]["el_0.5"][3])
    hi = max(data["tworay"]["el_0.5"][5], data["tworay"]["el_0.3"][5])
    a.axvspan(lo, hi, color="tab:red", alpha=0.10)
    a.axhline(bud, ls="--", color="dimgray")
    a.set_title("Hover over ground: the 3-D solve\nfails before it helps")
    a.set_xlabel("anchor elevation error (deg, 1σ)")
    a.set_ylabel("cardiac residual (rad)")
    a.text(0.12, bud * 1.25, "cardiac budget", color="dimgray")
    a.legend(loc="center left", frameon=False)

    # (b) unresolved two-ray DoA error
    for amp, ls in zip(("0.3", "0.5", "1.0"), (":", "-", "--")):
        b.loglog(data["tworay"]["sep"], data["tworay"][f"el_{amp}"],
                 "o" + ls, color="tab:red")
        b.loglog(data["tworay"]["sep"], data["tworay"][f"az_{amp}"],
                 "s" + ls, color="tab:blue")
    b.axhline(data["el_sweep"]["scenes"]["hover_ground"]["budget_lim"],
              ls="--", color="dimgray")
    b.set_title("Unresolved multipath: the 16.9°\n"
                "elevation beam cannot separate it")
    b.set_xlabel("companion scatterer separation (deg)")
    b.set_ylabel("DoA error (deg, RMS)")
    b.text(1.05, 6.0, "elevation", color="tab:red", fontweight="bold")
    b.text(1.05, 0.02, "azimuth", color="tab:blue", fontweight="bold")
    b.text(4.0, 1.05, "3-D solve budget", color="dimgray")
    b.text(4.0, 0.045, "companion amplitude\n1.0 / 0.5 / 0.3×",
           color="dimgray")

    # (c) z-aliasing gain per scene
    names = ["hover_ground", "hover_elevated", "hallway"]
    labels = ["hover over ground", "hover + structures",
              "hallway\n(near-horizontal)"]
    vals = [abs(data["el_sweep"]["scenes"][n]["alpha"]) for n in names]
    cols = ["tab:red" if v > 0.1 else "tab:blue" for v in vals]
    c.barh(range(len(vals))[::-1], vals, color=cols, height=0.55)
    for i, v in enumerate(vals):
        c.text(v + 0.008, len(vals) - 1 - i, f"{v:.3f}", va="center")
        c.text(0.008, len(vals) - 1 - i + 0.42, labels[i], va="bottom")
    c.set_yticks([])
    c.set_xlim(0, 0.42)
    c.set_title("One scene scalar decides:\ndrop z only when α ≈ 0")
    c.set_xlabel("z-aliasing gain |α|")

    # (d) ghost tolerance, five estimators
    styles = {"none": ("plain LS", "gray"),
              "raim": ("per-frame RAIM", "tab:purple"),
              "irls": ("Huber IRLS", "orange"),
              "traim": ("temporal RAIM", "tab:blue"),
              "consensus": ("subset consensus", "tab:red")}
    for rob in ("none", "raim", "irls", "traim", "consensus"):
        rows = data["ghost"][rob]
        lw = 3.0 if rob == "consensus" else 1.8
        d.semilogy([r["n_ghost"] for r in rows], [r["cardiac"] for r in rows],
                   "o-", color=styles[rob][1], label=styles[rob][0], lw=lw)
    d.axhline(bud, ls="--", color="dimgray")
    d.text(0.1, bud * 1.25, "cardiac budget", color="dimgray")
    d.set_title(f"Ghost tolerance of {N_ANCHORS} anchors: plain LS\n"
                "fails at 2, consensus at 6")
    d.set_xlabel(f"ghost anchors (of {N_ANCHORS})")
    d.set_ylabel("cardiac residual (rad)")
    d.legend(frameon=False, fontsize=10)

    # (e) consensus retention vs contamination
    rows = data["ghost"]["consensus"]
    frac = [r["n_ghost"] / N_ANCHORS for r in rows]
    e.plot(frac, [r["good_kept"] for r in rows], "s-", color="tab:blue",
           lw=2.5, label="good anchors kept")
    e.plot(frac[1:], [r["ghost_kept"] for r in rows[1:]], "o-",
           color="tab:red", lw=2.5, label="ghosts retained")
    e.axvline(0.5, ls="--", color="dimgray")
    e.set_ylim(-0.05, 1.25)
    e.text(0.27, 0.7, "50 % breakdown point\n(theoretical maximum)",
           color="dimgray")
    e.text(0.02, 1.05, "good anchors kept", color="tab:blue")
    e.text(0.08, 0.08, "ghosts retained", color="tab:red")
    e.set_title("Consensus holds to the theoretical\nlimit, then fails "
                "abruptly")
    e.set_xlabel("contaminated fraction")
    e.set_ylabel("retention")

    # (f) single-ghost offset sweep
    f.loglog(data["ghost_offset"]["offset"], data["ghost_offset"]["none"],
             "o-", color="gray", label="plain LS")
    f.loglog(data["ghost_offset"]["offset"],
             data["ghost_offset"]["consensus"], "o-", color="tab:red",
             lw=3.0, label="subset consensus")
    f.axhline(bud, ls="--", color="dimgray")
    f.text(0.55, bud * 1.3, "cardiac budget", color="dimgray")
    f.text(6.0, 0.02, "plain LS", color="gray")
    f.text(1.0, 0.0058, "subset consensus", color="tab:red")
    f.set_title("A single ghost's cost grows with\noffset; consensus "
                "removes it")
    f.set_xlabel("ghost–parent angular offset (deg)")
    f.set_ylabel("cardiac residual (rad)")

    for ax in axs.ravel():
        ax.spines[["top", "right"]].set_visible(True)
    fig.tight_layout(h_pad=3.0)
    fig.savefig(path, dpi=130)
    plt.close(fig)


def _compare(new: dict, old: dict, path: str = "") -> list[str]:
    """Recursive exact-value comparison; returns list of mismatch messages."""
    bad: list[str] = []
    if isinstance(new, dict) and isinstance(old, dict):
        for k in set(new) | set(old):
            if k not in new or k not in old:
                bad.append(f"{path}/{k}: missing on one side")
            else:
                bad += _compare(new[k], old[k], f"{path}/{k}")
    elif isinstance(new, list) and isinstance(old, list):
        if len(new) != len(old):
            bad.append(f"{path}: length {len(new)} vs {len(old)}")
        else:
            for i, (a, b) in enumerate(zip(new, old)):
                bad += _compare(a, b, f"{path}[{i}]")
    else:
        an, ao = float(new), float(old)
        same = (np.isnan(an) and np.isnan(ao)) or bool(
            np.isclose(an, ao, rtol=1e-9, atol=1e-12, equal_nan=True))
        if not same:
            bad.append(f"{path}: {an!r} != {ao!r}")
    return bad


def main() -> None:
    check = "--check" in sys.argv
    json_path = os.path.join(RESULTS_DIR, "exp67.json")
    fig_path = os.path.join(RESULTS_DIR, "fig_rbec_exp67.png")

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
        print(f"exp67_report --check: regenerated grid matches "
              f"{os.path.relpath(json_path)} exactly")
        return

    os.makedirs(RESULTS_DIR, exist_ok=True)
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
