#!/bin/bash
# Manual hand-off runbook: when the Claude API/Claude Code is unreachable
# (rate-limited, outage, quota), use this to keep working on dowiz via Hermes
# Agent instead. Hermes picks up the same operating rules and memory index
# via HERMES.md (see scripts/sync-memory-to-hermes.mjs) and its own
# fallback_providers chain (gemini -> openrouter/qwen3-coder -> openrouter/gpt-oss,
# configured in ~/.hermes/config.yaml) in case its own primary model is also down.
#
# Usage:
#   scripts/hermes-fallback.sh "finish the checkout idempotency test I was writing"
#   scripts/hermes-fallback.sh --continue my-session "keep going on the same task"
#   HERMES_FALLBACK_YOLO=1 scripts/hermes-fallback.sh "unattended cron-style task"
#
# Deliberately does NOT default to --yolo: this repo has auth/money/RLS/
# migration red-lines, and a fallback agent should hit the same approval
# prompts a primary session would. Set HERMES_FALLBACK_YOLO=1 to opt out for a
# specific unattended run — not a standing default.

set -euo pipefail

REPO_ROOT="/root/dowiz"
CONTINUE_ARGS=()

if [ "${1:-}" = "--continue" ]; then
  CONTINUE_ARGS=(--continue "$2")
  shift 2
fi

TASK="${1:-}"
if [ -z "$TASK" ]; then
  echo "Usage: $0 [--continue SESSION_NAME] \"<task description>\"" >&2
  exit 1
fi

YOLO_ARGS=()
if [ "${HERMES_FALLBACK_YOLO:-0}" = "1" ]; then
  echo "⚠ HERMES_FALLBACK_YOLO=1 — dangerous-command approval prompts bypassed for this run" >&2
  YOLO_ARGS=(--yolo)
fi

cd "$REPO_ROOT"
exec hermes chat -q "$TASK" --checkpoints --source tool "${CONTINUE_ARGS[@]}" "${YOLO_ARGS[@]}"
