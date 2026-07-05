#!/usr/bin/env bash
# session-recycle-guard.sh — Stop hook. The other half of AUTOMATIC session restart: when a session
# ends with cumulative tokens ≥ the 300K ceiling, it writes the recycle SIGNAL (+ a handoff pointer)
# that scripts/claude-recycle-loop.sh watches to relaunch a FRESH session. Pairs with the PostToolUse
# token-circuit-guard (which emits the "save+push+/clean" directive earlier). Read-only re: the repo;
# writes only the signal file. Fail-open.
set -uo pipefail

SESSION_MAX="${TOKEN_SESSION_MAX:-300000}"
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"
SIGNAL="$ROOT/.claude/state/RECYCLE"
INPUT="$(cat)"

TP="$(printf '%s' "$INPUT" | (jq -r '.transcript_path // ""' 2>/dev/null \
  || python3 -c 'import sys,json;print(json.load(sys.stdin).get("transcript_path",""))' 2>/dev/null))"
[ -n "$TP" ] && [ -f "$TP" ] || { echo '{}'; exit 0; }

sum() {
  if command -v jq >/dev/null 2>&1; then
    jq -s 'map(.message.usage // .usage // empty)
           | map((.input_tokens//0)+(.cache_creation_input_tokens//0)+(.output_tokens//0)) | add // 0' "$TP" 2>/dev/null
  else
    python3 - "$TP" <<'PY' 2>/dev/null
import sys,json
t=0
for l in open(sys.argv[1],encoding="utf-8",errors="ignore"):
    l=l.strip()
    if not l: continue
    try: o=json.loads(l)
    except: continue
    u=(o.get("message") or {}).get("usage") or o.get("usage") or {}
    t+=int(u.get("input_tokens",0))+int(u.get("cache_creation_input_tokens",0))+int(u.get("output_tokens",0))
print(t)
PY
  fi
}
USED="$(sum)"; case "$USED" in ''|*[!0-9]*) echo '{}'; exit 0;; esac

if [ "$USED" -ge "$SESSION_MAX" ]; then
  mkdir -p "$(dirname "$SIGNAL")" 2>/dev/null || true
  printf 'recycle @%s tok — resume from MEMORY.md index + latest h_t frame / handoff. (context-primer will resurface.)\n' "$USED" > "$SIGNAL" 2>/dev/null || true
  # Advisory systemMessage; the launcher does the actual relaunch on this signal.
  printf '{"systemMessage":"⛔ session ~%s tok ≥ %s — recycle signal written. claude-recycle-loop.sh will relaunch a fresh session; ensure everything is committed+pushed."}\n' "$USED" "$SESSION_MAX"
else
  echo '{}'
fi
exit 0
