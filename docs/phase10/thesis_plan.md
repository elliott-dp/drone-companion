# Phase 10 → thesis: organization and finishing plan

**Scope decision (2026-08-13): the thesis is the Phase 10 research programme
only.** The Phases 0–7 FC↔CC system is context — it appears as a short
platform-background section and as the integration target in the architecture
chapter, not as thesis chapters of its own.

This document is the working backbone for finishing: what the thesis claims
(§1), how existing material maps to chapters (§2), which claims carry which
evidence (§3), which experiments remain and what each buys (§4), the writing
order (§5), the asset inventory (§6), decisions only the author can make
(§7), and risks (§8). Update it as chapters land; it is the one place where
"what is left" is always current.

---

## §1 What the thesis is

**Draft thesis statement.** A 192-virtual-channel mmWave MIMO cascade on a
hovering UAV can recover cardiac-band chest displacement without any
cooperative reference, by synthesizing target and reference beams from the
*same datacube* and solving the platform's ego-motion from static-scene
anchors. The solve survives its real error sources if and only if it is (a)
budgeted against a taxonomy of eleven error terms, (b) gated by scene
geometry through a computable z-aliasing gain, (c) made robust to
mis-attributed anchors by subset consensus, and (d) stitched across
inter-burst gaps by anchor-based seam-RAIM. The dissertation develops the
method, closes its error budget analytically and in simulation, validates
the core solve on real cascade data at sub-millimetre agreement, validates
the downstream rate-estimation layer on clinical radar recordings against
ECG, and embeds all of it in a system architecture whose regulatory and
safety constraints are verified against primary sources.

**Title candidates** (pick late; the statement above is the anchor):

1. *Reference-Beam Ego-Motion Cancellation for UAV-Borne Vital-Sign Radar:
   Method, Validation, and System Architecture*
2. *Vital-Sign Sensing from a Hovering UAV with a mmWave MIMO Cascade: the
   RBEC Method*
3. *Same-Aperture Reference Beams for Airborne mmWave Vital-Sign Radar*

