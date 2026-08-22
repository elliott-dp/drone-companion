# RBEC exp5b — the P1 upgrade harness, built and fixture-proven

> **Status: P1's machinery is implemented and validated end-to-end on a
> seeded synthetic fixture; the real-data rerun is one command away, blocked
> only on the sequence archive.** This pass implements every item of
> thesis_plan §4 P1 — per-dwell α report, co-range structural pre-filter,
> subset-consensus solve, IMU-seeded integers, full-sequence dwell
> processing — as `tools/phase10/rbec/exp5b_upgrade.py`, against the specs
> in [`radar_rbec_validation.md`](radar_rbec_validation.md) §F.4 and
> [`radar_rbec_validation_exp67.md`](radar_rbec_validation_exp67.md)
> §B.4/§C.4/§D. Everything measured here is **[meas]** on the synthetic
> fixture (committed bundle `results/exp5b.json`, regenerated exactly by
> `--check`); **no real-data number in this document supersedes F.4's** —
> the 0.23/0.35 mm / 555 µm results stand untouched until the rerun.

## Part A — Models and their deliberate boundaries

* **The fixture, not the hallway.** ec_hallways_run4 could not be obtained
  in this environment: ColoRadar+ is Globus-only (interactive login), the
  v1 SharePoint/Drive routes are gone, and no public mirror of the raw ADC
  exists (checked: GitHub code/LFS/releases, S3/GCS reachable hosts). The
  fixture is a 10-anchor hallway-like scene written in the exact ColoRadar
  on-disk layout (3,145,728-byte frames, same waveform constants as the
  vendored calib), processed by the same bridge code paths a real sequence
  would take. It validates the *machinery*; it neither replaces nor
  predicts the real-data numbers.
* **Ghost model = exp7's mis-attribution, realized in raw ADC.** Two
  injected scatterers carry a believed direction (their steering vector)
  while their range-phase follows the *parent* anchor's LOS — a coherent
  sidelobe/ring artifact. G1 sits in the parent's exact range bin (the
  pre-filter's catch); G2 three bins away and strong enough to rank inside
  the anchor set (the consensus solve's catch).
* **IMU is a seeding aid, not an INS.** The synthetic accelerometer is the
  true second derivative of a band-limited (0.1–2.2 Hz) sway plus bias and
  noise; dead-reckoning assumes world-aligned attitude and takes the
  dwell-start velocity from GT (central difference). The measured quantity
  is integer-fix agreement vs GT seeding — "how far IMU seeding gets" —
  not navigation accuracy. The real archive's `imu/` layout is encoded to
  the dev-kit convention and must be re-verified when data lands (the
  reader falls back to GT seeding with a warning).
* **α with unmeasured elevations is a bound, not a value.** Hallway wall
  anchors have no measured elevation; the per-dwell report therefore
  carries α at nominal el = 0 *and* the seeded p95 of |α| over a ±5°
  elevation bound. Target elevation is held nominal — its uncertainty is
  P2's question.

## Part B — What the fixture run measures

Committed bundle `results/exp5b.json` (figure `fig_rbec_exp5b.png`);
regenerate + verify with

```bash
python3 -m tools.phase10.rbec.exp5b_upgrade --self-test
python3 -m tools.phase10.rbec.exp5b_upgrade --check
```

| Quantity | Dwell 0 | Dwell 1 |
|---|---|---|
| Co-range pre-filter flags (of 2 injected, G1 + sidelobe artifacts) | 8 | 8 |
| Consensus set size / excluded (9 anchors, 1 admitted ghost) | 8 / **1** | 8 / **1** |
| Held-out residual, plain → consensus [µm] | 74.1 → 70.2 | 96.4 → **74.8** |
| Increment error vs truth, consensus [mm RMS x/y] | 0.042/0.027 | 0.045/0.028 |
| GT increment RMS [mm x/y] | 1.69/1.63 | 1.77/1.97 |
| IMU-seeded integer agreement vs GT-seeded | 96.7 % | 98.8 % |
| α nominal / p95 over ±5° el bound | 0.000 / 0.036 | 0.000 / 0.039 |

* **The P1 prediction's mechanism, demonstrated:** the admitted coherent
  ghost is excluded by consensus in every dwell (exactly one exclusion —
  no good anchor lost), and the held-out static-cell residual improves,
  mean 85.2 → 72.5 µm. On the real sequence this same pair of numbers
  (`holdout_um_plain` vs `holdout_um_consensus`) *is* the test of the
  on-record prediction ("the 555 µm improves, or the identical-range
  clusters weren't ghosts").
* **The co-range pre-filter works as the cheap structural gate:** the
  same-bin ghost never reaches the solve; the flag list per dwell is the
  diagnostic exp5's console output only hinted at, now recorded.
* **IMU seeding is viable at fixture error levels:** ≥ 96 % integer
  agreement per 4.8 s dwell (92.7 % over a stressed 9.6 s single dwell),
  degrading exactly as the bias/noise arithmetic predicts. The measurement
  to publish is the same statistic on the real IMU.
* **An informative α negative:** with 9 azimuth-spread anchors and *only a
  bound* (±5°) on elevations, p95 |α| ≈ 0.037 — **above** the 0.02 gate.
  Azimuth geometry alone cannot certify the 2-D solve when anchor
  elevations are unknown at that bound; certification needs per-anchor
  elevation knowledge (scene model, structure survey) or a tighter bound —
  exactly the quantity P2 must propagate. (The fixture's *true* drawn
  geometry gives α = −0.019, under the gate — consistent with exp67's
  single-draw hallway α = +0.006.)
* **Exactness check retained:** the noise-free drop-z error equals
  α·K·dz to < 1e-9 on the fixture geometry (exp6's law, assertion 6 of the
  self-test).

## Part C — The rerun, when data lands

```bash
./tools/phase10/rbec/fetch_coloradar_calib.sh <root>       # calib: works now
# place kitti/ec_hallways_run4/ under <root> (Globus: arpg.colorado.edu/coloradarplus)
python3 -m tools.phase10.rbec.exp5b_upgrade --root <root> \
    --sequence ec_hallways_run4 --seed-mode imu
```

writes `results/exp5b_ec_hallways_run4.json` + figure with the identical
per-dwell schema, over the full 2192 frames in 30 s dwells. Compare
`holdout_um_*` against F.4's 555 µm and settle the prediction either way.

## Part D — Follow-ups

1. Real-data rerun (above) — the only remaining P1 item, blocked on the
   archive. Highest-value user action alongside P4/P5 (thesis_plan §4).
2. Extrinsics: `gt_increments`/`imu_increments` still use GT world axes
   (exp5's documented TODO); the mm3DGS mirror also vendors
   `transforms/base_to_{cascade,imu}.txt` — wire before the rerun if
   increment-axis fidelity matters beyond magnitude scoring.
3. The α-bound negative feeds P2 directly: propagate per-anchor elevation
   uncertainty instead of a uniform bound.
4. Per-anchor measured SNR → per-anchor σ_φ in the consensus tolerance
   (currently the budget scalar 0.0115 rad).
