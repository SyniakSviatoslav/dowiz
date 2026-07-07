#!/usr/bin/env bash
# agent-dispatch-gate.sh — PreToolUse gate on Agent/Task dispatch: MODEL ROUTING enforcement.
#
# STRUCTURE-UPGRADE.md Part B, step B1 — the ONE dispatch gate. Operator picked rollout mode (A)
# warn-then-ratchet (2026-07-06): B0 measured that ~86% of 1027 real dispatches carry NO explicit
# model: (only ~10% compliant). A blind hard-DENY would block ~90% of the live dispatch pattern
# (incl. legit Explore read-only lanes + the triad council) — the #47 over-block failure mode.
# So this ships in WARN mode: it LOGS the violation to harness-events.jsonl (_hev) and surfaces a
# non-blocking nudge, but never blocks. The DENY path is fully armed (TOKEN_GATE_MODE=deny, proven
# by scripts/guardrail-token-gates.mjs) so promoting a check to teeth is a one-line config flip
# once the _hev data shows the model: habit took.
#
# CHECKS (this increment — ONE check at a time per operator directive):
#   1. Agent/Task dispatch with NO explicit model:  (MODEL ROUTING: "explicit model: on every
#      Agent call"). WARN now; ratchet to DENY when compliance is habitual in the _hev log.
#   (fable-without-override + LANE-CLASS/router stamps are later increments — kept out to avoid
#    nudge-spam and cargo-culting while the model: habit is still forming; see STRUCTURE-UPGRADE B1.)
#
# HONESTY RULES (STRUCTURE-UPGRADE Part B): deterministic/regex only, no LLM; one _hev line per
# decision; fail OPEN + log `degraded` only when JSON is unparseable; warn output is non-blocking
# (stderr, exit 0) so the dispatch proceeds; the deny decision goes to stdout as the standard
# PreToolUse permissionDecision JSON (the convention scripts/guardrail-gate-armament.mjs asserts).
set -uo pipefail

INPUT=$(cat)
[ -z "$INPUT" ] && exit 0

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"

HEV_LOG="$ROOT/.claude/logs/harness-events.jsonl"
_hev() {
  mkdir -p "$(dirname "$HEV_LOG")" 2>/dev/null || true
  printf '{"ts":"%s","hook":"%s","event":"%s","target":"%s","detail":"%s"}\n' \
    "$(date -Iseconds)" "$1" "$2" \
    "$(printf '%s' "${3:-}" | tr '"\\' '..' | tr '\n' ' ' | cut -c1-200)" \
    "$(printf '%s' "${4:-}" | tr '"\\' '..' | tr '\n' ' ' | cut -c1-200)" \
    >>"$HEV_LOG" 2>/dev/null || true
}

# Parse → tool_name, model, slug on THREE separate lines (jq → python3 → node fallback, matching
# guard-bash). Newline-separated, not tab: a tab is IFS-whitespace, so an EMPTY model field
# (`\t\t`) collapses and `read` mis-assigns the slug to MODEL — the missing-model check would then
# never fire. Per-line reads preserve empty fields.
_parse() {
  if command -v jq &>/dev/null; then
    printf '%s' "$INPUT" | jq -r '[(.tool_name // ""), (.tool_input.model // ""), (.tool_input.description // .tool_input.subagent_type // "")] | .[]' 2>/dev/null
  elif command -v python3 &>/dev/null; then
    printf '%s' "$INPUT" | python3 -c '
import sys, json
try:
    d = json.loads(sys.stdin.read()); ti = d.get("tool_input", {}) or {}
    print("\n".join([str(d.get("tool_name") or ""), str(ti.get("model") or ""),
                     str(ti.get("description") or ti.get("subagent_type") or "")]))
except Exception:
    pass
' 2>/dev/null || true
  elif command -v node &>/dev/null; then
    printf '%s' "$INPUT" | node -e '
let d="";process.stdin.on("data",c=>d+=c);process.stdin.on("end",()=>{try{const o=JSON.parse(d);const ti=o.tool_input||{};process.stdout.write([o.tool_name||"",ti.model||"",ti.description||ti.subagent_type||""].join("\n"));}catch(e){}});' 2>/dev/null || true
  fi
}

