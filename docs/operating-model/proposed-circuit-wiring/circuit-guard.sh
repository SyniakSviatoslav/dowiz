#!/usr/bin/env bash
# circuit-guard.sh — PostToolUse hook (Edit|Write|MultiEdit). Runs the KNOWLEDGE-AS-CIRCUITS registry
# against the just-edited file the instant it lands, so a reintroduced error-pattern / broken core
# rule emits its signal automatically (no reliance on the agent noticing). Fail-open; friction via
# systemMessage (the pre-commit `run-circuits.mjs --staged` step is the hard block).
set -uo pipefail
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"
INPUT="$(cat)"

FP="$(printf '%s' "$INPUT" | (jq -r '.tool_input.file_path // .tool_response.filePath // ""' 2>/dev/null \
  || python3 -c 'import sys,json;d=json.load(sys.stdin);print((d.get("tool_input") or {}).get("file_path","") or (d.get("tool_response") or {}).get("filePath",""))' 2>/dev/null))"
[ -n "$FP" ] || { echo '{}'; exit 0; }
# repo-relative
REL="${FP#$ROOT/}"

OUT="$(cd "$ROOT" && node scripts/run-circuits.mjs "$REL" 2>&1)"; RC=$?
if [ "$RC" -eq 0 ]; then echo '{}'; exit 0; fi

MSG="$(printf '%s' "$OUT" | grep -E 'RED-LINE|warn' | head -4)"
if command -v jq >/dev/null 2>&1; then
  jq -cn --arg m "🔌 CIRCUIT TRIPPED on $REL:
$MSG
Fix the code; do not weaken the circuit (docs/operating-model/circuits/registry.json)." \
    '{systemMessage:$m, hookSpecificOutput:{hookEventName:"PostToolUse", additionalContext:$m}}'
else
  python3 -c 'import sys,json;m=sys.argv[1];print(json.dumps({"systemMessage":m,"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":m}}))' \
    "CIRCUIT TRIPPED on $REL: $MSG — fix the code, do not weaken the circuit."
fi
exit 0
