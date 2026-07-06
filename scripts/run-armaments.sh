#!/usr/bin/env bash
# run-armaments.sh — the FAST governance-armament suite (STRUCTURE-UPGRADE Part B, B4/B5 wiring).
#
# WHY: 2026-07-06 we found guardrail-gate-armament.mjs had been RED for days — it wasn't on any
# enforced runner (not in pre-commit), so a stale assertion rotted unseen (the same error-class-7 the
# whole Part-B stack fights: "only hook-enforced artifacts survive"). These armaments are fast,
# deterministic, hermetic, and have no product-code dependency — so they can run DECOUPLED from the
# slow pnpm typecheck/build/docker pre-commit steps. Run this before any harness commit (incl.
# --no-verify), from pre-commit, and from the weekly curation / THE EYE.
#
# Exit non-zero if ANY governance armament fails. No product build, no network — seconds, not minutes.
set -uo pipefail
cd "$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel 2>/dev/null || echo .)" || exit 0

fail=0
run() {
  local name="$1"; shift
  if "$@" >/tmp/armament-$$.out 2>&1; then
    echo "  ✓ $name"
  else
    echo "  ✗ $name"; sed 's/^/      /' /tmp/armament-$$.out | tail -8; fail=1
  fi
}

echo "governance armaments:"
run "hook-matchers (every gate covers its tool lane, no silent unregister)" node scripts/guardrail-hook-matchers.mjs
run "gate-armament (serious/red-line/guard-bash armed, not just registered)" node scripts/guardrail-gate-armament.mjs
run "token-gates (dispatch warn/deny/fable/distill armed + over-block guards)" node scripts/guardrail-token-gates.mjs
run "audit-token-router --self-test (a/e → exit1, over-block guards)" node scripts/audit-token-router.mjs --self-test
run "ledger-integrity (unique rows)" node scripts/guardrail-ledger-integrity.mjs
rm -f /tmp/armament-$$.out 2>/dev/null || true

if [ "$fail" -ne 0 ]; then
  echo "✗ run-armaments: a governance armament FAILED — a gate is disarmed, over-blocking, or rotted." >&2
  exit 1
fi
echo "✓ run-armaments: all governance armaments green."
