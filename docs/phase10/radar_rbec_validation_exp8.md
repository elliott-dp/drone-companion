# RBEC exp8 (thesis P2) — alpha's own error bar, and who is allowed to certify the 2-D solve

> **Headline: the α gate is certifiable only when the casualty's elevation
> comes from outside the array.** Propagating α's input uncertainties
> (module `tools/phase10/rbec/exp8_alpha_budget.py`, bundle
> `results/exp8.json`, `--check`-reproducible) shows the target-elevation
> sensitivity is dα/d el_t = −cos(el_t) − sin(el_t)(û·g), i.e. ≥ 1 in
> magnitude on every scene measured: casualty-elevation error maps
> ≥ 1:1 into α and does not average down with anchor count.
> The elevation aperture's achievable 1.75° (exp6) therefore busts the
> 0.02 gate on its own — p95 |α| ≈ 0.061 with *perfect* anchors — while
> a known target elevation certifies every hallway-class scene at
> anchor-elevation bounds up to 2°. Everything **[meas]** (seeded MC) +
> **[calc]** (Jacobian, validated against MC to 0.3 %).

## Part A — Models and their deliberate boundaries

* **Inputs, not outputs.** This experiment propagates uncertainty in
  α's *inputs* (anchor az/el, target az/el) through the exact closed
  form. It says nothing new about what a large α costs — that is exp6's
  α·K·dz law, already verified to 1e-9.
* **Error models**: anchor elevations uniform in ±b (the bound-only
  situation of wall/hallway scenes, exactly as the exp5b per-dwell
  report constructs it); anchor azimuths Gaussian at the 2.5°-grid
  quantization σ = 0.72°; target elevation Gaussian at σ ∈ {0, 0.5°,
  1.75°} — known / externally aided / measured by the elevation aperture
  (exp6's achievable 1.75°). p95 |α| is the gate statistic, matching the
  exp5b per-dwell report.
* **Scenes**: exp6's three synthetic geometries (true elevations known,
  nominal α ≠ 0) and the five real ASPEN still-window anchor sets from
  the committed exp5b guard-8 bundles (elevations unknown → bound-only,
  nominal α ≡ 0 by construction).
* **Correlated elevation errors are out of scope**: a common elevation
  bias (mounting pitch) enters α differently from independent per-anchor
  errors; the per-anchor-independent model matches the "unknown wall
  heights" situation the bound describes. Attitude-induced common tilt
  belongs to T2's gyro-prior treatment, not here.

## Part B — Results

Regenerate + verify:

```bash
python3 -m tools.phase10.rbec.exp8_alpha_budget --self-test
python3 -m tools.phase10.rbec.exp8_alpha_budget --check
```

**B.1 The certification map** (largest anchor-el bound, on the
{0.25, 0.5, 1, 2, 3, 5}° grid, keeping p95 |α| < 0.02):

| Scene | target el known | to 0.5° | to 1.75° (el aperture) |
|---|---|---|---|
| 5 × real ASPEN still windows | **2°** | 1° | **0 — never** |
| e6 hallway (nominal +0.006) | 2° | 0 | 0 |
| e6 hover_elevated (nominal −0.022) | 0 | 0 | 0 |
| e6 hover_ground (nominal −0.319) | 0 | 0 | 0 |

*(p95 is the gate statistic, matching exp5b's per-dwell report. A
certified cell is not exceedance-free: at the marginal cells 2–3.6 % of
draws still land above the gate — p99 reaches ~0.022–0.024 at the 2° /
known and 1° / 0.5° cells. Quote certification as "p95-certified" with
that fraction, or apply a p99 criterion and lose the run10/hallway 2°
cells.)*

Readings, in order of doctrine weight:

1. **The irreducible term.** dα/d el_t = −cos(el_t) − sin(el_t)(û·g):
   measured
   −1.000 on every el_t = 0 scene, −1.168/−1.339 on the hover scenes
   (the O(g) part grows with the anchors' z-content). At the reference
   config the target-el term carries ~70 % of α's variance. No anchor
   count or anchor-elevation knowledge helps it: **certifying the 2-D
   solve requires the casualty's elevation from outside the array** —
   rangefinder, scene geometry, or the constrain-z route. The elevation
   aperture (1.75°) gives p95 |α| ≈ 0.061 with perfect anchors — 3× the
   gate, always.
2. **The anchor part is manageable.** With target elevation known,
   1–2° anchor-elevation bounds certify every hallway-class scene, and
   the anchor part averages down as ~1/√N (measured p95 over N = 6→36:
   0.048 → 0.018; ratio at 6→24 is 0.48 vs √(6/24) = 0.50) — more
   anchors are a real lever for this term only.
3. **Azimuth is negligible — by MC, not by the linear shares**: on the
   real (el0 = 0) scenes the azimuth Jacobian is identically zero at the
   expansion point (α ≡ 0 in az there), so the panel-(b) zeros are
   structural; an independent MC with/without azimuth noise puts its
   true contribution at 0.01–0.02 % of the variance. Negligible either
   way, stated for the right reason.
4. **Hover scenes never certify — for the right reason.** Their
   *nominal* α (−0.022, −0.319) already violates the gate before any
   uncertainty: the gate's job there was done by exp6 (constrain z);
   P2 adds that no uncertainty bookkeeping changes that verdict.
5. **Consistency**: the ±5° / known-target cell reproduces the exp5b
   per-dwell committed bounds (0.037–0.043) on the same geometries with
   independent draws — the two reports measure the same quantity.

**B.2 Validation carried in the self-test**: exact agreement with exp6's
`z_alias_gain`; Jacobian vs MC at 0.1° bounds to < 8 % (measured 0.3 %);
the el-aperture-busts-gate assertion; N-scaling monotone and within the
1/√N envelope; the exp5b cross-check.

## Part C — What this changes elsewhere

* **C4's caveat is closed**: the α gate's own uncertainty is now
  budgeted; the certification conditions above are the quotable form.
* **Ch5/Ch4**: quote the gate as *conditional*: "|α| < 0.02, certified
  under target-elevation knowledge ≤ 0.5–1° and anchor-elevation bounds
  ≤ 1–2°" — never as a bare scene property.
* **Payload doctrine**: the casualty-elevation input should come from
  the rangefinder/scene model channel the harness already carries, not
  from elevation beamforming — consistent with exp67 B.4's "do not
  invest in elevation super-resolution".

## Part D — Follow-ups

1. Correlated (common-tilt) elevation error as a T2-coupled variant.
2. The N-scaling lever suggests re-examining the anchor-count budget
   jointly with P3's availability answer (exp9) — more anchors help both
   α and consensus, and cost the same beam-forming compute.
