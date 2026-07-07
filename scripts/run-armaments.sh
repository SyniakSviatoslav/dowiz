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
run "subagent-return-guard (0-tool-use degenerate return blocks; real work passes)" node scripts/guardrail-subagent-return-guard.mjs
run "audit-token-router --self-test (a/e → exit1, over-block guards)" node scripts/audit-token-router.mjs --self-test
run "context-budget-guard --self-test (two-tier WARN/HARD ladder + RED WARN≠HARD + extraction)" bash .claude/hooks/context-budget-guard.sh --self-test
run "module-integrity --self-test (manifest-boundary DENY + over-block guards)" node scripts/module-integrity.mjs --self-test
run "legacy-freeze --self-test (route-count regex + freeze compare logic)" node scripts/guardrail-legacy-freeze.mjs --self-test
run "cloud-session-report --self-test (token/time parse + outbound secret-scan)" node scripts/cloud-session-report.mjs --self-test
run "ledger-integrity (unique rows)" node scripts/guardrail-ledger-integrity.mjs
run "falsifiable-proof (VbM: every enforced proof can go RED — no false-positive metrics)" node scripts/guardrail-falsifiable-proof.mjs
run "no-orphan-guardrails --self-test (flags an unwired guardrail)" node scripts/guardrail-no-orphan-guardrails.mjs --self-test
run "no-orphan-guardrails (every guardrail wired to a runner — no dead machinery)" node scripts/guardrail-no-orphan-guardrails.mjs
run "loop-registry-parity --self-test (lying-cert + bogus-citation go RED)" node scripts/guardrail-loop-registry-parity.mjs --self-test
run "loop-registry-parity (live: CERTIFIED⇒report exists; cited paths exist)" node scripts/guardrail-loop-registry-parity.mjs
run "circuits engine --self-test (red-line trips, warn advisory, require_together)" node scripts/run-circuits.mjs --self-test
# KNOWLEDGE-AS-CIRCUITS enforcement on the staged set: red-line circuits BLOCK the commit (exit 2 →
# run() fails), warn-level circuits are advisory (--warn-ok → exit 0, printed but non-blocking). This
# is the one line that turns the circuit registry from shelf-ware into a pre-commit gate (fable-audit
# finding #2). No staged files (weekly curation / THE EYE) → nothing to check → clean.
run "circuits --staged (registry red-line patterns block; warns advisory)" node scripts/run-circuits.mjs --staged --warn-ok
rm -f /tmp/armament-$$.out 2>/dev/null || true

if [ "$fail" -ne 0 ]; then
  echo "✗ run-armaments: a governance armament FAILED — a gate is disarmed, over-blocking, or rotted." >&2
  exit 1
fi
echo "✓ run-armaments: all governance armaments green."
