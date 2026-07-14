#!/usr/bin/env bash
# build-kernel-wasm.sh — assemble the kernel wasm surface the web/ frontend loads.
#
# Strategy (per directive: the UI engine relies on kernel/Rust math; JS/TS is legacy):
#   the kernel is the single source of truth. We compile it to wasm32 and emit the
#   wasm-bindgen JS glue. The web/ shell ONLY consumes that glue — it never re-implements
#   geo/spectral math.
#
# Outputs (gitignored build artifacts):
#   kernel/pkg/        nodejs glue   (used by node tests + can load in-browser)
#   kernel/pkg-web/    web (ES module) glue + inline wasm (used by web/index.html)
#
# Fail-closed: any step error aborts the whole build (set -e).
set -euo pipefail
cd "$(dirname "$0")/.."   # repo root

KERNEL_DIR=kernel
PKG="$KERNEL_DIR/pkg"
PKG_WEB="$KERNEL_DIR/pkg-web"

echo "== build kernel wasm32 (release) =="
cargo build --release --target wasm32-unknown-unknown --manifest-path "$KERNEL_DIR/Cargo.toml"
WASM="$KERNEL_DIR/target/wasm32-unknown-unknown/release/dowiz_kernel.wasm"
[ -f "$WASM" ] || { echo "FATAL: $WASM missing" >&2; exit 1; }

echo "== emit nodejs glue -> $PKG =="
rm -rf "$PKG"
wasm-bindgen "$WASM" --out-dir "$PKG" --target nodejs

echo "== emit web (esm+inlined) glue -> $PKG_WEB =="
rm -rf "$PKG_WEB"
wasm-bindgen "$WASM" --out-dir "$PKG_WEB" --target web --no-typescript

echo "== built =="
ls -la "$PKG" "$PKG_WEB"
echo "OK: kernel wasm surface assembled. Consume from web/ via the glue only."
