#!/usr/bin/env bash
# tools/kernel-check.sh — FAST iteration gate for the bare-metal kernel.
# Usage: bash tools/kernel-check.sh [--wasm]
# Runs ONLY what changed is gated by: cargo fmt --check + cargo test (kernel).
# --wasm also rebuilds the wasm target + regenerates glue (slower, use sparingly).
#
# Design for max iteration speed (operator 2026-07-13):
#   * incremental cargo (target/ persists) -> test binary runs in ~0.01s when built
#   * no monorepo, no pnpm, no eslint — legacy gone, so nothing else to wait on
#   * single source of truth (kernel) means one gate, not N language gates
set -euo pipefail
cd "$(dirname "$0")/.."
cd kernel

echo ":: fmt check"; cargo fmt --check || { echo "FMT FAIL — run: (cd kernel && cargo fmt)"; exit 1; }
echo ":: test"; cargo test 2>&1 | tail -4
if [ "${1:-}" = "--wasm" ]; then
  echo ":: wasm build"; cargo build --target wasm32-unknown-unknown --release 2>&1 | tail -2
fi
echo "OK kernel green"
