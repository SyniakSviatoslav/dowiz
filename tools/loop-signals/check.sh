#!/usr/bin/env bash
# check.sh — on-demand loop SELF-CHECK: "am I (the agent) stuck in a loop right now?"
# Runs the Markov attractor detector against the CURRENT session transcript
# (failure-inclusive — the source the PostToolUse hook is blind to). Same signal the
# Stop/PostToolUse hooks fire automatically, but callable by the agent PROACTIVELY —
# the loop-health mirror of tools/verify-scope.sh (which answers "does my diff verify?").
#
# Usage:
#   bash tools/loop-signals/check.sh                 # auto-detect current transcript
#   bash tools/loop-signals/check.sh <t.jsonl> [n]   # explicit transcript, last n events
# Exit: 0 = HEALTHY, 1 = trapped (usable in a preflight).
set -u
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"
HERE="$ROOT/tools/loop-signals"
N="${2:-40}"
TX="${1:-}"
if [ -z "$TX" ]; then
  PROJ="$HOME/.claude/projects/$(printf '%s' "$ROOT" | sed 's#/#-#g')"
  TX="$(ls -t "$PROJ"/*.jsonl 2>/dev/null | head -1)"
  [ -z "$TX" ] && TX="$(ls -t "$HOME"/.claude/projects/*/*.jsonl 2>/dev/null | head -1)"
fi
if [ -z "$TX" ] || [ ! -f "$TX" ]; then
  echo "loop self-check: no session transcript found (fail-open)"; exit 0
fi
JSON="$(python3 "$HERE/transcript_events.py" "$TX" "$N" 2>/dev/null | python3 "$HERE/markov_attractor.py" 2>/dev/null)"
[ -z "$JSON" ] && { echo "loop self-check: analyzer produced no output (fail-open)"; exit 0; }
python3 - "$JSON" "$TX" <<'PY'
import json, sys
d = json.loads(sys.argv[1])
v = d.get("verdict", "?")
icon = {"HEALTHY": "🟢", "LIMIT_CYCLE": "🔴", "STRANGE_ATTRACTOR": "🔴"}.get(v, "⚪")
print(f"{icon} loop self-check: {v}")
print(f"   {d.get('reason','')}")
print(f"   events={d.get('events','?')} escape={d.get('escape_mass','?')} "
      f"entropy={d.get('entropy_rate_bits','?')} drift={d.get('drift','?')} "
      f"has_failure={d.get('has_failure','?')}")
print(f"   source: {sys.argv[2]}")
if v != "HEALTHY":
    print("   → you are orbiting without reaching a green/progress state. Change a STRUCTURAL")
    print("     variable (approach / context / model / assumption) or escalate (skill: doubt-escalation).")
    sys.exit(1)
PY
