#!/usr/bin/env bash
# route-request.sh — UserPromptSubmit router (nudge, NEVER blocks)
# Serious → /council; recurring + DoD → /loop-orchestrator (4-condition test).
set -uo pipefail

INPUT="$(cat)"
prompt=""
if command -v jq >/dev/null 2>&1; then
  prompt="$(printf '%s' "$INPUT" | jq -r '.prompt // empty' 2>/dev/null)"
elif command -v python3 >/dev/null 2>&1; then
  prompt="$(printf '%s' "$INPUT" | python3 -c 'import sys,json;print(json.load(sys.stdin).get("prompt",""))' 2>/dev/null)"
fi
[ -z "$prompt" ] && exit 0
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"

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


# don't nudge if a system command was already invoked, or if this "prompt" is not a human
# ask at all but an auto-relayed system artifact (background-task completion notice, etc.).
# MEASURED 2026-07-06 (.claude/logs/harness-events.jsonl, 529 route-request fires / 4.2d):
# 481/529 (91%) — 240/270 nudge-serious + 241/259 nudge-repeat — were `<task-notification>`
# payloads, not user requests. A task-notification reports that a background tool call
# already finished; nudging "run /council before code" at that point is a non-sequitur —
# the real enforcement for whatever that background call did lives in the PreToolUse gates
# (serious-gate.sh / red-line-doubt-gate.sh / protect-paths.sh), which already ran at the
# actual tool-call and are untouched by this exclusion.
case "$prompt" in
  /council*|/loop-orchestrator*|/loop*|/build-verify-loop*|/converge-loop*) exit 0 ;;
  *'<task-notification>'*) exit 0 ;;
esac

low="$(printf '%s' "$prompt" | tr '[:upper:]' '[:lower:]')"
ctx=""

# NOTE: "рефактор|refactor", "нова.?фіч|new.?feature" and bare "видали" (Ukr. "delete") were
# removed from SERIOUS 2026-07-06 — the nudge text itself says to ignore exactly this case
# ("дрібне (косметика/локальний рефактор без контракт-впливу) — ігноруй цей нудж"), so firing
# on the bare words contradicted the hook's own stated policy. Genuinely destructive deletes
# remain caught via `drop.?table` / `незворотн` / `деструктив`; a refactor or new feature that
# ALSO touches schema/auth/money/RLS/etc. still fires via those separate, unchanged keywords.
SERIOUS='міграц|migrat|schema|схем|контракт|contract|zod|гро[шж]|pric|цін|payment|оплат|cash|готівк|ledger|rls|auth|jwt|tenant|webhook|idempoten|state.?machine|стан.?замовлен|order.?status|websocket|realtime|реалтайм|geocode|notify|сповіщен|telegram|stripe|2checkout|sheets|drop.?table|незворотн|деструктив'
REPEAT='щораз|кожн|повторюва|repeat|регулярн|періодичн|автоматизу|automate|\bloop\b|цикл|every.?(day|time|week|hour|min|[0-9])|each.?(day|time)|nightly|weekly|hourly|daily|cron|schedul|recurring|always.?do|щодня'

if printf '%s' "$low" | grep -Eq "$SERIOUS"; then
  _hev route-request nudge-serious "$(printf '%s' "$low" | cut -c1-80)"
  ctx="⚠️ serious-gate router: запит схожий на СЕРЙОЗНУ зміну (схема/контракт/гроші/RLS/auth/state-machine/WS/інтеграція/незворотне). Політика: спершу проведи Тріадну Раду — /council <опис>, доведи до APPROVED, і лише тоді код. Дрібне (косметика/локальний рефактор без контракт-впливу) — ігноруй цей нудж."
fi
if printf '%s' "$low" | grep -Eq "$REPEAT"; then
  _hev route-request nudge-repeat "$(printf '%s' "$low" | cut -c1-80)"
  [ -n "$ctx" ] && ctx="$ctx
"
  ctx="${ctx}🔁 Схоже на ПОВТОРЮВАНУ задачу. Прожени через /loop-orchestrator (4-умовний тест: повторюється? DoD+верифікація? бюджет? навички?). Усі «так» — це петля, не разовий промпт."
fi

[ -z "$ctx" ] && exit 0

if command -v jq >/dev/null 2>&1; then
  jq -nc --arg c "$ctx" '{hookSpecificOutput:{hookEventName:"UserPromptSubmit",additionalContext:$c}}'
elif command -v python3 >/dev/null 2>&1; then
  python3 -c 'import json,sys;print(json.dumps({"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":sys.argv[1]}}))' "$ctx"
else
  printf '%s\n' "$ctx"
fi
exit 0
