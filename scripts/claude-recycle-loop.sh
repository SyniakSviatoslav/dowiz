#!/usr/bin/env bash
# claude-recycle-loop.sh — AUTOMATIC session restart (the launcher-level half of the anti-context-rot
# rule; the harness cannot self-restart, so this wraps it).
#
# Runs a Claude Code session; when that session writes the recycle signal (.claude/state/RECYCLE —
# emitted by session-recycle-guard.sh once cumulative tokens cross the 300K ceiling), this relaunches
# a FRESH session. On the fresh session the SessionStart context-primer + the handoff line in the
# signal resurface state, so work continues without the rotted context.
#
# Usage:  bash scripts/claude-recycle-loop.sh [args passed to claude ...]
#   headless/autonomous (claude -p '...' / cron): fully automatic — session ends, loop relaunches.
#   interactive: the guard forces a clean stop at the ceiling; this loop then relaunches a fresh one.
# Env: CLAUDE_BIN (default "claude"), CLAUDE_MAX_RESTARTS (default 20).
set -uo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
SIGNAL="$ROOT/.claude/state/RECYCLE"
BIN="${CLAUDE_BIN:-claude}"
MAX="${CLAUDE_MAX_RESTARTS:-20}"
n=0

mkdir -p "$(dirname "$SIGNAL")" 2>/dev/null || true
while :; do
  rm -f "$SIGNAL" 2>/dev/null || true
  "$BIN" "$@" || true
  if [ -f "$SIGNAL" ] && [ "$n" -lt "$MAX" ]; then
    n=$((n + 1))
    echo "[recycle-loop] token ceiling hit → fresh session #$((n + 1)) — handoff: $(cat "$SIGNAL" 2>/dev/null | head -c 200)"
    continue
  fi
  [ -f "$SIGNAL" ] && echo "[recycle-loop] max restarts ($MAX) reached — stopping to avoid a runaway loop."
  break
done
