#!/usr/bin/env bash
# serious-gate.sh — PreToolUse gate (Edit|Write|MultiEdit)
# Policy: code on a SERIOUS surface needs an APPROVED Triad-Council plan BEFORE the change.
# Friction, not verdict: a human bypasses via .claude/state/serious-override. Fail-open on errors.
# NOTE: installed as serious-gate.sh (NOT require-classification.sh) — that name is taken by the
# existing Stop/CHANGE-MANIFEST hook; this is an independent PreToolUse gate.
set -uo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"
STATE="$ROOT/.claude/state"
LOG="$ROOT/.claude/logs/classification.log"
mkdir -p "$STATE" "$(dirname "$LOG")" 2>/dev/null || true

INPUT="$(cat)"

# --- parse the file path (jq → python3); on failure → fail-open ---
fp=""
if command -v jq >/dev/null 2>&1; then
  fp="$(printf '%s' "$INPUT" | jq -r '.tool_input.file_path // .tool_input.path // empty' 2>/dev/null)"
elif command -v python3 >/dev/null 2>&1; then
  fp="$(printf '%s' "$INPUT" | python3 -c 'import sys,json;ti=json.load(sys.stdin).get("tool_input",{});print(ti.get("file_path") or ti.get("path") or "")' 2>/dev/null)"
fi
[ -z "$fp" ] && exit 0
rel="${fp#"$ROOT"/}"

# --- always pass: council/loop/config artifacts (else deadlock) ---
case "$rel" in
  docs/design/*|docs/adr/*|docs/governance/*|loops/*|.claude/*) exit 0 ;;
esac

# --- human bypass (friction, not verdict; human is final) ---
if [ -f "$STATE/serious-override" ]; then
  echo "$(date -Iseconds) ALLOW override $rel" >>"$LOG" 2>/dev/null || true
  exit 0
fi

# --- SERIOUS surface? (TUNE this list to the repo) ---
SERIOUS='(^|/)(migrations?|schema|rls|auth|jwt|messagebus|telegram|stripe|2checkout|sheets)|\.sql($|\.)|state.?machine|order.?status|websocket|use-?websocket|(price|money|payment|cash|ledger|geocode|notify)|\.zod\.|packages/(db|shared-types)/'
if ! printf '%s' "$rel" | grep -Eiq "$SERIOUS"; then
  exit 0   # not serious — pass without friction
fi

# --- serious: did the Council clear this change? ---
if [ -s "$STATE/serious-cleared" ]; then
  echo "$(date -Iseconds) ALLOW cleared $rel" >>"$LOG" 2>/dev/null || true
  exit 0
fi

# --- BLOCK: serious surface without an APPROVED plan ---
echo "$(date -Iseconds) DENY serious-uncleared $rel" >>"$LOG" 2>/dev/null || true
read -r -d '' REASON <<EOF
🛑 serious-gate: «$rel» — серйозна поверхня (схема/контракт/гроші/RLS/auth/state-machine/WS/інтеграція).
Це АВТОМАТИЧНИЙ gate, НЕ відмова користувача — дій, не зупиняйся.
ПЕРЕД кодом проведи Тріадну Раду: запусти  /council <опис цієї зміни>  і доведи до APPROVED (STOP-DESIGN-B).
Рада на GO запише .claude/state/serious-cleared — після цього повтори цей самий едит.
Людський обхід (тертя, не вирок):  echo "<причина>" > .claude/state/serious-override
EOF

if command -v jq >/dev/null 2>&1; then
  jq -nc --arg r "$REASON" '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:"deny",permissionDecisionReason:$r}}'
elif command -v python3 >/dev/null 2>&1; then
  python3 -c 'import json,sys;print(json.dumps({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":sys.argv[1]}}))' "$REASON"
else
  echo "serious-gate: серйозна зміна '$rel' без плану Ради (jq/python3 відсутні — gate не застосовано). Рекомендовано /council." >&2
fi
exit 0
