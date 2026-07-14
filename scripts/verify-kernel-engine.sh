#!/usr/bin/env bash
# Verified-by-Math regression gate for the canonical kernel + engine crates.
# One-shot: fails closed (exit != 0) on ANY test/build regression.
# Run locally before pushing; mirrors what CI should enforce.
set -euo pipefail
cd "$(dirname "$0")/.." || exit 1

echo "== dowiz kernel: cargo test =="
( cd kernel && cargo test --lib --quiet )
echo "   kernel OK"

echo "== dowiz engine: cargo test =="
( cd engine && cargo test --lib --quiet )
echo "   engine OK"

echo "== kernel: wasm32 release build (the geo *_js surface must compile to wasm) =="
( cd kernel && rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true \
    && cargo build --target wasm32-unknown-unknown --release --quiet )
echo "   wasm32 OK"

echo "== fmt check (changed files must be formatted) =="
( cd kernel && cargo fmt --check )
( cd engine && cargo fmt --check )
echo "   fmt OK"

echo "ALL GREEN — kernel + engine verified."
