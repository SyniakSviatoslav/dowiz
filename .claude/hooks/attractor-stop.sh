#!/usr/bin/env bash
# attractor-stop.sh — Stop hook. Closes the pure-failure blind spot: PostToolUse
# fires ONLY on success, so a turn of only failures never triggers loop-detector.sh's
# attractor check. The Stop hook fires at turn-end regardless and carries
# transcript_path, so evaluate the (failure-inclusive) transcript here too.
# Advisory, fail-open, NEVER blocks (additionalContext only), dedup'd with the
# PostToolUse path via the shared .attractor-last so it never double-nags.
set -uo pipefail
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"
STATE_DIR="$ROOT/.claude/.loop-state"; mkdir -p "$STATE_DIR" 2>/dev/null || true
INPUT="$(cat)"
TX="$(printf '%s' "$INPUT" | { jq -r '.transcript_path // empty' 2>/dev/null || python3 -c 'import sys,json;print(json.load(sys.stdin).get("transcript_path",""))' 2>/dev/null; })"
TE="$ROOT/tools/loop-signals/transcript_events.py"
MA="$ROOT/tools/loop-signals/markov_attractor.py"
LAST_A="$STATE_DIR/.attractor-last"
[ -n "$TX" ] && [ -f "$TX" ] && command -v python3 >/dev/null 2>&1 && [ -f "$TE" ] && [ -f "$MA" ] || exit 0
AJSON="$(python3 "$TE" "$TX" 40 2>/dev/null | python3 "$MA" 2>/dev/null || true)"
AVERDICT="$(printf '%s' "$AJSON" | sed -n 's/.*"verdict": *"\([A-Z_]*\)".*/\1/p')"
AREASON="$(printf '%s' "$AJSON" | sed -n 's/.*"reason": *"\([^"]*\)".*/\1/p')"
case "$AVERDICT" in
  LIMIT_CYCLE|STRANGE_ATTRACTOR)
    PREV="$(cat "$LAST_A" 2>/dev/null || echo)"
    if [ "$AVERDICT" != "$PREV" ]; then
      printf '%s' "$AVERDICT" > "$LAST_A" 2>/dev/null || true
      ADIR="🔴 ATTRACTOR at turn-end (Markov over the transcript, failures included) — ${AVERDICT}: ${AREASON}. You are stopping while orbiting without reaching a green/progress state. Next turn: change a STRUCTURAL variable or escalate (skill: doubt-escalation) — do not resume the same path."
      if command -v jq >/dev/null 2>&1; then
        jq -nc --arg d "$ADIR" '{hookSpecificOutput:{hookEventName:"Stop",additionalContext:$d}}'
      else
        python3 -c 'import json,sys;print(json.dumps({"hookSpecificOutput":{"hookEventName":"Stop","additionalContext":sys.argv[1]}}))' "$ADIR"
      fi
    fi ;;
  HEALTHY|"") : > "$LAST_A" 2>/dev/null || true ;;
esac
exit 0
