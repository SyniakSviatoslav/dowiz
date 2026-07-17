#!/usr/bin/env bash
# verify-scope.sh — fast, scope-routed verification for a change (accelerates
# the agent's edit→verify iteration loop). Mirrors .husky/pre-commit routing
# but is callable ON-DEMAND (before staging/commit) so errors are caught early,
# saving the commit-retry cycle. FAIL-CLOSED: every touched scope is still gated,
# just the right gate instead of `pnpm -r build` for everything.
#
# NOTE (row 20 prune, 2026-07-17): the legacy JS/TS + web/Astro branches were
# removed — the legacy thin-layer (row 21) was deleted 2026-07-13, so eslint and
# `astro build` routing referenced deleted scopes. This guardrail now tracks only
# the surviving Rust surface (kernel + engine), matching reality.
#
# Usage:
#   bash tools/verify-scope.sh            # verifies staged (else unstaged) diff
#   bash tools/verify-scope.sh path/a path/b   # verifies explicit paths
set -u

resolve_files() {
  if [ "$#" -gt 0 ]; then
    printf '%s\n' "$@"
    return
  fi
  local staged
  staged=$(git diff --cached --name-only --diff-filter=ACM)
  if [ -n "$staged" ]; then printf '%s\n' "$staged"; return; fi
  git diff --name-only --diff-filter=ACM
}

FILES=$(resolve_files "$@")
touches() { echo "$FILES" | grep -qE "$1"; }

echo "=== verify-scope: routing by touched scope (fail-closed) ==="
[ -z "$FILES" ] && { echo "  no changed files detected — nothing to verify."; exit 0; }

echo "[1/2] kernel (Rust)..."
if touches '^kernel/'; then (cd kernel && cargo test) || exit 1
else echo "  skip (no kernel change)"; fi

echo "[2/2] engine (Rust)..."
if touches '^engine/'; then (cd engine && cargo test) || exit 1
else echo "  skip (no engine change)"; fi

echo "=== verify-scope: ALL GATES PASSED ==="
