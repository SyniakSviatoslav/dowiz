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
# --lib only: wasm-bindgen below consumes the LIB wasm artifact exclusively. The kernel
# also ships several native CLI [[bin]]s (lm/markov_attractor/fdr_recorder — real
# filesystem/process tools, never meant to target wasm32); building them here would just
# make an unrelated native-tool wasm32-compat regression block the web/ wasm surface.
# --features wasm: the `#[wasm_bindgen]` entry points (kernel/src/wasm.rs, all 24 `_js`
# exports web/ imports — order_js/geo_*/spectral_*/fsm_*) are gated behind the off-by-
# default `wasm` feature (kernel/Cargo.toml). Without it this step still "succeeds" but
# silently emits an EMPTY glue module with zero kernel exports — web/'s ESM imports of
# named exports like `place_order_js` then fail at module-load time.
cargo build --release --target wasm32-unknown-unknown --lib --features wasm \
    --manifest-path "$KERNEL_DIR/Cargo.toml"
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
