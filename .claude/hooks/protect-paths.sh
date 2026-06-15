#!/usr/bin/env bash
# protect-paths.sh — Block edits to protected zones (contracts, schema, infra, governance)
set -euo pipefail

INPUT=$(cat)

# Parse JSON without requiring jq (jq not available on all platforms, e.g. Windows)
_extract_path() {
  if command -v jq &>/dev/null; then
    echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null
  elif command -v python3 &>/dev/null; then
    echo "$INPUT" | python3 -c "
import sys, json
try:
    d = json.loads(sys.stdin.read())
    print(d.get('tool_input', {}).get('file_path', ''), end='')
except Exception:
    pass
" 2>/dev/null || true
  elif command -v python &>/dev/null; then
    echo "$INPUT" | python -c "
import sys, json
try:
    d = json.loads(sys.stdin.read())
    print(d.get('tool_input', {}).get('file_path', ''), end='')
except Exception:
    pass
" 2>/dev/null || true
  elif command -v node &>/dev/null; then
    echo "$INPUT" | node -e "
let d = '';
process.stdin.on('data', c => d += c);
process.stdin.on('end', () => {
  try { const o = JSON.parse(d); process.stdout.write(o?.tool_input?.file_path || ''); } catch (e) {}
});
" 2>/dev/null || true
  fi
}

FILE=$(_extract_path)

PROTECTED='(^|/)(migrations|\.github|\.claude)/|(^|/)(fly\.toml|Dockerfile|pnpm-lock\.yaml)$|/package\.json$|packages/shared-types/|packages/db/|/contracts/|\.contract\.|/\.env'

if [ -n "$FILE" ] && echo "$FILE" | grep -qE "$PROTECTED"; then
  echo "BLOCKED: '$FILE' is in a protected zone (contracts/schema/infra/governance). This is an IMPROVEMENT requiring manual approval." >&2
  exit 2
fi
