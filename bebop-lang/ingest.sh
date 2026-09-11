#!/usr/bin/env bash
# ingest.sh — scaled ingest twin driver (A7 step 3).
# Generates an N-byte CSV file, compiles the bebop ingest program,
# runs all rows (bebop-cells, python, sqlite, rust-best, rust-common),
# prints ms, MB/s, maxrss KB, fold per row. Folds must agree.
set -euo pipefail
BEBOP_LANG="${BEBOP_LANG:-$(realpath "$(dirname "$0")")}"
BEBOP_BIN="${BEBOP_BIN:-$BEBOP_LANG/bebop.bin}"
SEED="$BEBOP_LANG/seed/build/seed"
GEN_INGEST="$BEBOP_LANG/tools/gen_ingest.py"
INGEST_BP="$BEBOP_LANG/bench/vs_rust/ingest.bp"
RUST_DIR="${RUST_DIR:-$BEBOP_LANG/bench/vs_rust}"
INGEST_TWIN="$BEBOP_LANG/bench/vs_rust/ingest_twin.py"

USAGE="$0 [bytes] [out-dir]"
TARGET="${1:-1048576}"   # default 1 MB
OUTDIR="${2:-/tmp/ingest_tw}"
mkdir -p "$OUTDIR"

DATA="$OUTDIR/ingest.txt"
BIN="$OUTDIR/ingest.bin"
BEST="$RUST_DIR/target/release/ingest_best"
COMMON="$RUST_DIR/target/release/ingest_common"

echo "=== ingest twin: $TARGET bytes ===" >&2

# 1. Generate the data file
python3 "$GEN_INGEST" "$TARGET" "$DATA"

# 2. Compile the bebop ingest program
"$SEED" "$BEBOP_BIN" compile "$INGEST_BP" "$BIN" >/dev/null 2>&1
[ -s "$BIN" ] || { echo "GUARD: ingest.bin empty/missing"; exit 1; }

# 3. Run the Python twin (which drives bebop + python + sqlite + rust)
python3 "$INGEST_TWIN" "$DATA" "$BIN"
