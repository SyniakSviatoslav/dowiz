#!/bin/sh
# Refresh the offline advisory data tools/gates/deps-audit.sh reads.
#   RustSec: a shallow git checkout of rustsec/advisory-db at $RUSTSEC_DB
#            (default ~/.cache/dowiz-deps/advisory-db). One git process, no cargo.
#   npm:     tools/deps/npm_audit.py refetches by itself when its cached answer is older than
#            --max-age-days, so it needs no step here; running it once warms the cache.
# The gate REFUSES (exit 2) when either source is missing or stale -- run this, then the gate.
set -eu
DB="${RUSTSEC_DB:-$HOME/.cache/dowiz-deps/advisory-db}"
if [ -d "$DB/.git" ]; then
  git -C "$DB" fetch --depth 1 --quiet origin HEAD
  git -C "$DB" reset --hard --quiet FETCH_HEAD
else
  mkdir -p "$(dirname "$DB")"
  git clone --depth 1 --quiet https://github.com/rustsec/advisory-db "$DB"
fi
echo "refresh: advisory-db at $DB is $(git -C "$DB" log -1 --format='%h %cd')"
