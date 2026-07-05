#!/usr/bin/env bash
# eye-guard.sh — the parallel "EYE": a cheap, deterministic real-time monitor (PostToolUse).
#
# It tallies BAD and CRITICAL signals per session and HALTS the agent for inspection ONLY when the
# threshold is reached: >= EYE_BAD_MAX (default 3) bad signals, OR >= 1 critical. Below that it is
# SILENT — the eye is FORBIDDEN to slow or pause the agent without >=3 bad or >=1 critical (operator
# directive). No LLM, no reasoning — pure signal counting, so it costs almost nothing.
#
# Signal classification from the tool_response of the event it observes:
#   CRITICAL — a hard block or red-line trip: "RED-LINE" (circuit/doubt), "BLOCKED" (protect-paths /
#              guard-bash). One is enough to stop.
#   BAD      — a plain tool failure: "is_error":true or "success":false. Three within a session stop.
# Aggregates naturally with the other guards (they surface these strings); extend by having circuit-
# guard/loop-detector append to the same tally if you want more sources.
set -uo pipefail
BAD_MAX="${EYE_BAD_MAX:-3}"
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"
INPUT="$(cat)"
SID="$(printf '%s' "$INPUT" | (jq -r '.session_id // "s"' 2>/dev/null \
  || python3 -c 'import sys,json;print(json.load(sys.stdin).get("session_id","s"))' 2>/dev/null))"
[ -n "$SID" ] || SID=s
DIR="$ROOT/.claude/state/eye"; mkdir -p "$DIR" 2>/dev/null || true
TALLY="$DIR/$SID.tally"
[ -f "$TALLY" ] || printf '0 0\n' > "$TALLY"

RESP="$(printf '%s' "$INPUT" | (jq -c '.tool_response // {}' 2>/dev/null \
  || python3 -c 'import sys,json;print(json.dumps(json.load(sys.stdin).get("tool_response",{})))' 2>/dev/null))"
NS="${RESP// /}"   # strip spaces so jq (":true") and python (": true") forms both match
sev=""
case "$NS" in
  *RED-LINE*|*BLOCKED*) sev="critical";;
  *'"is_error":true'*|*'"success":false'*) sev="bad";;
esac
[ -z "$sev" ] && { echo '{}'; exit 0; }

read -r bad crit < "$TALLY" 2>/dev/null || { bad=0; crit=0; }
case "$sev" in critical) crit=$((crit + 1));; bad) bad=$((bad + 1));; esac
printf '%s %s\n' "$bad" "$crit" > "$TALLY"

if [ "$crit" -ge 1 ] || [ "$bad" -ge "$BAD_MAX" ]; then
  printf '0 0\n' > "$TALLY"   # reset after firing so it doesn't spam
  R="EYE STOP — $bad bad + $crit critical signal(s) hit the threshold (>=${BAD_MAX} bad or >=1 critical). HALT and inspect the work: re-read the failing surface, confirm no pattern/rule is broken, before continuing."
  if command -v jq >/dev/null 2>&1; then
    jq -cn --arg r "$R" '{continue:false, stopReason:$r, systemMessage:$r}'
  else
    python3 -c 'import sys,json;print(json.dumps({"continue":False,"stopReason":sys.argv[1],"systemMessage":sys.argv[1]}))' "$R"
  fi
else
  echo '{}'   # below threshold — SILENT (forbidden to interrupt)
fi
exit 0
