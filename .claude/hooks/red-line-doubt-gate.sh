#!/usr/bin/env bash
# red-line-doubt-gate.sh — PreToolUse gate on Edit|Write|MultiEdit (Doubt model: red-line doubt-pass)
# On a RED-LINE glob (auth / money / RLS / packages/db/migrations/ / bulk) it emits a REQUIRED
# "doubt-pass" reminder (list options considered · why this · reversibility). For the genuinely
# IRREVERSIBLE set it requires explicit human confirmation (a state file). ZERO-noise on routine
# reversible edits — only fires on red-line globs.
#
# COMPLEMENTARY to protect-paths.sh: that hook hard-BLOCKS (exit 2) its protected zones. This gate
# is ADVISORY over the broader red-line set — it uses exit 0 + additionalContext and NEVER weakens
# or duplicates protect-paths' block. Only the truly-irreversible set gates (deny) here, and even
# then a human override file releases it. Fail-open on any parse/IO error.
#
# 2026-07-02 (P0 gate-rearm): the redline-confirmed release now EXPIRES — the file must be younger
# than 60 minutes. The previous `[ -f ... ]` check let a single 2026-06-23 confirmation hold the
# irreversible-migration gate open for 9 days. One confirmation = one work window, not forever.
set -uo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"
STATE="$ROOT/.claude/state"
mkdir -p "$STATE" 2>/dev/null || true


# P1 telemetry (meta-loop 2026-07-02): one JSONL line per decision — the advisory layer was
# unmeasurable (no lesson hit-counts, no deny/allow rates), so pruning/promotion ran blind.
# Advisory: never fails the hook.
HEV_LOG="$ROOT/.claude/logs/harness-events.jsonl"
_hev() {
  mkdir -p "$(dirname "$HEV_LOG")" 2>/dev/null || true
  printf '{"ts":"%s","hook":"%s","event":"%s","target":"%s","detail":"%s"}\n' \
    "$(date -Iseconds)" "$1" "$2" \
    "$(printf '%s' "${3:-}" | tr '"\\' '..' | tr '\n' ' ' | cut -c1-200)" \
    "$(printf '%s' "${4:-}" | tr '"\\' '..' | tr '\n' ' ' | cut -c1-200)" \
    >>"$HEV_LOG" 2>/dev/null || true
}

INPUT="$(cat)"

# --- extract file_path (jq → python3 → python → node) ---
fp=""
if command -v jq >/dev/null 2>&1; then
  fp="$(printf '%s' "$INPUT" | jq -r '.tool_input.file_path // .tool_input.path // empty' 2>/dev/null)"
elif command -v python3 >/dev/null 2>&1; then
  fp="$(printf '%s' "$INPUT" | python3 -c 'import sys,json;ti=json.load(sys.stdin).get("tool_input",{});print(ti.get("file_path") or ti.get("path") or "")' 2>/dev/null)"
elif command -v python >/dev/null 2>&1; then
  fp="$(printf '%s' "$INPUT" | python -c 'import sys,json;ti=json.load(sys.stdin).get("tool_input",{});print(ti.get("file_path") or ti.get("path") or "")' 2>/dev/null)"
elif command -v node >/dev/null 2>&1; then
  fp="$(printf '%s' "$INPUT" | node -e 'let s="";process.stdin.on("data",c=>s+=c);process.stdin.on("end",()=>{try{const ti=(JSON.parse(s).tool_input)||{};process.stdout.write(ti.file_path||ti.path||"");}catch(e){}});' 2>/dev/null)"
fi
[ -z "$fp" ] && exit 0
case "$fp" in
  "$ROOT"/*) REL="${fp#"$ROOT"/}" ;;
  *) REL="$fp" ;;
esac

# --- RED-LINE globs (advisory doubt-pass set) — auth / money / RLS / migrations / bulk surfaces ---
REDLINE='(^|/)(auth|jwt|otp|session|refresh.?token|login)|rls|policy\.sql|(^|/)(price|money|payment|cash|ledger|tax|payout|refund|invoice)|2checkout|stripe|packages/db/migrations/|\.zod\.|(^|/)(messagebus|webhook|idempoten)'

# --- IRREVERSIBLE set (gate → human confirm) — destructive / non-rollbackable surfaces ---
IRREVERSIBLE='packages/db/migrations/.*\.(sql|js|ts)$'

is_redline=0
printf '%s' "$REL" | grep -Eiq "$REDLINE" && is_redline=1
# routine reversible edit → ZERO output, pass clean
[ "$is_redline" -eq 0 ] && exit 0

is_irrev=0
printf '%s' "$REL" | grep -Eiq "$IRREVERSIBLE" && is_irrev=1

# ============ IRREVERSIBLE: require explicit human confirmation (friction, not a wall) ============
if [ "$is_irrev" -eq 1 ]; then
  # Confirmation is a WINDOW, not a switch: file must exist AND be <60 min old.
  if [ -n "$(find "$STATE/redline-confirmed" -mmin -60 2>/dev/null)" ]; then
    _hev red-line-gate allow "$REL" confirmed-window
    exit 0   # human cleared it within the last hour
  fi
  _hev red-line-gate deny "$REL" irreversible-unconfirmed
  REASON="🛑 RED-LINE / IRREVERSIBLE: «$REL» is a migration — forward-only, not rollbackable on prod.
Doubt model requires a HUMAN doubt-pass before this edit. Provide, then confirm:
  • options considered (alternatives to this migration / can it be additive-only?)
  • why THIS one (evidence: invariant / ADR / failing test)
  • reversibility plan (expand→contract? backfill? is there ANY undo?)
Human releases the gate for a 60-minute window (friction, not a verdict):
  echo \"<reason>\" > .claude/state/redline-confirmed
An older confirmation does NOT count — the gate re-arms itself after 60 minutes.
NOTE: this is ADVISORY-gated and complementary to protect-paths.sh (which already hard-blocks migrations/). It does not replace that block."
  if command -v jq >/dev/null 2>&1; then
    jq -nc --arg r "$REASON" '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:"deny",permissionDecisionReason:$r}}'
  elif command -v python3 >/dev/null 2>&1; then
    python3 -c 'import json,sys;print(json.dumps({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":sys.argv[1]}}))' "$REASON"
  else
    printf '%s\n' "$REASON" >&2
  fi
  exit 0
fi

# ============ RED-LINE (reversible): advisory doubt-pass reminder, never blocks ============
_hev red-line-gate advise "$REL" doubt-pass
CTX="🔶 RED-LINE doubt-pass required for «$REL» (auth/money/RLS/integration surface).
Before this edit, state your doubt-pass (skill: doubt-escalation):
  1) OPTIONS considered — the 2–3 approaches you weighed;
  2) WHY THIS one — evidence (file:line / ADR / invariant / test) that it dominates;
  3) REVERSIBILITY — how this is undone if wrong (revert / flag-off / no data loss).
If a security/invariant doubt survives → raise security-sentinel or invariant-guardian for a file:line check, then resume.
Advisory only (this gate does not block; protect-paths.sh enforces hard zones separately)."
if command -v jq >/dev/null 2>&1; then
  jq -nc --arg c "$CTX" '{hookSpecificOutput:{hookEventName:"PreToolUse",additionalContext:$c}}'
elif command -v python3 >/dev/null 2>&1; then
  python3 -c 'import json,sys;print(json.dumps({"hookSpecificOutput":{"hookEventName":"PreToolUse","additionalContext":sys.argv[1]}}))' "$CTX"
else
  printf '%s\n' "$CTX" >&2
fi
exit 0
