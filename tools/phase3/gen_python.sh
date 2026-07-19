#!/usr/bin/env bash
# ============================================================================
# gen_python.sh — generate the pymavlink Python bindings for cc_dialect
# (dev plan Phase 3.2: "a pymavlink script with the custom dialect").
#
# Python bindings are SCAFFOLDING (the dev plan's words): they exist to
# isolate "PX4 sends wrong" from "Rust decodes wrong". Like the Rust
# bindings they are NOT vendored — regenerated on demand from the same
# pinned toolchain (cc-dialect/.venv-mavgen, pymavlink==2.4.49) and the
# same staged XML set (cc_dialect.xml + pinned upstream includes), so they
# can never drift from the contract. Output: generated/cc_dialect.py
# (gitignored).
# ============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIALECT_DIR="$(cd "$SCRIPT_DIR/../../cc-dialect" && pwd)"
OUT_DIR="$SCRIPT_DIR/generated"
VENV="$DIALECT_DIR/.venv-mavgen"

[[ -x "$VENV/bin/python" ]] || {
    echo "[gen_python] creating mavgen venv"
    python3 -m venv "$VENV"
    "$VENV/bin/pip" install --quiet -r "$DIALECT_DIR/requirements.txt"
}

# stage the dialect + pinned includes (same rule as gen_c.sh / build.rs)
STAGE="$DIALECT_DIR/build/definitions-py"
rm -rf "$STAGE" "$OUT_DIR"
mkdir -p "$STAGE" "$OUT_DIR"
cp "$DIALECT_DIR/cc_dialect.xml" "$STAGE/"
cp "$DIALECT_DIR/upstream/common.xml" "$DIALECT_DIR/upstream/standard.xml" \
   "$DIALECT_DIR/upstream/minimal.xml" "$STAGE/"

# invoked as `python <script>` (venv shebangs break on paths with spaces)
PYTHONHASHSEED=0 "$VENV/bin/python" "$VENV/bin/mavgen.py" \
    --lang Python3 --wire-protocol 2.0 \
    --output "$OUT_DIR/cc_dialect.py" \
    "$STAGE/cc_dialect.xml"

"$VENV/bin/python" - "$OUT_DIR" <<'EOF'
import sys, importlib.util
spec = importlib.util.spec_from_file_location("cc_dialect", sys.argv[1] + "/cc_dialect.py")
mod = importlib.util.module_from_spec(spec); spec.loader.exec_module(mod)
n = len(mod.mavlink_map)
assert n >= 12, f"only {n} messages in map"
for mid in (54000, 54007, 54010, 54013, 0, 111):
    assert mid in mod.mavlink_map, f"missing msgid {mid}"
print(f"[gen_python] OK — {n} message types incl. CC_* block -> generated/cc_dialect.py")
EOF
