#!/usr/bin/env bash
# Roadmap items 1+13 — kernel zero-dep gate. See BLUEPRINT-ITEMS-01-13-ci-zero-dep-gate-2026-07-19.md
#
# Three checks, all against the dowiz-kernel DEFAULT no-dev build:
#   (A) full `cargo tree -e no-dev --locked --offline` name set ⊆ kernel/ZERO-DEP-ALLOWLIST.txt
#   (B) allowlist shrinks monotonically vs origin/main (growth => RED unless the gate itself is edited)
#   (C) sha256 of kernel/Cargo.lock is byte-identical before vs after the check (item 13 / §10-P6)
#
# Runs with --locked --offline so resolution never touches the network; CI additionally wraps this
# in `unshare -n` so the networking-disabled proof is continuous, not a one-off.
#
# SCOPE RULE: dowiz-kernel default no-dev build ONLY. Per-crate / workspace-wide gating is item 31.
set -euo pipefail
ALLOW=kernel/ZERO-DEP-ALLOWLIST.txt
DOC=docs/design/SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md

# (13) lockfile hash BEFORE — the gate must not mutate resolution state
h0=$(sha256sum kernel/Cargo.lock | cut -d' ' -f1)

# (1) actual tree: default features, no dev edges, locked, offline, names only
cargo tree --manifest-path kernel/Cargo.toml -e no-dev --locked --offline --prefix none \
  | awk '{print $1}' | sort -u | grep -v '^dowiz-kernel$' | grep -v '^$' > /tmp/zdg-actual.txt

grep -vE '^\s*(#|$)' "$ALLOW" | sort -u > /tmp/zdg-allow.txt

# GATE A — any dependency not in the allowlist ⇒ RED (fails on any new dependency)
new=$(comm -23 /tmp/zdg-actual.txt /tmp/zdg-allow.txt)
if [ -n "$new" ]; then
  echo "ZERO-DEP GATE RED: crate(s) in the kernel default tree but not allowlisted:" >&2
  echo "$new" >&2
  echo "The kernel is contractually zero-dep — see $DOC §0.1. New deps go through the item-25 procedure." >&2
  exit 1
fi

# GATE B — monotonic shrink: HEAD allowlist must be a subset of origin/main's
if base=$(git show origin/main:"$ALLOW" 2>/dev/null); then
  grown=$(comm -13 <(echo "$base" | grep -vE '^\s*(#|$)' | sort -u) /tmp/zdg-allow.txt)
  if [ -n "$grown" ]; then
    echo "ZERO-DEP GATE RED: allowlist GREW vs origin/main (shrink-only invariant):" >&2
    echo "$grown" >&2
    echo "Growth requires an explicit reviewed exception: edit the zero-dep-gate job itself in the same diff. See $DOC §0.1." >&2
    exit 1
  fi
fi   # first-ever commit of the allowlist: no baseline yet, Gate B vacuously green

# GATE C (13) — lockfile hash AFTER must be unchanged (P6: verdict is a function of the repo only)
h1=$(sha256sum kernel/Cargo.lock | cut -d' ' -f1)
if [ "$h0" != "$h1" ]; then
  echo "ZERO-DEP GATE RED: Cargo.lock changed during the check ($h0 -> $h1) — nondeterminism leak (item 13 / §10-P6)." >&2
  exit 1
fi

echo "zero-dep-gate GREEN: $(wc -l < /tmp/zdg-actual.txt) external crates, all allowlisted; allowlist shrink-only OK; lockfile hash stable ($h0)."
