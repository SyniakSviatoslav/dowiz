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
# Stage 2 (Markov analyze) is now the native kernel binary `markov_attractor`
# (built from the kernel crate: `cargo build --release --bin markov_attractor` →
# kernel/target/release/markov_attractor). Stage 1 (transcript→token mapping) stays
# the Python extractor. The bin emits the exact JSON contract this script parses.
MA_BIN="$(cd "$ROOT" && pwd)/kernel/target/release/markov_attractor"
if [ ! -x "$MA_BIN" ]; then
  MA_BIN="$(cd "$ROOT" && pwd)/kernel/target/debug/markov_attractor"
fi
if [ -x "$MA_BIN" ]; then
  JSON="$(python3 "$HERE/transcript_events.py" "$TX" "$N" 2>/dev/null | "$MA_BIN" 2>/dev/null)"
else
  # Fallback: keep the (now-deleted) Python path silent rather than fail-closed on missing build.
  JSON=""
fi
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
