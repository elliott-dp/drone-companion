# Primary-source verification pass — 2026-08

The Phase 10 documents were first drafted on a network where `ti.com`,
`arxiv.org`, `ieeexplore`, `pmc.ncbi.nlm.nih.gov`, `ecfr.gov`,
`docs.nvidia.com`, `expresslrs.org` and `docs.px4.io` were blocked, so many
claims carried **[corrob]** (assembled from search extracts, primary never
read) or **[unver]** (single source or inference) tags, and each document's
Part H listed what still had to be opened. This pass opened the primary
sources on an unblocked network, verified every tagged claim, answered the
survey's five open questions, and applied the corrections in place. Claims
that survived a primary read now carry **[prim]**.

**Method and its limits, honestly stated.** Twelve research work packages,
each reading the design docs first and then the actual primary documents
(PDFs downloaded and read page-by-page where fetching failed), with exact
section/table/page citations required and fabricated citations forbidden.
Verification status: the two most design-changing refutations (the 60 GHz
UAV carve-out; the ELRS EU868 LBT stubs) were **independently re-fetched and
confirmed** from the eCFR XML and the ExpressLRS source; one package
(TI software chain) ran twice independently and agreed on substance across
all six claims; the remaining verdicts rest on a single careful primary read
each. Scoreboard across 55 claims: **26 confirmed, 21 nuanced (right but
materially incomplete), 6 refuted, 2 not-found-as-stated** — every refutation
and nuance has been folded into the documents.

---

## 1. What changed (the headlines)

