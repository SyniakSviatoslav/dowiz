#!/usr/bin/env bash
# guard-bash.sh — Block dangerous shell commands (deploy, infra, deps, migration, merge-level)
set -euo pipefail

INPUT=$(cat)
CMD=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

DANGER='fly\s+(deploy|secrets)|supabase\s|wrangler\s|git\s+push\s+(origin\s+)?main|git\s+push\s+--force|pnpm\s+migrate:up|pnpm\s+(add|remove)|npm\s+install|rm\s+-rf\s+/'

if echo "$CMD" | grep -qiE "$DANGER"; then
  echo "BLOCKED: '$CMD' — deploy/infra/deps/migration/merge-level command. Manual only." >&2
  exit 2
fi
