#!/usr/bin/env bash
# ============================================================================
# gen_c.sh — MAVLink C binding generation for cc_dialect (dev plan Phase 0.3)
#
# What it does:
#   1. Creates/reuses a project-local Python venv (.venv-mavgen) with the
#      *pinned* pymavlink from requirements.txt (mavgen is the reference
#      MAVLink generator; pinning it pins the generated code).
#   2. Stages cc_dialect.xml together with the pinned upstream includes
#      (upstream/common.xml, standard.xml, minimal.xml) into build/definitions
#      so <include> resolution uses exactly the pinned files.
#   3. Runs mavgen (C, wire protocol 2.0) and vendors the output into
#      generated/c/  — this directory is COMMITTED. CI regenerates it and
#      fails on any diff (drift guard, dev plan Phase 0.4 job 1).
#   4. Normalizes the embedded build date in version.h files so regeneration
#      is byte-stable (a timestamp would make the CI drift check useless).
#   5. Refreshes generated/dialect_hash.{h,rs} via hash.sh.
#
# Usage:
#   ./gen_c.sh                  regenerate generated/c + hash artifacts
#   ./gen_c.sh --px4 <PX4_ROOT> additionally install cc_dialect.xml into a
#                               PX4 checkout (see note below)
#
# PX4 note: modern PX4 generates its MAVLink headers at firmware build time
# from XML inside its mavlink/mavlink submodule. "Vendoring into PX4"
# therefore means copying cc_dialect.xml into
#   <PX4_ROOT>/src/modules/mavlink/mavlink/message_definitions/v1.0/
# and selecting it in the board config: CONFIG_MAVLINK_DIALECT="cc_dialect".
# The headers vendored here in generated/c/ are for everything that is NOT
# the PX4 firmware build: the golden-vector generator, host-side C tests,
# and the CI drift check.
# ============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

PX4_ROOT=""
if [[ "${1:-}" == "--px4" ]]; then
    PX4_ROOT="${2:?usage: gen_c.sh --px4 <PX4_ROOT>}"
fi

VENV=".venv-mavgen"
BUILD="build"
STAGE="$BUILD/definitions"
OUT_TMP="$BUILD/c-out"
OUT_FINAL="generated/c"

# --- 1. pinned mavgen toolchain --------------------------------------------
if [[ ! -x "$VENV/bin/python" ]]; then
    echo "[gen_c] creating venv $VENV"
    python3 -m venv "$VENV"
fi
"$VENV/bin/pip" install --quiet --requirement requirements.txt
MAVGEN_VER="$("$VENV/bin/python" -c 'import pymavlink; print(pymavlink.__version__)')"
echo "[gen_c] pymavlink $MAVGEN_VER"

# --- 2. stage the dialect + pinned includes --------------------------------
rm -rf "$STAGE" "$OUT_TMP"
mkdir -p "$STAGE"
cp cc_dialect.xml "$STAGE/"
cp upstream/common.xml upstream/standard.xml upstream/minimal.xml "$STAGE/"

# --- 3. run mavgen ----------------------------------------------------------
# Invoked as `python <script>` (not the script directly, whose shebang breaks
# on paths containing spaces; not `-m pymavlink.generator.mavgen`, which the
# pip package deprecated as an executable module).
# PYTHONHASHSEED=0: mavgen embeds MAVLINK_*_XML_HASH values computed with
# Python's hash(), which is salted per process — pin the seed or every run
# produces a different vendored tree and the CI drift check is meaningless.
PYTHONHASHSEED=0 "$VENV/bin/python" "$VENV/bin/mavgen.py" \
    --lang C --wire-protocol 2.0 \
    --output "$OUT_TMP" \
    "$STAGE/cc_dialect.xml"

# --- 4. byte-stable output: normalize embedded build dates ------------------
# mavgen stamps version.h with the generation time; pin it so the vendored
# tree only changes when the XML (or mavgen version) changes.
find "$OUT_TMP" -name 'version.h' -print0 | while IFS= read -r -d '' f; do
    sed -i '' -e 's/^#define MAVLINK_BUILD_DATE ".*"/#define MAVLINK_BUILD_DATE "PINNED-BY-GEN_C_SH"/' "$f" 2>/dev/null \
        || sed -i -e 's/^#define MAVLINK_BUILD_DATE ".*"/#define MAVLINK_BUILD_DATE "PINNED-BY-GEN_C_SH"/' "$f"
done

# --- 5. vendor ---------------------------------------------------------------
rm -rf "$OUT_FINAL"
mkdir -p "$(dirname "$OUT_FINAL")"
mv "$OUT_TMP" "$OUT_FINAL"

# sanity: the headers we depend on must exist
for h in cc_dialect/mavlink.h cc_dialect/cc_dialect.h common/common.h; do
    [[ -f "$OUT_FINAL/$h" ]] || { echo "error: expected header missing: $OUT_FINAL/$h" >&2; exit 1; }
done

# --- 6. dialect hash ---------------------------------------------------------
./hash.sh

echo "[gen_c] vendored C headers -> $OUT_FINAL/"

# --- optional: install the XML into a PX4 checkout ---------------------------
if [[ -n "$PX4_ROOT" ]]; then
    DEST="$PX4_ROOT/src/modules/mavlink/mavlink/message_definitions/v1.0"
    [[ -d "$DEST" ]] || { echo "error: $DEST not found — is $PX4_ROOT a PX4 checkout with submodules?" >&2; exit 1; }
    cp cc_dialect.xml "$DEST/"
    echo "[gen_c] installed cc_dialect.xml -> $DEST/"
    echo "[gen_c] now set CONFIG_MAVLINK_DIALECT=\"cc_dialect\" in the board config"
    echo "        (e.g. boards/cuav/v6x/default.px4board) and rebuild."
fi