| # | Finding | Documents affected |
|---|---|---|
| 1 | **60 GHz is a lawful airborne band in both jurisdictions** — 47 CFR 15.255(b)(3) (July 2023) permits UAV radar at 60–64 GHz (≤ 20 dBm peak EIRP, ~50 % duty, ≤ 400 ft AGL), and EU 57–64 GHz generic SRD (100 mW e.i.r.p., EN 305 550) has no airborne restriction. The docs' "60 GHz is closed for aircraft too" was wrong. A 60 GHz TI sensor is the airborne fallback if 76–81 GHz authorisations stall. | harness A.2 |
| 2 | **ELRS EU868 has no LBT.** `LBT.cpp/.h` are compiled only for `Regulatory_Domain_EU_CE_2400`; EU868 gets no-op stubs and no duty-cycle limiter. There is also no 100 mW tier at 868 (25 mW e.r.p. in the ELRS hop sub-bands, EN 300 220-2 B.1). The real loss model is RF loss + stubborn-sender retries + 16-message TX buffer shedding. | harness A.1, fc_integration G.1–G.3, J7 |
| 3 | **The ~1 Hz APLL/VCO recalibration is real, documented, and non-disableable** (SPRACF4C §3.1/3.2/6; ICD Table 5.36) — but all *other* runtime cals (the documented jump source) are host-disableable and TI recommends freezing them in cascade. E10 is now instrumented: `ENABLE_CAL_REPORT` delivers a timestamped report per cal event. 30 s coherence is plausible per TI's own docs. | survey B.1, transport E10, dataset `device_state` |
| 4 | **The mmWave vitals range frontier is 7 m** (77 GHz, camera-guided beamforming, arXiv:2304.11057); nothing published beyond it, and no airborne vitals of any kind at the 5–30 m envelope. The 192-channel cascade buys +12–23 dB over every published system — D4 past 7 m is novel territory, not reproduction. | survey Part H, D4 |
| 5 | **Anchor-referenced ego-motion compensation is flight-proven at 77 GHz** (Stöckel et al., IEEE TRS 2024: walls as anchors, 98 % motion removal, 1.33 % respiration error; second-derivative unwrapping needed exactly as A5's arithmetic predicted). Dual-radar reference cancellation flew at 24 GHz in 2025 (sub-mm displacement under > 100 mm platform motion). The beam-as-reference-channel experiment remains unpublished — this aperture's unique contribution. | survey B.10, Part H |
| 6 | **External frame triggering is provisioned by TI**: J4 pin 64 `EXT_DIG_SYNC` is a documented alternative input to the U8 LMK00804B fan-out — Phase 11 "drive SYNC_IN" is an input-path selection, not board rework. | transport D.3 |
| 7 | **TDA2 Ethernet reality**: the stock capture usecase already streams (TFDTP, ~320–550 Mbit/s ceiling — not the assumed ~110 MB/s), but SSD capture and streaming are two different firmware images, so E3 (simultaneity) is expected to fail on stock firmware. Drop rates are periodicity-dependent: ~0.05–0.1 % at 50–100 ms frames (the VITALS regime), percent-level only at 4 ms. | transport B, C, E |
| 8 | **Path C prior art is strong**: RidgeRun ships a working AWR2243-over-CSI-2 V4L2 driver for Jetson (Xavier NX), and an AGX Orin r36.4.4 bring-up exists. The Orin Nano module has exactly four 2-lane CSI ports, but the devkit exposes only two — four coherent streams need a custom carrier. | transport C |
| 9 | Two latency figures in the ML sections were mis-sourced: 0.26 s/sample is DPDCNet on an RTX 3090 (activity recognition, per 10-s sample); "~1.7 s hybrid on a 1080 Ti" traces to **no paper** — the real number is 3.719 s/10-s sample, 3.157 s of it CPU-bound RoI selection, so the GPU-ratio extrapolation was invalid. Dwell-cadence conclusions survive. | survey C.2, realtime C.3, harness C.1 |
| 10 | Buried-victim physics got its honest number: concrete is ~1000 dB/m one-way (~20 dB/cm round trip) at 77–81 GHz per ITU-R P.2040-4 — "tens of dB/m" described the 1–4 GHz rescue bands. | survey Part E |

---

## 2. Per-package record

### 2.1 MMWCAS-RF-EVM (SWRU553A + SWRA574B)

Read: SWRU553A (Sep 2019, rev Feb 2020, 44 pp, in full); SWRA574B (rev Feb 2020).

* Slaves HW-triggered / master SW-triggered — **confirmed** (SWRA574B §5.3,
  Table 6; trigger slaves first, master last). Master emits a `DIG_SYNC_OUT`
  pulse **every frame**; SW-trigger start uncertainty is tens of µs but frame
  cadence is crystal-scheduled — supports the per-dwell ppm-drift fit.
* U8 = LMK00804B 1:4 sync buffer — **confirmed**; one output loops back to the
  master's own `DIG_SYNCIN` for delay matching. **Nuanced:** injection does not
  require rework — `EXT_DIG_SYNC` (J4 pin 64) is the provisioned alternative
  input. Candidate observation tap: J4 pin 83 (SYNCOUT, muxed with SOP1;
  pin-direction annotations inconsistent — bench-verify).
* Antenna numbers — **confirmed**: 192-element virtual array, 86 λ/2 azimuth
  positions (42.5 λ), 4-element MRA elevation spanning 3 λ, element 12 dBi,
  ±60°/±30° 3 dB FoV, λ at 78.5 GHz.
* "Regulatory notice" in the UG — **refuted**: §7 reads "No additional
  regulatory information is available for the MMWCAS-RF-EVM."
* New: inter-chip `DIG_SYNC_IN` imbalance is ns-class (Table 1); RF board is
  5 V / 8 A max (~40 W worst case) via J4/J5 from the DSP-EVM or bench J6;
  TI's own worked cascade example uses 50 ms framing = the project's 20 Hz.

### 2.2 MMWCAS-DSP-EVM / TDA2 (SPRUIS6 + Vision SDK 3.08 + TI E2E staff answers)

* Path A (SSD capture) is TI's recommended raw path — **confirmed** (SPRUIS6;
  Studio Cascade UG; E2E 953863/1174366: the shipped firmware cannot stream raw
  over Ethernet at all).
* "Drop rates of a few percent are normal" — **nuanced**: strongly
  periodicity-dependent (1.8–3.3 % at 4 ms → 0.05–0.1 % at 50–100 ms), plus a
  known last-frame-lost issue; E2 must sweep periodicity.
