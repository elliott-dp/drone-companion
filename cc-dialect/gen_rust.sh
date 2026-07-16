#!/usr/bin/env bash
# ============================================================================
# gen_rust.sh — Rust binding generation check for cc_dialect (Phase 0.3)
#
# The Rust bindings are deliberately NOT vendored. rust-mavlink's supported
# custom-dialect path is build-time generation: crates/cc-protocol/build.rs
#   1. stages cc_dialect.xml + the pinned upstream includes into OUT_DIR,
#   2. runs mavlink-bindgen on them,
#   3. computes CC_DIALECT_HASH from the XML (same definition as hash.sh),
# and cc-protocol include!()s the result. The bindings therefore can never
# drift from the XML — they are regenerated on every build of the workspace.
#
# This script is the Phase 0.3 deliverable that *proves the wiring works*
# outside of a full workspace build, and is what CI calls:
#
#   ./gen_rust.sh --check    build cc-protocol (forces binding generation)
#   ./gen_rust.sh            build + run the cc-protocol test suite
#                            (golden round-trip + fuzz/property tests)
# ============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$WORKSPACE_ROOT"

[[ -f "crates/cc-protocol/build.rs" ]] || {
    echo "error: crates/cc-protocol/build.rs not found — workspace not wired" >&2
    exit 1
}

echo "[gen_rust] building cc-protocol (this regenerates the dialect bindings)"
cargo build --package cc-protocol

if [[ "${1:-}" != "--check" ]]; then
    echo "[gen_rust] running cc-protocol tests (golden round-trip + fuzz)"
    cargo test --package cc-protocol
fi

echo "[gen_rust] OK — bindings generated from cc-dialect/cc_dialect.xml at build time"
