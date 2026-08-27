# RBEC exp9 (thesis P3 + D.9) — the anchor-redundancy budget, jointly

> **Headline: nine anchors are sufficient at the design point — and the
> way D.9 had to be built is a finding in itself.** Three mechanisms
> spend the same anchor redundancy: dwell-level subset consensus (exp7),
> per-seam seam-RAIM (exp4), and the D.9 integer-chain RAIM that exp5b
> Part G's absorbing-miss finding mandated. Joint availability, measured
> over N × ghosts × per-gap IMU σ (module
> `tools/phase10/rbec/exp9_availability.py`, bundle `results/exp9.json`):
> at the ≤100 µm design budget (Part G projects ~55 µm at the 47 ms
> gap), the min-N answer to P3's "may exceed 9" is **5 / 6 / 9** for
> 0 / 1 / 2 ghosts — arm-independent, with 1000-dwell point estimates
> 0.994 / 0.995 / 0.999 (Wilson 95 % lower bounds 0.987 / 0.989 /
> 0.994). Between the budget and exp4's wall, anchor count helps but
> cannot restore ≥99 %; at the 450 µm wall availability is 0 at every
> N ≤ 15 *for the innovation-gated estimators tested* — the coherent-wrap
> 2π branch ambiguity, with a LAMBDA-class joint solve the open
> counter-candidate. Everything **[meas-sim]**, seeded,
> `--check`-reproducible (the check reruns the full grid, ~40 min).

## Part A — Models and their deliberate boundaries

* **Frame-level abstraction.** One phase sample per frame; within-burst
  unwrap assumed clean (exp3/4 located the failures at the seams). No
  APLL steps or casualty leakage — exp3/exp4 carry those (a probe with
  exp4's 4 mm respiration added shifts 300 µm failure rates by only
  ~2–3 points and the design point by zero).
* **D.9 is two-pass, and that is a measured necessity, not a style
  choice.** The first implementation excluded/snapped per seam only; an
  adversarial review caught it snapping *ghosts* (their wrapped
  self-tracking slips look exactly like good-anchor slips), and
  instrumentation found why: correlated mis-attribution is
  **common-mode in the per-seam innovations** and gets absorbed into the
  3-dof shared-error solve — the ghosts ride as inliers (inlier-history
  ~0.9) while the estimate corrupts (593 µm error under 2 ghosts). The
  per-seam statistic structurally cannot identify correlated ghosts;
  the dwell-level consensus can (exp7's result, re-derived here the
  hard way). Pass 1 chains and lets the dwell consensus identify the
  untrusted set; pass 2 re-chains with the verdict enforced. With the
  two-pass, the contaminated shared-error estimate improves 593 → 17 µm
  and the final pass's truth-side ghost-snap counter is asserted zero.
* **Availability** = target integer chain error-free AND the dwell
  consensus finds a rank-3 set of ≥ 4 (a real conjunct — no silent
  fallback) AND cardiac in-band residual < 0.110 rad.
* **Statistics**: 100 dwells/cell for the surface (1.00 there bounds
  failure below ~3 %); 1000 dwells/cell at the design point. Same seeds
  across arms — paired comparison.

## Part B — Results

| Per-gap IMU σ | ghosts 0 | ghosts 1 | ghosts 2 |
|---|---|---|---|
| **100 µm (design)** — min-N, deep-confirmed | **5** (0.994) | **6** (0.995) | **9** (0.999) |
| — same cells, plain arm (deep) | 0.994 | 0.991 | 0.999 |
| 300 µm — availability at N = 9, d9 / plain | 0.68 / 0.27 | 0.53 / 0.26 | 0.45 / 0.24 |
| 450 µm (exp4's wall) — any N ≤ 15, both arms | 0 | 0 | 0 |

1. **The P3 answer.** Min-N **5 / 6 / 9** at the design budget,
   arm-independent, deep-confirmed (the 100-dwell surface's optimistic
   N = 7 at 2 ghosts is exactly what the deep pass exists to catch:
   0.987 at n = 1000). Each ghost costs ~1.5–2 anchors of margin, and
   the consumption is consensus's. The floor is a cliff: N = 5 with 2
   ghosts avails 0.006. N = 12 reads 1.000 at n = 1000 — supportable as
   ≥ 99.7 % (rule of three), *not* as a three-nines guarantee.
2. **What D.9 buys, stated exactly.** At the design point the arms
   coincide at and above min-N (and slightly trail plain below it —
   0.80 vs 0.86 at N6/g2 — where neither certifies anyway). Its value
   is (a) at the 300 µm edge: ~2× plain's availability (0.68 vs 0.27
   clean) by snapping isolated slips (~6/dwell measured vs the ~9/dwell
   Gaussian-tail incidence — some slips still land uncorrected) and
   keeping the shared-error estimate clean under contamination (17 vs
   593 µm), and (b) architectural: it is the mechanism that makes the
   dwell-consensus verdict *causally usable* by the seam machinery.
3. **The honest negative, scoped.** Between the budget and the wall,
   anchor count helps (0.35 → 0.86 over N 5 → 15 at 300 µm, d9) but
   never restores ≥ 99 %. At the 450 µm wall availability is 0 at every
   N *for the innovation-gated estimators tested*: the shared error
   makes anchors wrap coherently and a common 2π branch shift is
   geometrically consistent with the innovations. An oracle shared-error
   probe confirms the wall is estimator-limited, not
   information-limited — so a LAMBDA-class joint integer solve (the
   0.45 mm prior is well inside the 1.9 mm lattice spacing) plausibly
   breaks it; scoped as the open question D.1, and the claim
   "redundancy cannot substitute for seed accuracy" is made only for
   the estimators tested.
4. **Consistency with exp4**: the 300 µm partial regime and the 450 µm
   dead wall reproduce exp4's "≲300 µm requirement, wall at ~450 µm"
   from an independent implementation.

## Part C — What this changes elsewhere

* **The doctrine row (thesis §3)**: anchor budget N ≥ 9 for the full
  estimator at the design IMU budget with up to 2 ghosts (≥ 99.4 % at
  95 % confidence); N = 12 is the margin recommendation (≥ 99.7 %, and
  it buys exp8's α-bound 1/√N lever with the same anchors).
* **exp5b D.9 follow-up: closed** — with its architecture corrected to
  two-pass and the reason documented as an honest negative (per-seam
  innovation RAIM is structurally blind to correlated mis-attribution;
  joins C9 beside "D_A cannot gate ghosts").
* **The F-series IMU budget (≤100 µm/gap) is the real gate**: Part G's
  ~55 µm projection at the 47 ms gap sits comfortably inside it.

## Part D — Follow-ups

1. Joint integer LS (LAMBDA-class) across anchors at the wall — the
   oracle probe says the information is there; the estimator is not.
2. Ghost counts > 2, heterogeneous anchor SNR (corner reflector), and
   uncorrelated-ghost topologies (the correlated case is the hard one
   measured here).
3. Availability under the *measured* hover sway spectrum (P5's ULogs)
   instead of the synthetic knee model.
4. Live-operation warmup: pass 2 consumes a dwell-level verdict, so the
   first dwell of a mission runs pass-1-only — quantify the exposure.
