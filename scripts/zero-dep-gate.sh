#!/usr/bin/env bash
# Roadmap items 1+13 (kernel) + item 31 (per-crate extension).
# See BLUEPRINT-ITEMS-01-13-ci-zero-dep-gate-2026-07-19.md (kernel gate)
#     BLUEPRINT-ITEM-31-enactment-per-crate-gate-2026-07-19.md (this parametrization).
#
# Usage:  zero-dep-gate.sh [<crate-dir>]     (no arg => kernel, backward-compatible).
#
# Three checks, all against the <crate> DEFAULT no-dev build:
#   (A) full `cargo tree -e no-dev --locked --offline` external name set
#       ⊆ <crate>/ZERO-DEP-ALLOWLIST.txt
#   (B) allowlist shrinks monotonically vs origin/main (growth => RED unless the gate itself
#       is edited in the same reviewed diff)
#   (C) sha256 of <crate>/Cargo.lock is byte-identical before vs after the check
#       (item 13 / §10-P6: the verdict is a pure function of the repo)
#
# Runs with --locked --offline so resolution never touches the network; CI additionally wraps
# this in `unshare -n` so the networking-disabled proof is continuous, not a one-off.
#
# SCOPE RULE: one crate's DEFAULT no-dev build per invocation. The roster
# `scripts/zero-dep-crates.txt` is looped over this script in the `zero-dep-gate` CI job.
# EXCLUSIONS (item 31 §2.5): `agent-governance-wasm` (absolute-path dep on /root/bebop-repo,
# CI-unresolvable — portability defect, its own follow-up) and `mesh-adapter` (relative
# ../../bebop-repo path, resolvable only in the dual-checkout layout — gated inside the
# existing `mesh-adapter` CI job, not the roster loop).
set -euo pipefail
CRATE="${1:-kernel}"                       # crate directory relative to repo root
ALLOW="$CRATE/ZERO-DEP-ALLOWLIST.txt"
LOCK="$CRATE/Cargo.lock"                    # Gate C hashes this (all rostered crates have one)
DOC=docs/design/SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md

if [ ! -f "$ALLOW" ]; then
  echo "ZERO-DEP GATE RED: no allowlist at $ALLOW (every rostered crate must declare one)." >&2
  exit 1
fi

# (13) lockfile hash BEFORE — the gate must not mutate resolution state
h0=$(sha256sum "$LOCK" | cut -d' ' -f1)

# (1) actual tree: default features, no dev edges, locked, offline, names only.
# NOTE: cargo tree stays in its own pipeline so a resolution failure (e.g. --locked
# mismatch) is still loud under pipefail; the root/path-dep filtering is a separate step
# whose grep is `|| true`-guarded so the ZERO-dep end state (item 5: every line filtered
# out ⇒ grep exit 1) reports an empty set instead of aborting.
#
# path-dep filter (item 31 §2.1): with `--prefix none`, cargo tree renders the root package
# and EVERY path dependency as `name vX.Y.Z (/abs/path)` and every registry crate as
# `name vX.Y.Z`. `grep -v ' (/'` removes the root and all in-tree/sibling path deps
# (dowiz-kernel, dowiz-engine, agent-facade, bebop crates, …) in one principled step —
# no hardcoded crate-name list to rot. `(proc-macro)`/`(*)` dedup markers do not contain
# ` (/` and pass through to awk unchanged; `awk 'NF'` drops any blank line.
cargo tree --manifest-path "$CRATE/Cargo.toml" -e no-dev --locked --offline --prefix none \
  > /tmp/zdg-raw.txt </dev/null
{ grep -v ' (/' /tmp/zdg-raw.txt || true; } | awk 'NF{print $1}' | sort -u > /tmp/zdg-actual.txt

{ grep -vE '^\s*(#|$)' "$ALLOW" || true; } | sort -u > /tmp/zdg-allow.txt

# GATE A — any dependency not in the allowlist ⇒ RED (fails on any new dependency)
new=$(comm -23 /tmp/zdg-actual.txt /tmp/zdg-allow.txt)
if [ -n "$new" ]; then
  echo "ZERO-DEP GATE RED [$CRATE]: crate(s) in the default no-dev tree but not allowlisted:" >&2
  echo "$new" >&2
  echo "New deps go through the item-25 dependency-replacement procedure. See $DOC §0.1." >&2
  exit 1
fi

# GATE B — monotonic shrink: HEAD allowlist must be a subset of origin/main's.
# IMPORTANT (poison avoidance): NEVER run a git command with a pathspec/treeish that can
# FAIL for a first-commit allowlist (`git show origin/main:$ALLOW`, `git cat-file -e ...`,
# and even `git ls-tree origin/main -- $ALLOW` all emit "fatal: path '…' exists on disk,
# but not in 'origin/main'"). Such a failing origin/main object access was observed to
# corrupt cargo's very next `rustc -` target-info probe in this shared-worktree environment.
# Instead list origin/main's whole tree WITHOUT a pathspec (cannot fail on a missing path)
# and test membership in the shell; `git show` then only runs on a blob known to exist.
if git ls-tree -r --name-only origin/main 2>/dev/null | grep -qxF "$ALLOW"; then
  base=$(git show "origin/main:$ALLOW")
  grown=$(comm -13 <(printf '%s\n' "$base" | grep -vE '^\s*(#|$)' | sort -u) /tmp/zdg-allow.txt)
  if [ -n "$grown" ]; then
    echo "ZERO-DEP GATE RED [$CRATE]: allowlist GREW vs origin/main (shrink-only invariant):" >&2
    echo "$grown" >&2
    echo "Growth requires an explicit reviewed exception: edit the zero-dep-gate job itself in the same diff. See $DOC §0.1." >&2
    exit 1
  fi
fi   # first-ever commit of the allowlist: no baseline yet, Gate B vacuously green

# GATE C (13) — lockfile hash AFTER must be unchanged (P6: verdict is a function of the repo only)
h1=$(sha256sum "$LOCK" | cut -d' ' -f1)
if [ "$h0" != "$h1" ]; then
  echo "ZERO-DEP GATE RED [$CRATE]: Cargo.lock changed during the check ($h0 -> $h1) — nondeterminism leak (item 13 / §10-P6)." >&2
  exit 1
fi

echo "zero-dep-gate GREEN [$CRATE]: $(grep -c . /tmp/zdg-actual.txt) external crates, all allowlisted; allowlist shrink-only OK; lockfile hash stable ($h0)."