* `cascade_radar_capture_only` — **nuanced**: no modification needed; ships
  with `Network_tx` (TFDTP/TCP selectable, `network_rx.exe` host side, one
  radar frame split into 4 TFDTP frames by channel id). TFDTP is only
  supported on IPU1_1.
* SDK EOL / AWR2243-incompatible — **confirmed** (last release Dec 2019;
  Radar SDK 3.7/3.8 "does not work with AWR2243 Cascade Imaging Kit"; the
  patch has open failure reports). TI's supported flow: capture + offline
  MATLAB.
* SPRUIS6 hardware facts: 512 GB NVMe M.2 2280 on PCIe 2.0; 1 GbE DP83867
  (with IEEE 1588 start-of-frame detection — a time-sync hook); 12 V ≥ 3 A
  (12 V/5 A typical). SPRUIS6 predates the AWR2243 RF board (describes
  4× AWR1243P); hardware facts carry over, device statements do not.
* New: a 12-bit packed capture mode exists — free 25 % reduction; TDA2 GbE
  practical ceiling is ~320–550 Mbit/s TFDTP (plain NDK on M4: ~2 MB/s).

### 2.3 AWR2243 RF & calibration (SWRS223D + SPRACV2 + SPRACF4C + ICD 2.23)

* 1 Hz APLL/VCO runtime recalibration — **nuanced (mechanism confirmed)**:
  always-on, 1 s periodicity, executes in inter-frame idle (APLL 150 µs +
  VCO 300 µs + 50 µs apply), cannot be disabled; `CALIBRATION_PERIODICITY=1`
  is an invalid value that *stops* them — never use as a freeze. All other
  runtime cals are host-disableable via `PERIODIC_CALIB_ENABLE_MASK`; the ICD
  recommends disabling periodic cals in cascade "for phase synchronization".
  TI documents abrupt gain/phase steps from runtime cal updates (one RX gain
  code = 2 dB) but never quantifies the APLL/VCO phase step — E10 measures
  exactly that residual, instrumented via `ENABLE_CAL_REPORT` (per-event
  timestamp, die temperature, hardware-updated flag).
* Phase-vs-temperature — RX inter-channel ±3° over −40…140 °C (SWRS223D §7.7,
  with runtime cals enabled); TX phase-shifter ~±2.3° over temp rel. 25 °C,
  < ±2° after LUT correction (SPRACV2); gain ~0.4/0.2 dB per 10 °C frozen.
  Cascade recipe: freeze cals, factory save/restore, corner-reflector offsets,
  host-forced simultaneous temp-index transitions, DSP-subtract the jump;
  residual smooth drift delegated to application references (= the scene
  anchor).
* Phase noise — **confirmed**: −96 dBc/Hz (76–77 GHz VCO1) / −94 dBc/Hz
  (77–81 GHz VCO2) at 1 MHz offset; the only specified RF offset.
* 30 s coherency — **answered**: no TI coherence-duration spec exists;
  SPRACV2's whole scheme targets step-free absolute phase across frames.
  Residuals over 30 s with cals frozen: smooth thermal drift (compensable),
  0/1.1 ns bimodal chirp-start jitter (0.4° @ 1 MHz IF — favour low IF),
  and the unquantified 1 s APLL/VCO cal. Nothing precludes the window.
* New: `CALIB_MON_TIME_UNIT` valid 40–250 ms, ≥ 1 ms blank per unit for
  APLL+SYNTH, ≥ 250 µs contiguous idle needed; internal temp sensor is ±7 °C;
  static-clutter estimates must be reset after any calibration event.

### 2.4 TI software chain (rl_sensor.h + Industrial Toolbox 4.12.1 source)

* Frame-config constraints — **confirmed**; two gotchas: legacy `numLoops`
  max is 255 (a 256-chirp single-profile frame needs 2 chirp-RAM entries ×
  128 loops), and cascade slaves *must* be HW-triggered (300 ps inter-chip
  uncertainty).
