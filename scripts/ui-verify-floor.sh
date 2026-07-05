#!/usr/bin/env bash
# ui-verify-floor.sh — Layer 1 (FLOOR) of the UI Build-Verification Loop.
#
# The mechanical, always-available floor a per-change UI review runs FIRST (before the visual /
# vision layers). It only RUNS existing gates — it never reimplements them. A red floor = not done;
# the visual/vision review is moot until it's green. See docs/operating-model/ui-build-verification-loop.md.
#
# Usage: scripts/ui-verify-floor.sh            (whole repo)
#        scripts/ui-verify-floor.sh <path...>  (grep checks scoped to paths)
set -u
cd "$(git rev-parse --show-toplevel 2>/dev/null || echo .)"
SCOPE=("${@:-apps/web/src packages/ui/src}")
fail=0

echo "── FLOOR 1/5 · lint ──"
pnpm -s lint || fail=1

echo "── FLOOR 2/5 · lint:gates (fixtures) ──"
pnpm -s lint:gates >/dev/null 2>&1 && echo "ok" || { echo "lint:gates FAILED"; fail=1; }

echo "── FLOOR 3/5 · typecheck ──"
pnpm -s typecheck || fail=1

echo "── FLOOR 4/5 · i18n parity (never-miss translations) ──"
pnpm exec tsx scripts/i18n-parity.ts || fail=1

echo "── FLOOR 5/5 · design-drift greps (token/scale bypass) ──"
echo "• hardcoded colours past tokens.css:"
grep -rEn "#[0-9a-fA-F]{3,8}|rgb\(|hsl\(" ${SCOPE[@]} --include=*.tsx --include=*.css 2>/dev/null | grep -v tokens.css | head -20 || true
echo "• arbitrary Tailwind values (off-scale spacing/size):"
grep -rEn "\b(p|m|w|h|gap|text|rounded)-\[" ${SCOPE[@]} --include=*.tsx 2>/dev/null | head -20 || true

echo ""
[ "$fail" = 0 ] && echo "FLOOR GREEN — proceed to STATES → VISUAL → VISION." || { echo "FLOOR RED — fix before the visual/vision layers."; exit 1; }
