#!/usr/bin/env bash
# Loop Selection Router — UserPromptSubmit hook (Router spec §4).
#
# OPERATOR-INSTALL (not auto-installed — .claude/ is protect-paths-gated):
#   1. Copy/symlink this into your hooks dir, e.g. .claude/hooks/router-hook.sh
#   2. Register it in .claude/settings.json under hooks.UserPromptSubmit:
#        { "hooks": { "UserPromptSubmit": [
#            { "hooks": [ { "type": "command",
#                "command": "bash $CLAUDE_PROJECT_DIR/.claude/hooks/router-hook.sh" } ] } ] } }
#   3. (Backup) add to CLAUDE.md: "Run loop-selection (the router) first on every command."
#
# Contract: Claude Code passes the prompt as JSON on stdin ({"prompt": "..."}).
# We route it and print one announce line; Claude Code injects stdout as context.
# ADVISORY: always exits 0 — the router announces + records; it never blocks a command.
set -euo pipefail

ROOT="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}"
INPUT="$(cat || true)"

# Extract .prompt from the hook JSON (fallback: treat stdin as the raw prompt).
PROMPT="$(printf '%s' "$INPUT" | node -e 'let d="";process.stdin.on("data",c=>d+=c).on("end",()=>{try{process.stdout.write(JSON.parse(d).prompt||"")}catch{process.stdout.write(d)}})' 2>/dev/null || printf '%s' "$INPUT")"
[ -z "${PROMPT// }" ] && exit 0

printf '%s' "$PROMPT" | npx --prefix "$ROOT" tsx "$ROOT/tools/loop-harness/src/router.ts" "$ROOT/loops/runs" 2>/dev/null || true
exit 0
