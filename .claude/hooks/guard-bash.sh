#!/usr/bin/env bash
# guard-bash.sh — Block dangerous shell commands (deploy, infra, deps, migration, merge-level)
set -euo pipefail

INPUT=$(cat)

# Parse JSON without requiring jq (jq not available on all platforms, e.g. Windows)
_extract_cmd() {
  if command -v jq &>/dev/null; then
    echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null
  elif command -v python3 &>/dev/null; then
    echo "$INPUT" | python3 -c "
import sys, json
try:
    d = json.loads(sys.stdin.read())
    print(d.get('tool_input', {}).get('command', ''), end='')
except Exception:
    pass
" 2>/dev/null || true
  elif command -v python &>/dev/null; then
    echo "$INPUT" | python -c "
import sys, json
try:
    d = json.loads(sys.stdin.read())
    print(d.get('tool_input', {}).get('command', ''), end='')
except Exception:
    pass
" 2>/dev/null || true
  elif command -v node &>/dev/null; then
    echo "$INPUT" | node -e "
let d = '';
process.stdin.on('data', c => d += c);
process.stdin.on('end', () => {
  try { const o = JSON.parse(d); process.stdout.write(o?.tool_input?.command || ''); } catch (e) {}
});
" 2>/dev/null || true
  fi
}

CMD=$(_extract_cmd)

DANGER='fly\s+(deploy|secrets)|supabase\s|git\s+push\s+(origin\s+)?main|git\s+push\s+--force|pnpm\s+migrate:up|pnpm\s+(add|remove)|npm\s+install|rm\s+-rf\s+/'

if [ -n "$CMD" ] && echo "$CMD" | grep -qiE "$DANGER"; then
  echo "BLOCKED: '$CMD' — deploy/infra/deps/migration/merge-level command. Manual only." >&2
  exit 2
fi
