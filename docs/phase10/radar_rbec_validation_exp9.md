# RBEC exp9 (thesis P3 + D.9) — the anchor-redundancy budget, jointly

> **Headline: nine anchors are exactly sufficient at the design point —
> and redundancy cannot substitute for seed accuracy.** Three mechanisms
> spend the same anchor redundancy: dwell-level subset consensus (exp7),
> per-seam seam-RAIM (exp4), and the new D.9 integer-chain RAIM that
> exp5b Part G's absorbing-miss finding mandated. Their joint
> availability, measured over N × ghosts × per-gap IMU σ (module
> `tools/phase10/rbec/exp9_availability.py`, bundle `results/exp9.json`):
> at the ≤100 µm design budget (Part G projects ~55 µm at the 47 ms
> gap), min-N for ≥99 % dwell availability is **5 / 6 / 9** for 0 / 1 / 2
> ghosts — confirmed at 1000 dwells/cell (0.994 / 0.991 / 0.998), with
> three-nines needing N = 12 at 2 ghosts. Above the budget no anchor
> count helps: at exp4's 450 µm wall availability is 0 at every N ≤ 15,
> because coherently wrapping anchors create a 2π branch ambiguity that
> innovation-only RAIM cannot break. Everything **[meas-sim]**, seeded,
> `--check`-reproducible (note: the check reruns the full grid, ~25 min).

## Part A — Models and their deliberate boundaries

* **Frame-level abstraction.** One phase sample per frame; within-burst
  unwrap assumed clean (exp3/4 located the failures at the seams). No
  APLL steps or casualty leakage — exp3/exp4 carry those; this
  experiment isolates the redundancy economics.
* **The three mechanisms, as designed**: D.9 = per-seam robust consensus
  over the innovations (minimal 3-subsets, batched; **raw** residual
  statistic — a wrapped statistic aliases a slipped anchor straight back
  into the inlier set and feeds its ±2π-offset innovation to the LS, a
  bug class the self-test now pins), excluding continuous outliers
  (ghosts) from the shared-error solve and snapping detected 2π·m slips
  back (de-absorbing the chain). Plain arm = exp4's seam-RAIM verbatim:
  unweighted LS over *all* anchors. Dwell-level consensus = exp7's,
  RMS-statistic form, on the unwrapped chains.
* **Availability** = target integer chain error-free AND consensus
  retains a rank-3 set of ≥ 4 AND cardiac in-band residual < 0.110 rad.
* **Ghosts** are exp7's mis-attribution (parent LOS 25° off): their
  innovations are continuous outliers, not 2π slips — D.9 must exclude,
  not snap, them; asserted in the self-test alongside the snap behavior
  on genuine slips.
* **Statistics**: 100 dwells/cell for the full surface (an availability
  of 1.00 there only bounds failure below ~3 % at 95 %); 1000
  dwells/cell for the design-point deep pass (1.000 bounds failure below
  ~0.3 %). Same seeds across arms — paired comparison.

## Part B — Results

| Per-gap IMU σ | ghosts 0 | ghosts 1 | ghosts 2 |
|---|---|---|---|
| **100 µm (design)** — min-N ≥ 99 % | **5** (0.994 @1000) | **6** (0.991) | **9** (0.998) |
| 300 µm (exp4's requirement edge) — availability at N = 9 | 0.68 d9 / 0.27 plain | 0.51 / 0.26 | 0.42 / 0.24 |
| 450 µm (exp4's wall) — any N ≤ 15 | 0 / 0 | 0 / 0 | 0 / 0 |

1. **The P3 answer.** "May exceed 9" resolves to: **9 is exactly
   sufficient at the design point with 2 ghosts** (0.998 at 1000
   dwells); each ghost costs ~1.5–2 anchors of margin (5 → 6 → 9), and
   the consumption is consensus's, not seam-RAIM's. Three-nines at 2
   ghosts needs N = 12. Below the floor the collapse is a cliff, not a
   slope: N = 5 with 2 ghosts avails 0.006.
2. **D.9 is load-bearing at the edge, not at the design point.** At
   100 µm both arms coincide (slips are rare). At 300 µm D.9 roughly
   doubles plain seam-RAIM's availability (0.68 vs 0.27 clean) by doing
   the two things Part G asked for: its ghost-excluded shared-error
   estimate is ~17× better under contamination (34 vs 593 µm, measured
   in the self-test), and it snaps isolated anchor slips back
   (~8/dwell at 300 µm, matching the Gaussian-tail prediction),
   de-absorbing the chains.
3. **The honest negative: redundancy cannot substitute for seed
   accuracy.** At 450 µm the availability is zero at every N — more
   anchors do not help, because the shared error's magnitude makes *all*
   anchors wrap coherently, and a common 2π branch shift is
   geometrically consistent with the innovations (the GNSS integer
   ambiguity, reborn). Innovation-only RAIM cannot break it; whether
   joint integer LS (LAMBDA-class) could is an open question noted in
   Part D, so the claim is scoped to the estimators tested.
4. **Consistency with exp4**: the 300 µm partial regime and the 450 µm
   dead wall reproduce exp4's "≲300 µm requirement, wall at ~450 µm"
   from an independent implementation — a cross-check between the two
   modules' physics.

## Part C — What this changes elsewhere

* **The doctrine row (thesis §3)**: anchor budget N ≥ 9 for the
  full estimator at the design IMU budget with up to 2 ghosts; N = 12
  buys three-nines and is the recommendation where beam-compute allows
  (it also buys α-bound margin — exp8's 1/√N lever, same anchors).
* **exp5b D.9 follow-up: closed.** The integer-chain RAIM exists,
  measured; its raw-residual statistic is the implementation note that
  matters (the wrapped variant silently reverts to plain seam-RAIM).
* **The F-series IMU budget (≤100 µm/gap) is confirmed as the real
  gate**: availability collapses within a factor ~3–4.5 above it and no
  redundancy rescues it. Part G's ~55 µm projection at the 47 ms gap
  sits comfortably inside.

## Part D — Follow-ups

1. Joint integer LS (LAMBDA-class) across anchors at the wall — could a
   lattice solve break the coherent-wrap branch ambiguity that defeats
   innovation-only RAIM? Bounds the "cannot substitute" claim.
2. Ghost counts > 2 and heterogeneous anchor SNR (corner reflector) in
   the availability surface.
3. Availability under the *measured* hover sway spectrum (P5's ULogs)
   instead of the synthetic knee model.
