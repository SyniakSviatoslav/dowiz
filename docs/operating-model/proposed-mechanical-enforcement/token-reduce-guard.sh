#!/usr/bin/env bash
# token-reduce-guard.sh — PreToolUse hook on Bash. MECHANICALLY caps the biggest token sink: verbose
# command output. Deterministic pattern match (no agentic reasoning). For a SAFE whitelist of noisy
# read-only commands that lack any output control, it rewrites the command via updatedInput to append
# `2>&1 | tail -N` (the exact pattern an operator uses by hand). Anything with an existing pipe /
# redirect / head|tail is left untouched. Fail-open, never blocks.
set -uo pipefail
INPUT="$(cat)"
TAIL_N="${TOKEN_REDUCE_TAIL:-60}"
pass_through() { echo '{}'; exit 0; }

get() {
  if command -v jq >/dev/null 2>&1; then printf '%s' "$INPUT" | jq -r "$1 // \"\"" 2>/dev/null
  else printf '%s' "$INPUT" | python3 -c "import sys,json;d=json.load(sys.stdin)
print((d.get('tool_input') or {}).get('command','') if '$1'=='.tool_input.command' else d.get('tool_name',''))" 2>/dev/null; fi
}
[ "$(get '.tool_name')" = "Bash" ] || pass_through
CMD="$(get '.tool_input.command')"
[ -n "$CMD" ] || pass_through

# Already output-controlled? leave it alone (safety: never double-pipe / change a redirect's meaning).
case "$CMD" in *'|'*|*'>'*|*'tail '*|*'head '*|*'wc '*|*' -q'*|*'--quiet'*|*'repowise distill'*) pass_through;; esac

# Whitelist of known-noisy, read-only, tail-safe commands (build/test/log dumps).
NOISY=0
case "$CMD" in
  'cargo build'*|'cargo test'*|'cargo clippy'*|'cargo check'*) NOISY=1;;
  'pnpm build'*|'pnpm -r build'*|'pnpm typecheck'*|'pnpm -r typecheck'*|'npm run build'*) NOISY=1;;
  'git log '*) case "$CMD" in *' -n'*|*' -[0-9]'*|*'--oneline'*) NOISY=0;; *) NOISY=1;; esac;;
esac
[ "$NOISY" -eq 1 ] || pass_through

NEWCMD="{ $CMD ; } 2>&1 | tail -n $TAIL_N"
NOTE="token-reduce: appended '| tail -n $TAIL_N' to a noisy command (mechanical). Re-run without the wrapper if you need full output; prefer 'repowise distill' for structured noise."

if command -v jq >/dev/null 2>&1; then
  printf '%s' "$INPUT" | jq -c --arg nc "$NEWCMD" --arg note "$NOTE" \
    '{hookSpecificOutput:{hookEventName:"PreToolUse", updatedInput:(.tool_input + {command:$nc}), additionalContext:$note}}'
else
  printf '%s' "$INPUT" | python3 -c '
import sys,json
d=json.load(sys.stdin); ti=dict(d.get("tool_input") or {}); ti["command"]=sys.argv[1]
print(json.dumps({"hookSpecificOutput":{"hookEventName":"PreToolUse","updatedInput":ti,"additionalContext":sys.argv[2]}}))' "$NEWCMD" "$NOTE"
fi
exit 0
