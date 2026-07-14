#!/usr/bin/env bash
# verify-safety-floor.sh — the self-limiting invariant for the self-ecosystem-evolution capability.
# The agent may evolve its own machinery (.claude/**), but the PRODUCT-SAFETY FLOOR — the red-line
# enforcement wired into the gates — may only be strengthened or scope-corrected, NEVER removed.
# This script asserts that floor is intact. If any check fails, the floor was breached.
#
# Modes:
#   • PostToolUse hook (stdin = tool JSON): runs the full floor check on any .claude/** edit; on
#     breach → exit 2 (loud alarm) + FLOOR-BREACH row in .claude/logs/self-mod.log.
#   • Standalone (`bash verify-safety-floor.sh`, no stdin): runs the check, prints PASS/FAIL, exits 0/1.
# Fail-CLOSED on a genuine breach; fail-open only if a gate file is missing on disk (repo mid-edit).
set -uo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"
H="$ROOT/.claude/hooks"; S="$ROOT/.claude"

INPUT="$(cat 2>/dev/null || true)"
if [ -n "$INPUT" ]; then
  fp="$(printf '%s' "$INPUT" | grep -oE '"file_path"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | sed -E 's/.*"([^"]*)".*/\1/')"
  # Only edits under .claude/** can lower the floor; anything else → nothing to check.
  [ -n "$fp" ] && ! printf '%s' "$fp" | grep -q '\.claude/' && exit 0
fi

# Each row: <file>|||<fixed substring that MUST still be present>|||<what it guards>
CHECKS=(
  "$H/protect-paths.sh|||migrations|||protect-paths blocks migrations/"
  "$H/protect-paths.sh|||packages/db/|||protect-paths blocks packages/db/"
  "$H/protect-paths.sh|||packages/shared-types/|||protect-paths blocks shared-types/"
  "$H/protect-paths.sh|||/contracts/|||protect-paths blocks /contracts/"
  "$H/protect-paths.sh|||\\.env|||protect-paths blocks .env (root or nested)"
  "$H/protect-paths.sh|||.github|||protect-paths blocks .github/ (CI)"
  "$H/protect-paths.sh|||Dockerfile|||protect-paths blocks Dockerfile"
  "$H/protect-paths.sh|||fly\\.toml|||protect-paths blocks fly.toml"
  "$H/protect-paths.sh|||serious-override|||protect-paths keeps auth tokens operator-only (no self-authorize)"
  "$H/guard-bash.sh|||(deploy|secrets)|||guard-bash blocks fly deploy/secrets"
  "$H/guard-bash.sh|||push\\s+--force|||guard-bash blocks git push --force"
  "$H/guard-bash.sh|||migrate:up|||guard-bash blocks pnpm migrate:up"
  "$H/guard-bash.sh|||(add|remove)|||guard-bash blocks pnpm add/remove"
  "$H/post-edit-gates.sh|||document\\.cookie|||post-edit-gates red-lines document.cookie"
  "$H/post-edit-gates.sh|||Math\\.random|||post-edit-gates red-lines insecure-random secrets"
  "$H/post-edit-gates.sh|||parseFloat|||post-edit-gates red-lines float money"
  "$H/post-edit-gates.sh|||customer_phone|||post-edit-gates red-lines raw PII"
  "$S/settings.json|||protect-paths.sh|||protect-paths.sh still wired"
  "$S/settings.json|||guard-bash.sh|||guard-bash.sh still wired"
  "$S/settings.json|||verify-safety-floor.sh|||floor invariant still wired"
  "$H/serious-gate.sh|||rls|||serious-gate still covers RLS"
  "$H/serious-gate.sh|||auth|||serious-gate still covers auth"
)

fails=()
for row in "${CHECKS[@]}"; do
  file="${row%%|||*}"; rest="${row#*|||}"; needle="${rest%%|||*}"; label="${rest#*|||}"
  # Missing file mid-edit → fail-open (don't alarm on a transient absence).
  [ -f "$file" ] || continue
  grep -qF -- "$needle" "$file" || fails+=("$label  [missing: '$needle' in ${file#"$ROOT"/}]")
done

if [ "${#fails[@]}" -gt 0 ]; then
  {
    echo "🛑 SAFETY-FLOOR BREACH — a self-mod removed product-red-line enforcement. ${#fails[@]} check(s) failed:"
    for f in "${fails[@]}"; do echo "   ✗ $f"; done
    echo "The floor may only be strengthened or scope-corrected, never removed. Revert the offending self-mod."
  } >&2
  mkdir -p "$ROOT/.claude/logs" 2>/dev/null || true
  printf '%s FLOOR-BREACH %d-checks-failed\n' "$(date -Iseconds 2>/dev/null || date)" "${#fails[@]}" \
    >> "$ROOT/.claude/logs/self-mod.log" 2>/dev/null || true
  exit 2
fi

# standalone success message (silent when run as a hook that passed)
[ -z "$INPUT" ] && echo "SAFETY-FLOOR intact — all ${#CHECKS[@]} checks pass."
exit 0