* Successive phase differencing and impulse-noise removal — **confirmed at
  source level** (threshold 1.5, single-sample linear interpolation, applied
  to the differenced phase).
* Cardiac band — **nuanced**: 0.8–2.0 Hz is the *decision* band with a hard
  `MAX_HEART_RATE_BPM = 120` cap; the 68xx filter is actually 0.8–4.0 Hz
  (14xx: 0.8–2.0); only the 2nd breathing harmonic is cancelled; coefficients
  hard-coded for 20 fps; the 68xx phase-to-displacement constant is the
  77 GHz wavelength even under a 60 GHz profile (~28 % scale error).
* Circle-fitting DC compensation — **nuanced**: literature (CW) technique;
  TI's FMCW chain has none.
* Multi-cell fusion in TI's chain — **refuted**: strictly single-bin,
  single-RX, re-selected every 6.4 s. (Fusion is literature practice, with
  hard numbers: HR MAE 0.66 vs 1.97 BPM; 0.84 vs 3.99 in a stroke ward.)
* Estimator fusion + confidence metric — **confirmed at source level**.

### 2.5 US regulatory (eCFR current text + FCC documents)

* § 95.3331 closed use list and § 95.3333 airborne prohibition + required
  inhibit mechanism — **confirmed verbatim**. Rationale: RAS protection
  (DA-24-200), so a waiver is unlikely.
* Subpart M: licence-by-rule (§ 95.3305); 50 dBm average / 55 dBm peak EIRP
  (§ 95.3367, 1 MHz RBW); 15.253 is now [Reserved].
* "60 GHz closed for aircraft too" — **refuted**: 15.255(b)(3) permits UAV
  FDS/radar at 60–64 GHz, ≤ 20 dBm peak EIRP, off-times ≥ 2 ms summing
  ≥ 16.5 ms per 33 ms, ≤ 121.92 m AGL. *(Independently re-verified from the
  eCFR XML during this pass.)*
* Airborne route — **confirmed**: Part 5 experimental (Form 442 / STA ≤ 6 mo;
  grant must expressly authorise airborne operation; expect non-interference
  and RAS coordination). 227 UAS experimental approvals since Jan 2025; FCC
  DA 26-314 (Apr 2026) is consulting on drone-spectrum reforms — re-check
  periodically. Supply-chain note: FY2025 NDAA §1709 Covered-List additions
  for foreign UAS components (DA 25-1086).

### 2.6 EU regulatory (EN 302 264, EN 300 220-1/-2, ERC 70-03 Oct 2025, (EU) 2025/105, TR 104 078)

* 76–81 GHz scoping — **nuanced (conclusion holds)**: exclusion is by closed
  scoping ("road vehicle based radar functions"; EN 302 264 requires permanent
  fixed installation on a wheels/rails vehicle; aircraft only while taxiing).
  The sole airborne category (band 79b) is obstacle detection on **manned**
  EASA CS-27/CS-29 rotorcraft. Cite (EU) 2025/105, not 2019/1345. Airborne
  trials = individual national experimental authorisation. TR 104 078
  (2025-06) formally requests CEPT/ECC open 76–77 GHz (and 57–64 GHz) for
  onboard UAS radar — pending.
* "100 mW with LBT" — **refuted**: ELRS EU868 hops 13 channels across
  863.275–869.575 MHz; those sub-bands are 25 mW e.r.p. with or without LBT
  (only 869.4–869.65 allows 500 mW). Polite access replaces duty-cycle limits
  but caps cumulative TX at 100 s/hour per 200 kHz.
* LBT mechanics — **nuanced**: EN 300 220-1 §5.21 mandates ≥ 160 µs CCA and
  deferral-or-AFA-hop; dropping the packet interval is an implementation
  consequence of fixed TDMA slots, not standard-mandated.
* New: ERC 70-03's introduction is the basis for SRD-on-aircraft legality
  (ELRS-868-on-drone), previously assumed without citation; 57–64 GHz SRD
  (100 mW, EN 305 550) already permits airborne use in the EU.

