#!/usr/bin/env bash
# context-budget-guard — deterministic session-length cap (operator directive 2026-07-05:
# "stop at 25% context usage and restart fresh").
#
# WHY: cache reads = Σ(context at every call) → grows quadratically with session length.
# The cap converts marathon sessions into h_t-handoff + fresh session — the single biggest
# cache-read lever (measured 2026-07-05: 3.26B reads/24h, 96% of processed volume).
#
# HOW: UserPromptSubmit hook. Estimates CURRENT context from the transcript's last usage
# entry (input + cache_read + cache_creation of the most recent API call ≈ the live prefix).
# Over budget → injects a mandatory wrap-up directive. A session cannot restart itself;
# the guard forces the handoff so the OPERATOR restarts with one keystroke (/clear).
#
# APPLY (protected zone — operator):
#   cp docs/operating-model/proposed-hooks/context-budget-guard.sh .claude/hooks/
#   chmod +x .claude/hooks/context-budget-guard.sh
#   # settings.json → hooks.UserPromptSubmit += {"hooks":[{"type":"command",
#   #   "command":"bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/context-budget-guard.sh\""}]}
# TUNE: CONTEXT_WINDOW (default 200000), CONTEXT_BUDGET_PCT (default 25).
# TESTED red→green 2026-07-05: fires on a real 507K-context transcript at 25%/200K;
# silent at window=10M; silent on missing/empty transcript.

set -eu
INPUT=$(cat)

CTX=$(printf '%s' "$INPUT" | python3 -c "
import sys, json
try:
    inp = json.loads(sys.stdin.read())
    path = inp.get('transcript_path') or ''
    last = 0
    with open(path, errors='ignore') as fh:
        for line in fh:
            if '\"cache_read_input_tokens\"' not in line:
                continue
            try:
                u = (json.loads(line).get('message') or {}).get('usage') or {}
            except Exception:
                continue
            v = (u.get('input_tokens') or 0) + (u.get('cache_read_input_tokens') or 0) + (u.get('cache_creation_input_tokens') or 0)
            if v:
                last = v
    print(last)
except Exception:
    print(0)
" 2>/dev/null) || CTX=0
case "$CTX" in ''|*[!0-9]*) CTX=0;; esac

WINDOW=${CONTEXT_WINDOW:-200000}
PCT=${CONTEXT_BUDGET_PCT:-25}
BUDGET=$(( WINDOW * PCT / 100 ))

[ "$CTX" -lt "$BUDGET" ] && exit 0

cat <<EOF
🧯 CONTEXT BUDGET REACHED: ~${CTX} tokens live context ≥ ${PCT}% of ${WINDOW} (budget ${BUDGET}).
MANDATORY (quadratic cache-read cost from here on):
1. Finish ONLY the current atomic step — no new arcs, no new lanes.
2. Persist state: update docs/ops/*-h_t.json (+ re-encode .vsa1) and the session-resume memory.
3. End with a HANDOFF block (next step, open gates, lane IDs) and tell the operator to /clear
   or start a fresh session resumed from the h_t frame.
EOF
