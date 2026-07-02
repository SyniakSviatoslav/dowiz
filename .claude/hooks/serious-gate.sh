#!/usr/bin/env bash
# serious-gate.sh — PreToolUse gate (Edit|Write|MultiEdit)
# Policy: code on a SERIOUS surface needs an APPROVED Triad-Council plan BEFORE the change.
# Friction, not verdict: a human bypasses via .claude/state/serious-override. Fail-open on errors.
# NOTE: installed as serious-gate.sh (NOT require-classification.sh) — that name is taken by the
# existing Stop/CHANGE-MANIFEST hook; this is an independent PreToolUse gate.
#
# 2026-07-02 (P0 gate-rearm, ledger): clearance is now PER-LINE `slug|expiry-epoch`. The old
# `[ -s serious-cleared ]` check meant ANY accumulated non-empty file cleared ALL serious surfaces
# forever — the gate was de-facto open from 06-21 to 07-02 (7 stale slugs, 400+ blind ALLOWs).
# Legacy bare-slug lines (no expiry) are treated as EXPIRED. Council GO appends `slug|now+72h`.
set -uo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"
STATE="$ROOT/.claude/state"
LOG="$ROOT/.claude/logs/classification.log"
mkdir -p "$STATE" "$(dirname "$LOG")" 2>/dev/null || true


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
# docs/* precision exemption (P1, post-edit-gates 06-29 precedent): markdown documents a
# serious surface, it IS never one — ledger/ADR/reflection appends must not need clearance.
case "$rel" in
  docs/*|loops/*|.claude/*) exit 0 ;;
esac

# --- human bypass (friction, not verdict; human is final) ---
if [ -f "$STATE/serious-override" ]; then
  echo "$(date -Iseconds) ALLOW override $rel" >>"$LOG" 2>/dev/null || true
  _hev serious-gate allow "$rel" override
  exit 0
fi

# --- SERIOUS surface? (TUNE this list to the repo) ---
SERIOUS='(^|/)(migrations?|schema|rls|auth|jwt|messagebus|telegram|stripe|2checkout|sheets)|\.sql($|\.)|state.?machine|order.?status|websocket|use-?websocket|(price|money|payment|cash|ledger|geocode|notify)|\.zod\.|packages/(db|shared-types)/'
if ! printf '%s' "$rel" | grep -Eiq "$SERIOUS"; then
  exit 0   # not serious — pass without friction
fi

# --- serious: did the Council clear this change? ---
# Per-line format `slug|expiry-epoch`; a line clears the gate only while expiry > now.
# Lines without a numeric expiry (legacy bare slugs) NEVER clear — accumulation must not re-open the gate.
now="$(date +%s)"
cleared_slug=""
if [ -f "$STATE/serious-cleared" ]; then
  while IFS='|' read -r slug exp _rest; do
    case "$exp" in ''|*[!0-9]*) continue ;; esac
    if [ "$exp" -gt "$now" ]; then cleared_slug="${slug:-unnamed}"; break; fi
  done <"$STATE/serious-cleared"
fi
if [ -n "$cleared_slug" ]; then
  echo "$(date -Iseconds) ALLOW cleared($cleared_slug) $rel" >>"$LOG" 2>/dev/null || true
  _hev serious-gate allow "$rel" "cleared($cleared_slug)"
  exit 0
fi

# --- BLOCK: serious surface without an APPROVED (unexpired) plan ---
echo "$(date -Iseconds) DENY serious-uncleared $rel" >>"$LOG" 2>/dev/null || true
_hev serious-gate deny "$rel" serious-uncleared
read -r -d '' REASON <<EOF
🛑 serious-gate: «$rel» — серйозна поверхня (схема/контракт/гроші/RLS/auth/state-machine/WS/інтеграція).
Це АВТОМАТИЧНИЙ gate, НЕ відмова користувача — дій, не зупиняйся.
ПЕРЕД кодом проведи Тріадну Раду: запусти  /council <опис цієї зміни>  і доведи до APPROVED (STOP-DESIGN-B).
Рада на GO запише рядок "<slug>|<expiry-epoch>" у .claude/state/serious-cleared (діє 72h, потім gate
озброюється сам) — після цього повтори цей самий едит. Застарілі/без-expiry рядки НЕ відкривають gate.
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