### 2.7 ELRS + PX4 (expresslrs.org, firmware source, PX4 source)

* Bandwidth table — **confirmed** (every figure and percentage re-derived
  from the `/software/mavlink` throughput tables; the older
  `/info/telem-bandwidth` page is a different, non-MAVLink table). New:
  K1000 Full costs 10 dB sensitivity vs 200 Hz Full; Gemini doubles downlink;
  ELRS's suggested `MAV_0_RATE` 9600 B/s oversubscribes every 868 mode.
* EU868 LBT — **refuted** (see headline 2). *(Independently re-verified from
  `LBT.h` during this pass.)* Where LBT does exist (2.4 GHz CE), busy → skip
  with TX-done faked at nominal time — the "slots lost, not delayed"
  intuition was right in the wrong band.
* `RC_CHANNELS` in the onboard stream set — **confirmed** (20 Hz in ONBOARD
  and ONBOARD_LOW_BANDWIDTH).
* CRSF telemetry — **nuanced**: custom build (`crsf_rc` replacing `rc_input`);
  exactly five frames round-robin ~2 Hz each (battery, GPS, attitude, flight
  mode, fused-altitude-as-baro); RSSI/LQ are RX-side.
* "FC waits to be asked for streams" — **refuted for PX4**: instances start
  with `-x`, stream the profile set from boot, and PX4 does not implement
  `REQUEST_DATA_STREAM` at all. The failure mode is ArduPilot-shaped.
* EdgeTX custom-message limitation — **confirmed** (fixed `msgid` switch in
  `MAVLink.cpp`). New: only `MAV_COMP_ID_AUTOPILOT1` messages are converted
  (the vitals stream must be re-emitted by the autopilot instance — which the
  Part E design already does), and `STATUSTEXT` → Yaapu passthrough is a
  zero-display-code handset alert path.

### 2.8 Jetson Orin (Developer Guide, module/carrier/design-guide PDFs, NVIDIA staff posts)

* HTE/GTE — **confirmed** for Orin Nano; timestamps in TSC ticks (31.25 MHz,
  32 ns), userspace via GPIO-v2 `EVENT_CLOCK_HTE`; devkit 40-pin exposes only
  two AON pins (I2C0 SDA/SCL, with 1.5 kΩ pull-ups to 3.3 V); module AON
  GPIO03–06 need a custom carrier.
* PTP — **confirmed**: AGX Orin yes (MGBE), Orin NX/Nano no; the Realtek
  RTL8111H is on the **module**, so no carrier fixes it; retrofit = i210-class
  PCIe/M.2 NIC.
* No DLA/PVA, 1024 cores, 1.28 FP32 TFLOPs, 68 GB/s — **confirmed** from the
  datasheet (MAXN_SUPER: 2.08 TFLOPs / 102 GB/s / 67 sparse TOPS).
* CSI — **confirmed**: 8 lanes as four 2-lane ports (combinable 2×4-lane),
  2.5 Gbit/s/lane, 16 VCs; devkit exposes two ports only.
* Non-camera CSI capture — **nuanced upward**: RidgeRun ships an AWR2243
  V4L2 driver (Xavier NX, RAW8 2048×1 per chirp, up to 2 devices); AGX Orin
  r36.4.4 community bring-up exists. RAW16 framing is an alternative, not
  the established choice.
* NVMe — **nuanced**: 200–350 MB/s is a planning floor, not a measurement;
  community fio spans ~100–800 MB/s by drive on the Gen3 ×4 slot.

### 2.9 Literature: range, estimators (8 claims)

