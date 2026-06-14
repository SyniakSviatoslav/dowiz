#!/usr/bin/env bash
# protect-paths.sh — Block edits to protected zones (contracts, schema, infra, governance)
set -euo pipefail

INPUT=$(cat)
FILE=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

PROTECTED='(^|/)(migrations|\.github|\.claude)/|(^|/)(fly\.toml|Dockerfile|pnpm-lock\.yaml)$|/package\.json$|packages/shared-types/|packages/db/|/contracts/|\.contract\.|/\.env'

if echo "$FILE" | grep -qE "$PROTECTED"; then
  echo "BLOCKED: '$FILE' is in a protected zone (contracts/schema/infra/governance). This is an IMPROVEMENT requiring manual approval." >&2
  exit 2
fi
