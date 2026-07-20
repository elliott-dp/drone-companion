#!/usr/bin/env bash
# ============================================================================
# build_px4.sh — reproducible px4_sitl_default build for the CCFC fork
#
# Wraps the cmake+ninja invocation with the environment decisions pinned:
#   * GIT_SUBMODULES_ARE_EVIL=1  — we manage submodules explicitly (only the
#     set required for SITL is initialized; PX4's Makefile would otherwise
#     pull every simulator asset).
#   * Gazebo (gz) modules force-skipped via CMAKE_DISABLE_FIND_PACKAGE_*:
#     this machine's Homebrew gz install has broken cmake exports
#     (gz-gui8 Qt IMPORTED target), and Phase 2 verification deliberately
#     uses PX4's built-in SIH simulator instead (no external sim, works
#     headless on macOS). The gz_bridge/gz_plugins CMakeLists treat
#     "not found" as a clean skip.
#   * PX4's Python deps come from the fork-local .venv-px4
#     (Tools/setup/requirements.txt).
#
# Usage:  ./build_px4.sh [PX4_DIR]     (default: ../../../PX4-Autopilot-CCFC)
# ============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PX4_DIR="${1:-$(cd "$SCRIPT_DIR/../../../../PX4-Autopilot-CCFC" && pwd)}"

[[ -f "$PX4_DIR/CMakeLists.txt" ]] || { echo "error: PX4 tree not found at $PX4_DIR" >&2; exit 1; }
[[ -x "$PX4_DIR/.venv-px4/bin/python" ]] || {
    echo "[build_px4] creating PX4 python venv"
    python3 -m venv "$PX4_DIR/.venv-px4"
    "$PX4_DIR/.venv-px4/bin/pip" install --quiet -r "$PX4_DIR/Tools/setup/requirements.txt"
}

source "$PX4_DIR/.venv-px4/bin/activate"
export GIT_SUBMODULES_ARE_EVIL=1

# Phase 3: the mavlink module builds against cc_dialect (CONFIG_MAVLINK_DIALECT
# in the board config). The fork's mavlink CMakeLists installs the vendored XML
# into the mavlink submodule at configure time and hash-gates it against
# src/include/ccfc/cc_dialect_hash.h — no separate install step (an external
# installer script broke CI, which never ran it).

# Protobuf is disabled for the same reason as the gz packages: it is only
# consumed by simulation/gz_msgs (gated on `if (Protobuf_FOUND)`), and the
# Homebrew protobuf 35 headers require C++17 while that target compiles as
# C++14 — with SIH there is no gz, hence no protobuf need at all.
cmake -S "$PX4_DIR" -B "$PX4_DIR/build/px4_sitl_default" \
    -DCONFIG=px4_sitl_default \
    -G Ninja \
    -DCMAKE_DISABLE_FIND_PACKAGE_gz-transport=TRUE \
    -DCMAKE_DISABLE_FIND_PACKAGE_gz-sim=TRUE \
    -DCMAKE_DISABLE_FIND_PACKAGE_gz-sensors=TRUE \
    -DCMAKE_DISABLE_FIND_PACKAGE_gz-plugin=TRUE \
    -DCMAKE_DISABLE_FIND_PACKAGE_Protobuf=TRUE

ninja -C "$PX4_DIR/build/px4_sitl_default"
echo "[build_px4] OK -> $PX4_DIR/build/px4_sitl_default/bin/px4"