HMUSIC and autocorrelation numbers **confirmed** to their primaries (60 GHz /
12.8 s and 77 GHz / 0.7–0.84 m / ~50 s respectively); D3's 1–2 rpm / 3–5 BPM
baseline **confirmed** as a safe floor (best-in-class < 1 rpm; error U-shaped
in range, optimum ~0.7 m); VMD-over-EMD **confirmed** with the K/α caveat;
TI band **confirmed** (with the internal inconsistency noted); multi-cell
fusion **nuanced** ("standard practice" overstated — quantified benefit
instead); difference beamforming **corrected** (intra-chest harmonic
suppression, not sub-resolution separation — IEEE full text unavailable, so
that specific claim stays unverified); micro-Doppler presence **nuanced**
(solid within-dataset, degraded cross-dataset/tangential). Range frontier:
see headline 4.

### 2.10 Literature: airborne (5 claims)

Hover-vs-translate finding **confirmed** at its primary (Rong et al. 2021,
Fig. 4) but the band was 7–9 GHz UWB, **not sub-6**; anchor-method claim
**refuted in the doc's favour** (it is the demonstrated primary airborne
method — three independent groups, including 77 GHz); compensation-method
enumeration **expanded** (dual-radar ANC flew 2025; 62–69 GHz 4D-imaging
pipeline exists in simulation; beam-as-reference still unpublished); downwash
on clothing **confirmed unmeasured** (vegetation-only precedent); two-ray
false-breathing **confirmed as this doc's own synthesis** (adjacent evidence
only). The Stöckel/Fraunhofer 77 GHz airborne line was unknown to the docs
and is now cited throughout B.10.

### 2.11 Literature: clutter & RCS (3 claims)

λ/4 arithmetic **exact** (applicability rewritten as geometry-dependent
~λ/4–λ); concrete attenuation **corrected upward** (~1000 dB/m one-way at
77–81 GHz, ITU-R P.2040-4 — strengthening the surface-sensor conclusion);
fielded-rescue-systems claim **corrected** (below ~10 GHz, not 150 MHz–4 GHz;
DELSAR is seismic; RECCO is harmonic radar for a worn reflector). The 94/95
GHz Michigan terrain corpus (Ulaby/Nashashibi/Sarabandi) **exists** — the
"unmeasured σ⁰" claim was too strong; the true gaps are rubble σ⁰ at W-band,
elevated-aspect human RCS at 76–81 GHz, and 76–81 GHz terrain σ⁰ at 20–70°.

### 2.12 Literature: ML & datasets (8 claims)

ADCNet **confirmed** (arXiv-only; FFT-RadNet is the peer-reviewed anchor);
self-supervised denoise-then-classify and contrastive pretraining
**confirmed** (DPDCNet; MAE; RiCL — 20 % labels reaching supervised mAP);
cross-modal supervision **nuanced** (Radatron = camera + manual; RaDelft =
**lidar**-supervised; ColoRadar = pose-only — consider a lidar/depth camera
as the label source, not just RGB); the 0.26 s / "1.7 s" figures **corrected**
(headline 9); GAN/diffusion imaging **confirmed** (HawkEye; RA-L 2024
diffusion); automotive HWA **confirmed** (TI SWRU526, NXP SPT — noting the
AWR2243 front end itself has zero fixed-function DSP); HDF5 story
**confirmed** (SWMR restrictions; journaling never shipped since the 2008
RFC); ethics **nuanced** (approval precedes *collection*, not release —
FAU 85_15B, U Twente CIS 230671).

---

## 3. Still open after this pass

1. **Every bench measurement** (transport §E, realtime §E, dataset §C.5) —
   the pass sharpened several (E2 sweeps periodicity, E3 expected to fail on
   stock firmware, E10 instrumented) but measured none.
2. The APLL/VCO cal's actual phase-step magnitude (E10) — TI never publishes
   it.
3. Difference-beamforming's sub-resolution separation claim — primary is
   IEEE-paywalled; treat as unestablished.
4. Rubble σ⁰ at W-band, elevated-aspect human RCS at 76–81 GHz, downwash on
   clothing (D8) — unpublished anywhere; the harness can measure all three.
5. CEPT/ECC action on TR 104 078 (EU UAS radar at 76–77 GHz) and FCC
   DA 26-314 (drone-spectrum reforms) — regulatory watch items.
