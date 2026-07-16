#!/usr/bin/env bash
# ============================================================================
# build_golden.sh — build + run the golden-vector generator (Phase 1.2)
#
# Compiles gen_golden.c against the VENDORED C headers (generated/c) and the
# current dialect_hash.h, runs it, and leaves the two committed artifacts in
# this directory:
#     golden_frames.bin     raw MAVLink 2 frames (the wire contract)
#     golden_manifest.txt   human/CI-readable frame table
#
# Run ../gen_c.sh first whenever cc_dialect.xml changed; CI runs both and
# fails if the committed artifacts differ from the fresh regeneration.
# (A shell script rather than a Makefile: make target paths break on the
# spaces in this project's absolute path.)
# ============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIALECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$SCRIPT_DIR"

[[ -f "$DIALECT_DIR/generated/c/cc_dialect/mavlink.h" ]] || {
    echo "error: vendored headers missing — run ../gen_c.sh first" >&2; exit 1; }
[[ -f "$DIALECT_DIR/generated/dialect_hash.h" ]] || {
    echo "error: dialect_hash.h missing — run ../hash.sh first" >&2; exit 1; }

mkdir -p "$DIALECT_DIR/build"

CC_BIN="${CC:-cc}"
"$CC_BIN" -std=c11 -O2 -Wall -Wextra -Werror -Wno-address-of-packed-member \
    -I "$DIALECT_DIR/generated/c" \
    -I "$DIALECT_DIR/generated" \
    gen_golden.c -o "$DIALECT_DIR/build/gen_golden"

"$DIALECT_DIR/build/gen_golden" "$SCRIPT_DIR"

echo "--- sha256 of committed artifacts ---"
if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 golden_frames.bin golden_manifest.txt
else
    sha256sum golden_frames.bin golden_manifest.txt
fi
