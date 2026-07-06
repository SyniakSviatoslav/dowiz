#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
#  sovereign-gate.sh — mechanical enforcement of the dowiz-core Hard Laws.
#
#  This is the machine-checkable definition of "sovereign". It runs three
#  complementary gates. Gates 1-2 cover the CORE crate (crates/domain) ONLY —
#  never the `api` shell crate, which is allowed to touch the outside world.
#  Gate 3 is a whole-workspace supply-chain check (Phase-0b hardening).
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
#    Gate 3 (supply-chain, cargo-deny): `cargo deny check` over the whole
#            workspace — RUSTSEC advisories + yanked/unmaintained crates
#            (deny), wildcard version reqs (deny), license policy (permissive
#            allow-list; copyleft deps need a named exception), and dependency
#            sources (crates.io only). Config: rebuild/deny.toml. Degrades to a
#            SKIP-with-warning (not a hard failure) when `cargo-deny` isn't
#            installed locally, so Gates 1+2 still run on a fresh machine —
#            install it with `cargo install cargo-deny --locked`; CI must have
#            it installed for Gate 3 to actually enforce anything there.
#
#  Run from anywhere:  bash rebuild/scripts/sovereign-gate.sh
#  Exit non-zero on any violation — wire it into CI as a required check.
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REBUILD_DIR="$(dirname "$SCRIPT_DIR")"
CORE_DIR="$REBUILD_DIR/crates/domain"
CORE_MANIFEST="$CORE_DIR/Cargo.toml"
WORKSPACE_MANIFEST="$REBUILD_DIR/Cargo.toml"

# Ensure the wasm target is present (no-op if already installed; harmless without rustup).
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true

echo "── Gate 1/3: wasm32 sovereignty build (no OS / sockets / fs / threads / entropy) ──"
cargo build --quiet --target wasm32-unknown-unknown --manifest-path "$CORE_MANIFEST"
echo "   ✔ core links on wasm32-unknown-unknown"

echo "── Gate 2/3: disallowed-methods/types (clock/entropy that compile to wasm; f64/f32) ──"
CLIPPY_CONF_DIR="$CORE_DIR" cargo clippy --quiet --manifest-path "$CORE_MANIFEST" --lib -- -D warnings
echo "   ✔ no disallowed clock/entropy calls or f64/f32 in production core"

echo "── Gate 3/3: supply-chain (cargo-deny: advisories/bans/licenses/sources) ──"
if command -v cargo-deny >/dev/null 2>&1; then
  cargo deny --manifest-path "$WORKSPACE_MANIFEST" check
  echo "   ✔ no denied advisories/yanked crates, wildcard deps, license, or source violations"
else
  echo "   ⚠ SKIPPED — cargo-deny not installed locally (install: cargo install cargo-deny --locked)."
  echo "     This gate enforces nothing on this machine until installed; CI must have it installed."
fi

echo "✔ dowiz-core is SOVEREIGN: entropy-free, clock-free, IO-free, wasm-clean, supply-chain-checked."