**Contributions** (each must survive an examiner asking "where is that
shown?" — pointers in §3):

- **C1 — The RBEC method itself**: same-cube target/reference beams, the
  T1–T11 error taxonomy, the shared-LO common-mode argument (range
  correlation; residuals scale with range *difference*), and the closed
  cardiac budget. [`radar_rbec_method.md`](radar_rbec_method.md) A–G.
- **C2 — Seam-RAIM**: anchors LS-solve the shared per-gap IMU error and
  correct the target's integer prediction; the unwrap cliff is removed at
  onset and the IMU requirement relaxes ≲100 → ≲300 µm per 47 ms gap.
  [`radar_rbec_validation.md`](radar_rbec_validation.md) E2.
- **C3 — Calibration-spur geometry**: correlated per-chip calibration errors
  produce *map-predictable discrete spurs*, not a raised floor; with the
  real TIDUEN5A map the binding spur sits at 2.6° → anchor-separation rule
  ≥ 3° (4° preferred). Validation B + §E2.
- **C4 — The z-aliasing gain α**: a closed-form, scene-computable scalar
  that decides 2-D solve vs IMU-constrained z — and the measured proof that
  the elevation aperture (four distinct rows) cannot support a 3-D solve
  (achievable 1.75° RMS vs 0.94° budget).
  [`radar_rbec_validation_exp67.md`](radar_rbec_validation_exp67.md) B.
- **C5 — The ghost-anchor error class**: amplitude-dispersion gates are
  structurally blind to coherent ghosts; per-frame residual tests are the
  wrong statistic; subset consensus holds to the 50 % theoretical breakdown.
  exp67 doc C.
- **C6 — Real-data validation of the anchor solve**: on 4×AWR2243 cascade
  raw ADC (ColoRadar+), increment agreement 0.23/0.35 mm RMS (x/y) against
  50/68 mm of true motion; 555 µm held-out static-cell residual — the
  no-ground-truth metric. Validation F.4.
- **C7 — The confidence-gated estimator bank on clinical truth**: six
  cardiac estimators + fusion; at confidence ≥ 0.5, HR MAE 3.05 BPM (p90
  6.37) at 30 % coverage on Erlangen ECG-referenced recordings — the
  three-state report doctrine (number / no-number / don't-know) validated.
  [`radar_vitals_bank_validation.md`](radar_vitals_bank_validation.md).
- **C8 — Verified system doctrine**: the two-band regulatory strategy
  (76–81 GHz airborne prohibited US+EU; 60–64 GHz airborne lawful within
  limits), the airborne-transmit interlock, sync/dataset/real-time
  architecture — 55 claims checked against primary sources, 6 refuted and
  corrected.
  [`radar_primary_source_findings.md`](radar_primary_source_findings.md).
- **C9 — The honest-negative catalogue** (methodological thread, not a
  chapter): WLS gains nothing under systematic angle errors; the chest
  prior degrades seam-RAIM; post-hoc slip repair is impossible at the
  cliff; D_A cannot gate ghosts; elevation super-resolution cannot rescue
  the 3-D solve. Examiners probe exactly here — surface these
  deliberately.

---

## §2 Chapter map

| # | Chapter | Primary sources (already written) | Material | Remaining work |
|---|---------|-----------------------------------|---------:|----------------|
| 1 | Introduction: SAR casualty detection from a hovering UAV; why ego-motion is *the* problem; contributions | method A; survey A; new text | ~20 % | Write last. 6–10 pages incl. platform background (Phases 0–7 in ≤ 2 pages + citation of the repo) |
| 2 | Background & related work: radar vitals physics, estimator landscape, airborne prior art, the frontier (7 m record; UWB ≤ 5 m hover; anchor compensation flight-proven at 77 GHz) | survey B, C; findings 2.9–2.12; method I (prior-art table) | ~75 % | Reframe survey prose from "what to build" to "what is known"; import citations from findings (each claim already carries its source) |
| 3 | Regulatory & platform constraints: §95.3331/95.3333, EU closed scoping, the 60–64 GHz airborne route, ELRS/EU868 link, transmit interlock | findings 2.5–2.8; harness A.2; fc_integration G | ~80 % | Condense; add the "develop at 77 on ground / fly at 60–64 / Part 5 for full cascade" strategy as a figure |
| 4 | The RBEC method: signal model, T1–T11, deterministic estimator + ANC variant, common-mode physics, budgets, failure modes, falsifiers | method A–G, J | ~85 % | Fold in the exp-driven amendments (validation E; exp67): α gate into the estimator section, consensus into anchor admission, seam-RAIM as first-class. Unify notation |
| 5 | Numerical validation: spur MC, GDOP, end-to-end budget verdict, cliff + seam-RAIM, leakage topologies, z-gate, ghost consensus | validation A–F; exp67 A–D; results/ bundle | ~85 % | Run P2, P3 (§4); regenerate all tables as figures (only exp67 has a committed figure today — replicate the `exp67_report` pattern for exp1–4) |
| 6 | Real-data validation: the cascade bridge, exp5 first results, upgraded run | validation F.4; bridge/exp5 code | ~50 % | P1 upgrade run (consensus + α + IMU-seeded integers + full sequence); P4 aspen_run9 if downloaded. This chapter grows the most |
| 7 | Rate estimation on clinical data: the bank, Erlangen benchmark, the bradycardia band lesson | vitals doc; vitals code + selftest | ~70 % | P6 widens to subjects 11–30 + scenarios; tail forensics; else publish as-is with stated coverage |
| 8 | System architecture: harness roles/two-path, transport & sync (HTE ledger), real-time budget, dataset & storage, FC integration | harness; transport; realtime; dataset; fc_integration | ~80 % | These are design docs — the chapter is a 15–25 page condensation with forward pointers to the repo, not a merge of five full docs |
| 9 | Bench protocol (and results, if hardware arrives): frozen-cal configuration, sync tap, E-tests; headline E10 (cal-report/APLL measurement) | bench manual 0–10; transport E | protocol ~90 %, results 0 % | Decide scope with supervisor (§7). Without hardware: present as designed-and-ready protocol + what each E-test would decide. V2 two-ray ground-anchor measurement now sharpened by exp6 |
| 10 | Conclusions & future work: what is proven at which tier, the honest negatives, falsifiers, V2–V6 ladder | method H, J; all Follow-ups sections | ~30 % | Write second-to-last; harvest every doc's follow-up list |
| A | Appendix: per-claim verification record | findings (verbatim) | 100 % | Format only |
| B | Appendix: reproducibility — seeded scripts, `--check` pattern, dataset access, licences | rbec/vitals READMEs | ~60 % | P10 fixtures; one table: script → doc table/figure it produces |
| C | Appendix: hover-capture flight card + ULog kit | HOVER_CAPTURE.md | 95 % | — |
| D | Appendix: dataset schema & bench data product | dataset B; bench 8 | 90 % | — |

**Structural rule that already exists in the material**: every results
chapter (5, 6, 7) inherits the "Part A — models and their deliberate
boundaries" discipline. Keep it — it is the strongest examiner-proofing the
docs have.

---

## §3 Claims & evidence register

Tier vocabulary (already used across the docs): **[prim]** primary-source
verified · **[meas-sim]** measured in seeded simulation · **[meas-real]**
measured on real-sensor data · **[meas-clin]** measured on clinical data ·
**[calc]** arithmetic from datasheet numbers · **[pending]** designed, not
yet run.

| Claim (headline form) | Number | Tier | Where | Upgrade path |
|---|---|---|---|---|
| Cardiac budget closes, worst-realistic | 0.095 vs 0.110 rad | [meas-sim] | validation D | P5 measured hover sway; bench V2 |
| Seam-RAIM removes the unwrap cliff; per-gap IMU need | ≲ 300 µm / 47 ms (wall ~450 µm) | [meas-sim] | validation E2 | P3 joint availability with consensus |
| Correlated cal → discrete spurs at map angles; separation rule | spur 2.6°; rule ≥ 3°/4° | [meas-sim]+[prim] | validation B, §E2 | bench E-series phase-cal report |
| True CMRR of the common-mode step train | 26–29 dB (geometric ceiling) | [meas-sim] | validation §E2 | **E10 on hardware** — the single most valuable bench number |
| α decides drop-z vs constrain-z; closed form exact | hallway +0.006 / ground −0.319; <1e-9 | [meas-sim] | exp67 B | P2 α error bar |
| Elevation aperture cannot carry a 3-D solve | 1.75° achievable vs 0.94° budget | [meas-sim] | exp67 B | bench V2 two-ray measurement (D.4) |
| D_A is ghost-blind; consensus breakdown | D_A 0.031 = parent; holds to 6/12 | [meas-sim] | exp67 C | P1 wires consensus into exp5 |
| Anchor solve tracks real cascade data | 0.23/0.35 mm vs 50/68 mm; 555 µm held-out | [meas-real] | validation F.4 | P1 (quality-gated), P4 (Vicon GT) |
| Conf-gated HR meets D3 when confident | MAE 3.05 BPM @ 30 % coverage | [meas-clin] | vitals doc | P6 subjects 11–30 + scenarios |
| Cardiac band must reach below 0.8 Hz | GDN0010 @ 46 BPM | [meas-clin] | vitals doc | — (documented tradeoff) |
| 76–81 GHz airborne prohibited (US, EU); 60–64 GHz lawful route | §95.3333; EN 305 550 | [prim] | findings 2.5/2.6 | — |
| APLL/VCO ~1 Hz always-on recal, in cardiac band, shared-LO | SPRACV2/ICD | [prim] | findings 2.3 | **E10 measures it** |
| Range frontier for published mmWave vitals | 7 m | [prim] | findings 2.9 | — |
| Honest negatives (WLS, chest prior, post-hoc repair, D_A, elevation) | — | [meas-sim] | validation E2/F; exp67 | present as results, not caveats |

---

## §4 Experiment matrix

**Done** (all committed, all seeded): exp1 spur MC · exp2 GDOP · exp3
end-to-end · exp4 seam-RAIM + leakage · exp5 first real-data results · exp6
z-gate · exp7 ghost consensus · exp67_report bundle (`--check` green) ·
Erlangen subjects 01–10 benchmark · bridge validated against real cascade
calib · bench analysis stack + manual (protocols ready).

**Pending, in recommended order:**

| P# | Experiment | Feeds | Cost | Blocked on | What it buys |
|----|-----------|-------|------|-----------|--------------|
| P1 | exp5 upgrade run: per-dwell α report, co-range structural pre-filter, subset-consensus solve, IMU-seeded integers, full 2192-frame sequence. **Harness done 2026-08-22** (`exp5b_upgrade.py`, fixture-proven + adversarially reviewed — [`radar_rbec_validation_exp5b.md`](radar_rbec_validation_exp5b.md)); only the data run remains | Ch6 | run: hours | **sequence archive** (Globus is interactive-only; see exp5b doc Part C) | Turns C6 from "quick harness" to defensible chapter core. Prediction on record: the 555 µm improves, or the identical-range clusters weren't ghosts — informative either way; the `--baseline` arm now attributes the change |
| P2 | Propagate α's own uncertainty (casualty elevation measured by the same weak aperture) | Ch5/Ch4 | hours | nothing | Closes the one open caveat on C4 before it is quoted |
| P3 | Consensus × seam-RAIM joint availability (both consume anchor redundancy; combined anchor-count requirement unknown, may exceed 9) | Ch5 | ~1 day | nothing | Doctrine-level number: minimum anchor count for the full estimator |
| P4 | aspen_run9 rerun (Vicon mm-class GT) | Ch6 | ½ day compute | **user: Globus download** | Upgrades C6's GT from interpolated 1.3 Hz pose to mm-class — the rigorous error bound |
| P5 | Hover ULogs → `hover_ingest` → exp3/exp4 on measured sway | Ch5 | ½ day + flights | **user: fly the flight card** | Replaces the last synthetic input (0.3 Hz-knee sway model) with the real platform spectrum |
| P6 | Erlangen subjects 11–30, Valsalva/Tilt scenarios, tail forensics (GDN0002/0008/0010), harmonic-aware joint estimation | Ch7 | 1–2 days | nothing (public data) | Coverage + the identified next lever on the tail |
| P7 | LCMV anchor→target null; RAIM behaviour when an anchor itself slips | Ch5 | 1 day | nothing | Completeness of the mitigation story |
| P8 | Mid-dwell ghost onset (detection latency of consensus) | Ch5/10 | ½ day | nothing | Closes exp67 C.5's stated gap |
| P9 | Bench: E-tests per manual; **E10 cal-report/APLL first**; V2 two-ray ground-anchor measurement at realistic depression angles | Ch9 | days–weeks | **hardware arrival** | E10: the method's central physical claim measured, not simulated. V2: decides whether exp6's 1.75° is pessimistic or optimistic |
| P10 | CC BY fixtures for vitals parity tests | App. B | hours | nothing | Reproducibility appendix completeness |

No-hardware minimum for a defensible thesis: P1–P3 (+ P6 if Ch7 feels
thin). P4/P5 are the two highest-value user actions. P9 upgrades the
tier of several [meas-sim] claims but the thesis stands without it if Ch9
is framed as protocol + decision table (§7 question 2).

---

## §5 Writing order

Principle: **freeze evidence per chapter, then write that chapter** —
never write against results that might still move.

1. **Ch4 (method)** — evidence already frozen; largest editing-only win.
   Fold validation-E and exp67 amendments in as you go.
2. **Ch5 (numerical validation)** — after P2/P3 land (small). Generate the
   missing figures (see §6).
3. **P1 → Ch6 (real data)** — run the upgrade, then write. If aspen_run9
   (P4) arrives while writing, it slots into the same chapter structure.
4. **Ch7 (clinical)** — as-is, or after P6.
5. **Ch2 + Ch3 (background, regulatory)** — mechanical: reframe survey +
   findings; citations are pre-verified.
6. **Ch8 (architecture)** — condensation pass over five docs.
7. **Ch9 (bench)** — protocol chapter; results only if P9 happened.
8. **Ch10, then Ch1** — conclusions before the introduction; the intro's
   contribution list is then just true.
9. **Appendices** — mostly formatting; do continuously.

Rough writing effort at thesis quality, given the material: Ch4 ≈ 4 d,
Ch5 ≈ 4 d, Ch6 ≈ 3 d (+P1), Ch7 ≈ 2 d, Ch2 ≈ 4 d, Ch3 ≈ 2 d, Ch8 ≈ 4 d,
Ch9 ≈ 2 d, Ch1+Ch10 ≈ 3 d, appendices ≈ 2 d → **~30 writing days** plus
P1–P3 (~3 days) and whatever of P4–P9 is in scope. No deadline is on
record — set one in §7 and back-plan.

---

## §6 Asset inventory

**Figures that exist**: `results/fig_rbec_exp67.png` (+ regenerating
driver). **Everything else is currently tables/console output.** The
`exp67_report` pattern (module → JSON bundle → figure → `--check`) is the
template; replicate it for exp1–4 (Ch5), exp5 (Ch6), Erlangen (Ch7).
Figures to produce, by chapter:

- Ch3: two-band regulatory strategy diagram; link budget table → chart.
- Ch4: beam geometry (target + anchors, one datacube); T1–T11 budget
  waterfall; seam-RAIM timing diagram.
- Ch5: spur spectrum with map-predicted angles overlaid; GDOP maps; cliff
  trace with/without seam-RAIM; CMRR vs step size; (from exp67, done).
- Ch6: increment scatter RBEC-vs-GT; held-out residual histogram; anchor
  map on the energy image with α and consensus set size per dwell.
- Ch7: per-subject MAE/coverage vs confidence gate; GDN0010 band case.
- Ch8: two-path architecture; three-way frame ledger; dataset schema.

**Datasets & licences** (verify exact citation text at writing time —
sources are in the findings doc): ColoRadar / ColoRadar+ (cite the dataset
paper + the + extension; check redistribution terms before shipping
fixtures), Erlangen clinical (figshare, **CC BY 4.0** — fixtures
redistributable with attribution), TI documents (cite, don't reproduce
figures — redraw).

**Code modules** (all cited as repo artifacts): `tools/phase10/rbec/*`
(core, array_model, geometry, endtoend, exp1–7, exp67_report,
coloradar_bridge, hover kit) and `tools/phase10/vitals/*` (dsp, estimators,
vmd, cw, bank, exp_erlangen, selftest).

**Writing workspace**: recommend a `thesis/` directory in this repo (LaTeX;
one file per chapter; bibliography seeded from the findings doc's
references). Scaffolding it is a half-day task — say the word and it can be
generated with the chapter skeletons and the §3 register pre-imported.

---

## §7 Open decisions (author/supervisor input needed)

1. **Institution mechanics**: template (LaTeX class?), language, page
   budget, deadline. Everything in §5 back-plans from the deadline.
2. **Is bench hardware in scope?** Decides Ch9's shape (protocol-only vs
   protocol+results) and whether P9 gates submission. Recommendation:
   plan the thesis to stand without it; treat E10 as the upgrade that
   lands if hardware does.
3. **Phases 0–7 presence**: recommended as ≤ 2 pages of platform background
   in Ch1 + the integration contract in Ch8 — confirm the supervisor
   agrees the FC↔CC system itself is out of scope.
4. **Publication strategy**: the RBEC method + validation (Ch4–6) is a
   self-contained paper; decide whether to submit before or after the
   thesis (affects writing order only mildly — the paper is a subset).
5. **Claim-tier presentation**: keep the [prim]/[meas-…] tags in the thesis
   text (distinctive, honest) or map them to conventional prose? Keep — but
   confirm the supervisor is comfortable.

---

## §8 Risks

- **Hardware schedule** (P9): mitigated — thesis stands on sim + real-data
  + clinical tiers; Ch9 is written as protocol either way.
- **aspen_run9 access** (P4): Globus route is live but manual; if it dies,
  Ch6 stands on ec_hallways + the P1 upgrade; state the GT limitation.
- **Scope creep**: the follow-up lists total far more than a thesis needs.
  The gate is §4's "no-hardware minimum" — everything else is
  future-work material for Ch10.
- **The two fragile claims to guard**: (a) exp5's sub-mm agreement is a
  2-D, hallway-geometry result — exp6's α analysis is the *reason* it
  held; always present them together. (b) The 3.05 BPM clinical number is
  *at 30 % coverage* — always quote coverage with accuracy, or an examiner
  will.
- **Multi-session git hazard** (operational): the branch is pushed from
  several sessions; after every fetch+rebase, verify the union (both
  sessions' commits present) before pushing.
