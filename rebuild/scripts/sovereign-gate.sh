#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
#  sovereign-gate.sh — mechanical enforcement of the dowiz-core Hard Laws.
#
#  This is the machine-checkable definition of "sovereign". It runs two
#  complementary gates over the CORE crate (crates/domain) ONLY — never the
#  `api` shell crate, which is allowed to touch the outside world.
#
#    Gate 1 (wasm32): the core must compile to wasm32-unknown-unknown. A clean
#            build PROVES the production dependency graph has no OS, no sockets,
#            no filesystem, no OS threads, and no entropy source. (If it links,
#            it is portable to the browser — the Phase One/Two readiness proof.)
#
#    Gate 2 (disallowed-methods + disallowed-types): catches the clock/entropy
#            calls that DO compile to wasm (SystemTime::now / Instant::now /
#            env::var) and would slip Gate 1, PLUS bans f64/f32 outright — float
#            arithmetic is not guaranteed bit-identical native↔wasm32, which would
#            silently break deterministic replay. Config: crates/domain/clippy.toml.
#            Scoped to `--lib` so #[cfg(test)] modules (which may use Uuid::new_v4
#            via the dev-dep) are exempt — only production core code is policed.
#
#  Run from anywhere:  bash rebuild/scripts/sovereign-gate.sh
#  Exit non-zero on any violation — wire it into CI as a required check.
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REBUILD_DIR="$(dirname "$SCRIPT_DIR")"
CORE_DIR="$REBUILD_DIR/crates/domain"
CORE_MANIFEST="$CORE_DIR/Cargo.toml"

# Ensure the wasm target is present (no-op if already installed; harmless without rustup).
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true

echo "── Gate 1/2: wasm32 sovereignty build (no OS / sockets / fs / threads / entropy) ──"
cargo build --quiet --target wasm32-unknown-unknown --manifest-path "$CORE_MANIFEST"
echo "   ✔ core links on wasm32-unknown-unknown"

echo "── Gate 2/2: disallowed-methods/types (clock/entropy that compile to wasm; f64/f32) ──"
CLIPPY_CONF_DIR="$CORE_DIR" cargo clippy --quiet --manifest-path "$CORE_MANIFEST" --lib -- -D warnings
echo "   ✔ no disallowed clock/entropy calls or f64/f32 in production core"

echo "✔ dowiz-core is SOVEREIGN: entropy-free, clock-free, IO-free, wasm-clean."
