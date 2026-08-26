#!/bin/sh
# Fetch the ColoRadar cascade calibration files (the contents of the
# dataset's calib.zip for the cascade sensor) from a public vendored copy
# (adnan-armouti/mm3DGS), so exp5_coloradar.py needs only the sequence data.
# Usage: ./fetch_coloradar_calib.sh <dataset_root>
set -e
ROOT="${1:?usage: fetch_coloradar_calib.sh <dataset_root>}"
DST="$ROOT/calib/cascade"
mkdir -p "$DST"
BASE="https://raw.githubusercontent.com/adnan-armouti/mm3DGS/main/mmir/preprocessing/calib/cascade"
for f in antenna_cfg.txt waveform_cfg.txt heatmap_cfg.txt \
         coupling_calib.txt phase_frequency_calib.txt; do
    echo "fetching $f"
    curl -fsSL "$BASE/$f" -o "$DST/$f"
done
# rig extrinsics (needed by the D.7 -rot seed modes): translation line +
# quaternion x y z w line per file
TDST="$ROOT/calib/transforms"
mkdir -p "$TDST"
TBASE="https://raw.githubusercontent.com/adnan-armouti/mm3DGS/main/mmir/preprocessing/calib/transforms"
for f in base_to_imu.txt base_to_cascade.txt base_to_single_chip.txt \
         base_to_vicon.txt base_to_lidar.txt; do
    echo "fetching transforms/$f"
    curl -fsSL "$TBASE/$f" -o "$TDST/$f"
done
echo "calib ready under $DST (+ transforms)"
