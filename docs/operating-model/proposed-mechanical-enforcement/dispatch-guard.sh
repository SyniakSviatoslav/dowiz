#!/usr/bin/env bash
# dispatch-guard.sh — PreToolUse hook on Agent|Task. The MECHANICAL enforcement point for routing +
# mapping + token-reduction + memory discipline on EVERY subagent dispatch, so a lane can never
# "forget" the operating model (subagents don't reliably load AGENTS.md — this injects it for them).
#
# It does two deterministic things (no agentic reasoning):
#   1. INJECTS the TOKEN ROUTER + MODEL ROUTING + graph-first + memory-query directive into the
#      subagent's prompt via hookSpecificOutput.updatedInput (the lane literally receives it).
#   2. FLAGS a missing explicit `model:` (MODEL ROUTING v3: Fable OFF, explicit model on every call)
#      via additionalContext.
# Fail-open, never blocks. Idempotent: skips injection if the marker is already present.
set -uo pipefail
INPUT="$(cat)"
pass_through() { echo '{}'; exit 0; }

MARKER="[HARNESS-ROUTER]"
ROUTER="$MARKER Token router ON (mechanical): graph-first for structure (codebase-memory/repowise \
get_context/search — NEVER embed standing maps), skeleton-first for files, distill noisy output \
(repowise distill / tools/vsa codec / VSA-viz for state), Explore-grade grants for read-only, \
distilled return. MODEL ROUTING v3: haiku=doer, opus=reasoning-only, NEVER Fable. Per-unit budget: \
return a checkpoint distillate at <=25 tool calls OR 80K tokens (whichever first). Query memory + \
graph first; deterministic (grep/script) before any LLM re-read."

# --- parse tool_name, prompt, model (jq primary, python3 fallback) ---
read_field() { # $1 = jq path ; prints value or empty
  if command -v jq >/dev/null 2>&1; then
    printf '%s' "$INPUT" | jq -r "$1 // \"\"" 2>/dev/null
  else
    printf '%s' "$INPUT" | python3 -c "import sys,json,functools;d=json.load(sys.stdin)
p='$1'.replace('.tool_input','tool_input').lstrip('.').split('.')
cur=d
for k in p:
    cur=(cur or {}).get(k) if isinstance(cur,dict) else None
print(cur or '')" 2>/dev/null
  fi
}
TOOL="$(read_field '.tool_name')"
case "$TOOL" in Agent|Task) ;; *) pass_through;; esac

PROMPT="$(read_field '.tool_input.prompt')"
MODEL="$(read_field '.tool_input.model')"
[ -n "$PROMPT" ] || pass_through
case "$PROMPT" in *"$MARKER"*) INJECT=0;; *) INJECT=1;; esac

NOTE=""
[ -z "$MODEL" ] && NOTE="MODEL ROUTING v3 violation: this ${TOOL} call has NO explicit model: — set one (haiku doer / opus reasoning / never Fable)."

if [ "$INJECT" -eq 0 ] && [ -z "$NOTE" ]; then pass_through; fi

NEWPROMPT="$PROMPT"
[ "$INJECT" -eq 1 ] && NEWPROMPT="$PROMPT

$ROUTER"

# emit updatedInput (full tool_input with patched prompt) + optional additionalContext
if command -v jq >/dev/null 2>&1; then
  printf '%s' "$INPUT" | jq -c --arg np "$NEWPROMPT" --arg note "$NOTE" \
    '{hookSpecificOutput: ({hookEventName:"PreToolUse", updatedInput: (.tool_input + {prompt:$np})}
      + (if $note=="" then {} else {additionalContext:$note} end))}'
else
  printf '%s' "$INPUT" | python3 -c '
import sys,json
d=json.load(sys.stdin); np=sys.argv[1]; note=sys.argv[2]
ti=dict(d.get("tool_input") or {}); ti["prompt"]=np
out={"hookEventName":"PreToolUse","updatedInput":ti}
if note: out["additionalContext"]=note
print(json.dumps({"hookSpecificOutput":out}))' "$NEWPROMPT" "$NOTE"
fi
exit 0
