#!/usr/bin/env bash
# token-circuit-guard.sh — PostToolUse + SubagentStop hook. The mechanical anti-context-rot circuit
# for the AGENTS.md HARD TOKEN THRESHOLDS (operator directive 2026-07-05).
#
# Reads the CURRENT agentic unit's cumulative token usage from its transcript and emits a
# deterministic recycle directive when it crosses a threshold:
#   - sub-unit (subagent / loop / workflow worker) ≥ 80K  → return checkpoint distillate, recycle.
#   - main lead session                             ≥ 300K → save+push, short summary, /clean, fresh.
#
# Metric = cumulative NEW tokens = Σ over assistant turns of (input_tokens + cache_creation +
# output_tokens). cache_read is EXCLUDED on purpose — counting the re-read prefix every turn would
# trip 300K in ~3 turns; "new tokens" is the honest measure of work done and grows at a sane rate.
#
# Philosophy (matches loop-detector.sh): fail-OPEN on any parse/IO error, never crash the tool, emit
# high-friction guidance via systemMessage + additionalContext. A hook cannot hard-kill a running
# subagent in this harness, so this is a DETERMINISTIC signal, not a SIGKILL — paired with the
# per-unit budget carried in every dispatch brief (AGENTS.md), that is the enforcement.
set -uo pipefail

LANE_MAX="${TOKEN_LANE_MAX:-80000}"
SESSION_MAX="${TOKEN_SESSION_MAX:-300000}"
INPUT="$(cat)"

pass_through() { echo '{}'; exit 0; }

# --- transcript path (the hook stdin carries it) ---
tp() {
  if command -v jq >/dev/null 2>&1; then
    printf '%s' "$INPUT" | jq -r '.transcript_path // ""' 2>/dev/null
  else
    printf '%s' "$INPUT" | python3 -c 'import sys,json;print(json.load(sys.stdin).get("transcript_path",""))' 2>/dev/null
  fi
}
TP="$(tp)"
[ -n "$TP" ] && [ -f "$TP" ] || pass_through

# --- cumulative NEW tokens across the whole transcript ---
sum_tokens() {
  if command -v jq >/dev/null 2>&1; then
    jq -s 'map(.message.usage // .usage // empty)
           | map((.input_tokens // 0) + (.cache_creation_input_tokens // 0) + (.output_tokens // 0))
           | add // 0' "$TP" 2>/dev/null
  elif command -v python3 >/dev/null 2>&1; then
    python3 - "$TP" <<'PY' 2>/dev/null
import sys,json
total=0
for line in open(sys.argv[1],encoding="utf-8",errors="ignore"):
    line=line.strip()
    if not line: continue
    try: o=json.loads(line)
    except Exception: continue
    u=(o.get("message") or {}).get("usage") or o.get("usage") or {}
    total+=int(u.get("input_tokens",0))+int(u.get("cache_creation_input_tokens",0))+int(u.get("output_tokens",0))
print(total)
PY
  fi
}
USED="$(sum_tokens)"
case "$USED" in ''|*[!0-9]*) pass_through;; esac

# --- main session vs sub-unit (path heuristic; the main transcript is the top-level session file) ---
IS_SUB=0
case "$TP" in *subagent*|*/agents/*|*agent-*|*task-*|*subagents*) IS_SUB=1;; esac

emit() {  # $1 systemMessage, $2 additionalContext (both JSON-string-escaped by jq/python)
  local sm="$1" ac="$2"
  if command -v jq >/dev/null 2>&1; then
    jq -cn --arg sm "$sm" --arg ac "$ac" \
      '{systemMessage:$sm, hookSpecificOutput:{hookEventName:"PostToolUse", additionalContext:$ac}}'
  else
    python3 - "$sm" "$ac" <<'PY'
import sys,json
print(json.dumps({"systemMessage":sys.argv[1],"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":sys.argv[2]}}))
PY
  fi
}

if [ "$IS_SUB" -eq 0 ] && [ "$USED" -ge "$SESSION_MAX" ]; then
  emit "⛔ TOKEN CIRCUIT: session ~${USED} tok ≥ ${SESSION_MAX}. SAVE everything (h_t frame + memory + ledger), commit+PUSH to remote, write a SHORT session summary, then run /clean and resume fresh." \
       "AGENTS.md HARD TOKEN THRESHOLDS: do not keep accumulating. Checkpoint → push → /clean → fresh session now."
elif [ "$IS_SUB" -eq 1 ] && [ "$USED" -ge "$LANE_MAX" ]; then
  emit "🔁 TOKEN CIRCUIT: this agentic unit ~${USED} tok ≥ ${LANE_MAX}. STOP and return your checkpoint distillate {state, done, remaining, pointers}; a fresh unit will finish the task." \
       "AGENTS.md HARD TOKEN THRESHOLDS: no agentic unit grinds past ${LANE_MAX} tokens (quadratic prefix re-read). Return the distillate and end."
else
  pass_through
fi
exit 0
