#!/usr/bin/env bash
# One-shot regression gate for the crates the LIVE PRODUCT is made of.
#
# WHY THIS EXISTS. `verify-kernel-engine.sh` covers the kernel and the engine,
# and CI runs kernel, engine and apps/courier. Nothing anywhere ran the crates
# that actually serve a venue: `bebop-store` (the storage format), `dowiz-core`,
# `dowiz-hub` (the order log, the catalogue, the tables that replaced D1) and
# `workers/api` (every route). Four hundred and fifty tests existed and no gate
# named them, which means a green CI badge said nothing about the product.
#
# Fails closed on any regression. Run it before pushing; the `hub` job in
# .github/workflows/ci.yml runs the same four commands.
set -euo pipefail
cd "$(dirname "$0")/.." || exit 1

# No workspace in this repo: each crate is standalone and must be entered.
# `cargo -p` from the root resolves the manifest and pulls the wrong target
# graph — a documented false-green trap (see CLAUDE.md, "Build model").
for crate in crates/bebop-store crates/dowiz-core crates/dowiz-hub workers/api; do
  echo "== $crate: cargo test =="
  ( cd "$crate" && cargo test --quiet )
  echo "   $crate OK"
done

echo "== the SQL ratchet (may only fall) =="
sh tools/gates/no-sql.sh
echo "   ratchet OK"

echo "== the file-size ratchet (may only fall) =="
sh tools/gates/file-size.sh
echo "   ratchet OK"

echo "== the conservation gate's own proof =="
node e2e/gates/conservation.prove.mjs
echo "   proof OK"

echo "ALL GREEN — the hub crates and the Worker are verified."
