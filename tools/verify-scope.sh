#!/usr/bin/env bash
# verify-scope.sh — fast, scope-routed verification for a change (accelerates
# the agent's edit→verify iteration loop). Mirrors .husky/pre-commit routing
# but is callable ON-DEMAND (before staging/commit) so errors are caught early,
# saving the commit-retry cycle. FAIL-CLOSED: every touched scope is still gated,
# just the right gate instead of `pnpm -r build` for everything.
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
[ -z "$FILES" ] && echo "  no changed files detected — nothing to verify."

echo "[1/5] Lint staged JS/TS..."
JS=$(echo "$FILES" | grep -E '\.(ts|tsx|js|jsx|mjs|cjs)$')
if [ -n "$JS" ]; then npx eslint $JS || exit 1; else echo "  skip (no JS/TS)"; fi

echo "[1.5] i18n parity..."
if touches 'packages/ui/src/lib/i18n(-catalog)?\.ts$'; then
  pnpm exec tsx scripts/i18n-parity.ts || exit 1
else echo "  skip (no i18n change)"; fi

echo "[2/5] kernel (Rust)..."
if touches '^kernel/'; then (cd kernel && cargo test) || exit 1
else echo "  skip (no kernel change)"; fi

echo "[3/5] web (Astro island)..."
if touches '^web/'; then
  (cd web && npx astro build) || exit 1
  if touches '^web/src/lib/kernel/'; then
    (cd web/src/lib/kernel && node kernel.test.mjs) || exit 1
  fi
else echo "  skip (no web change)"; fi

echo "[4/5] workspace packages/apps (heavy build only for buildable files)..."
WS_BUILDABLE=$(echo "$FILES" | grep -E '^(apps|packages|tools|spikes)/' | grep -E '\.(ts|tsx|js|jsx|mjs|cjs)$|(^|/)(package\.json|tsconfig[^/]*\.json)$')
if [ -n "$WS_BUILDABLE" ]; then
  pnpm -r typecheck || exit 1
  pnpm -r build || exit 1
elif touches '^(apps|packages|tools|spikes)/'; then
  echo "  skip pnpm -r build (workspace change is script/docs only — no JS/TS/manifest)"
else echo "  skip (no workspace member change)"; fi

echo "[5/5] Fly.io config..."
if touches 'fly\.(toml|yaml|yml)$' && { command -v flyctl >/dev/null 2>&1 || command -v fly >/dev/null 2>&1; }; then
  flyctl config validate || fly config validate || exit 1
else echo "  skip (no fly config / CLI absent)"; fi

echo "=== verify-scope: ALL GATES PASSED ==="