TOOL_NAME=""; MODEL=""; SLUG=""
{ IFS= read -r TOOL_NAME; IFS= read -r MODEL; IFS= read -r SLUG; } < <(_parse) || true

# Fail OPEN + log degraded only when the JSON is unparseable (infra failure, never a silent block).
if [ -z "$TOOL_NAME" ]; then
  _hev agent-dispatch-gate degraded "" "unparseable tool_input; failing open"
  exit 0
fi

# Act ONLY on actual dispatch tools (exact match) — the matcher may be broad, but TaskCreate/
# TaskUpdate/TaskOutput/etc. are the task-LIST tools, not dispatches, and must never trigger.
case "$TOOL_NAME" in
  Agent|Task) : ;;
  *) exit 0 ;;
esac

MODE="${TOKEN_GATE_MODE:-warn}"        # model-absent check: warn (86% today); ratchet to deny on _hev habit.
FABLE_MODE="${TOKEN_FABLE_MODE:-deny}" # fable check: RE-ARMED to DENY (2026-07-07). The sanctioned
                                       # one-shot Fable audit is CONSUMED, so the standing MODEL ROUTING
                                       # rule ("Fable OFF for lanes") is restored. A human may still grant
                                       # a sanctioned exception via a non-expired .claude/state/fable-override
                                       # line; set TOKEN_FABLE_MODE=warn for a temporary escape hatch.
SLUG_SHORT="$(printf '%s' "$SLUG" | cut -c1-80)"

_deny() {
  _hev agent-dispatch-gate deny "$SLUG_SHORT" "$1"
  printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"%s"}}\n' \
    "$(printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g')"
  exit 0
}
_warn() {
  _hev agent-dispatch-gate warn "$SLUG_SHORT" "$1"
  printf '⚠ agent-dispatch-gate: %s\n' "$1" >&2   # stderr + exit 0 = visible but NON-blocking
}

# ── Check 1: explicit model: required ────────────────────────────────────────
MODEL_LC="$(printf '%s' "$MODEL" | tr '[:upper:]' '[:lower:]' | tr -d '[:space:]')"
if [ -z "$MODEL_LC" ]; then
  MSG="MODEL ROUTING: Agent dispatch '${SLUG_SHORT}' has no explicit model: — add one (haiku doer / opus reasoning-only; never Fable for lanes). AGENTS.md MODEL ROUTING."
  if [ "$MODE" = "deny" ]; then _deny "$MSG"; else _warn "$MSG [warn-only; ratcheting to deny once habitual]"; fi
fi

# ── Check 2: no Fable for dispatch lanes ─────────────────────────────────────
# Operator 2026-07-06: "use cheaper models instead of fable". Human-only EXPIRING override:
# .claude/state/fable-override, one line per grant `<slug>|<unix-expiry>`; a single non-expired
# line authorizes the arc (D4-style). Expiry is embedded in the line and compared to wall-clock
# HERE, fail-closed — never a cleanup step (lesson 2026-07-02-gate-state-file-expiry #47). guard-bash
# OVERRIDES + protect-paths keep the file human-only, so an agent cannot write its own bypass.
if printf '%s' "$MODEL_LC" | grep -q 'fable'; then
  OVERRIDE_FILE="$ROOT/.claude/state/fable-override"
  now="$(date +%s)"
  active=0
  if [ -f "$OVERRIDE_FILE" ]; then
    while IFS='|' read -r _oslug oexp _rest || [ -n "$_oslug" ]; do
      case "$oexp" in ''|*[!0-9]*) continue ;; esac   # malformed/empty expiry → ignore (fail-closed)
      if [ "$oexp" -gt "$now" ]; then active=1; break; fi
    done < "$OVERRIDE_FILE"
  fi
  if [ "$active" -eq 0 ]; then
    MSG="MODEL ROUTING: Fable is OFF for dispatch lanes (operator 2026-07-06: use cheaper models — haiku doer / sonnet / opus reasoning). Re-dispatch '${SLUG_SHORT}' with a cheaper model; a human may add a non-expired .claude/state/fable-override line for a sanctioned exception."
    if [ "$FABLE_MODE" = "warn" ]; then _warn "$MSG [warn-only]"; else _deny "$MSG"; fi
  fi
fi

exit 0
