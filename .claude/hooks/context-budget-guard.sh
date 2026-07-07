#!/usr/bin/env bash
# context-budget-guard.sh — deterministic TWO-TIER session-length cap.
#
# Operator directive 2026-07-05 ("stop at ~25% and restart fresh"); TIGHTENED 2026-07-07 per
# docs/operating-model/token-reduction-enforcement-2026-07-07.md §A1 into a graduated WARN→HARD
# ladder. GROUND TRUTH that justifies the second tier: audit-token-router measured a real peak
# lead-session context of 452,886 tokens (newest-12 run) — sessions ARE overrunning the 300K recycle
# line toward ~450K, so a single threshold under-serves. WARN lands at the 300K recycle line (soft:
# finish the atomic step, start the handoff); HARD at ~400K (mandatory wrap-up + operator /clear).
#
# WHY: cache reads = Σ(context at every call) → grows quadratically with session length. The cap
# converts marathon sessions into h_t-handoff + fresh session — the single biggest cache-read lever
# (measured: cache-read = 62.8% of lead-loop $, §11c of the token-economy report).
#
# HOW: UserPromptSubmit hook. Estimates CURRENT context from the transcript's last usage entry
# (input + cache_read + cache_creation of the most recent API call ≈ the live prefix). A session
# cannot restart itself; the guard forces the handoff so the OPERATOR restarts with one keystroke
# (/clear). NEVER blocks — UserPromptSubmit output is injected context, exit 0 always.
#
# TUNE (env): CONTEXT_WINDOW (default 200000). CONTEXT_BUDGET_PCT = the WARN tier (default 25; the
# live settings.json passes 30 → 300K@1M). CONTEXT_HARD_PCT = the HARD tier (default 40 → 400K@1M).
# FALSIFIABLE (VbM): `bash context-budget-guard.sh --self-test` synthesizes contexts at 3 levels and
# asserts silent<WARN, WARN≤ctx<HARD (and NOT hard-stop — the RED), HARD≤ctx → hard-stop, plus the
# real-transcript extraction path (452,886 parsed exactly; missing transcript → 0/silent).
set -eu

# ── CTX extraction: hook JSON on stdin → integer live-context tokens (last usage row) ──
_ctx_from_stdin() {
  python3 -c "
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
" 2>/dev/null || echo 0
}

# ── tier directive: CTX WARN_BUDGET HARD_BUDGET WINDOW → prints the tier directive (or nothing) ──
_emit_tier() {
  ctx=$1; warn=$2; hard=$3; window=$4
  if [ "$ctx" -ge "$hard" ]; then
    cat <<EOF
🧯 CONTEXT BUDGET — HARD STOP: ~${ctx} tokens live context ≥ HARD tier (${hard} of ${window}).
Cache-read cost is quadratic in session length and you are past the recycle line. MANDATORY NOW:
1. Finish ONLY the current atomic step — no new arcs, no new lanes, no new files.
2. Persist state: update docs/ops/*-h_t.json (+ re-encode .vsa1) and the session-resume memory.
3. End with a HANDOFF block (next step, open gates, lane IDs) and tell the operator to /clear and
   start a FRESH session resumed from the h_t frame. Do not continue past this step.
EOF
  elif [ "$ctx" -ge "$warn" ]; then
    cat <<EOF
⚠ CONTEXT BUDGET — WARN: ~${ctx} tokens live context ≥ WARN tier (${warn} of ${window}).
You are at the ${warn}-token recycle line; cache-read cost is climbing quadratically. Plan to wrap:
1. Prefer finishing the current atomic step over opening a NEW arc or lane.
2. Start drafting the HANDOFF (next step, open gates) so a /clear at the HARD tier (${hard}) is free.
EOF
  fi
}

# ── hermetic self-test (VbM: red + green) ─────────────────────────────────────
_self_test() {
  fix=$(mktemp -d)
  WIN=1000000; W=300000; H=400000; ok=1
  _has() { printf '%s' "$1" | grep -q "$2"; }
  chk() { if [ "$2" = "1" ]; then printf '  \xe2\x9c\x93 %s\n' "$1"; else printf '  \xe2\x9c\x97 %s\n' "$1"; ok=0; fi; }

  out=$(_emit_tier 250000 "$W" "$H" "$WIN")
  chk 'below WARN (250K) → silent' "$([ -z "$out" ] && echo 1 || echo 0)"

  out=$(_emit_tier 320000 "$W" "$H" "$WIN")
  chk 'WARN tier (320K) → emits WARN' "$(_has "$out" 'CONTEXT BUDGET — WARN' && echo 1 || echo 0)"
  chk 'WARN tier (320K) → NOT hard-stop (RED: WARN must differ from HARD)' "$(_has "$out" 'HARD STOP' && echo 0 || echo 1)"

  out=$(_emit_tier 420000 "$W" "$H" "$WIN")
  chk 'HARD tier (420K) → emits HARD STOP' "$(_has "$out" 'HARD STOP' && echo 1 || echo 0)"
  chk 'HARD tier (420K) → NOT the soft WARN heading (RED)' "$(_has "$out" 'CONTEXT BUDGET — WARN' && echo 0 || echo 1)"

  # boundary: exactly at WARN fires WARN, exactly at HARD fires HARD
  chk 'exactly WARN (300K) → WARN fires' "$(_has "$(_emit_tier 300000 "$W" "$H" "$WIN")" 'CONTEXT BUDGET — WARN' && echo 1 || echo 0)"
  chk 'exactly HARD (400K) → HARD fires' "$(_has "$(_emit_tier 400000 "$W" "$H" "$WIN")" 'HARD STOP' && echo 1 || echo 0)"

  # end-to-end extraction from a real temp transcript (the 452,886 ground-truth value)
  printf '{"message":{"usage":{"input_tokens":452886,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}}}\n' > "$fix/t.jsonl"
  ctx=$(printf '{"transcript_path":"%s"}' "$fix/t.jsonl" | _ctx_from_stdin)
  chk 'extraction: 452,886 transcript parsed exactly' "$([ "$ctx" = "452886" ] && echo 1 || echo 0)"
  ctx=$(printf '{"transcript_path":"%s/nope.jsonl"}' "$fix" | _ctx_from_stdin)
  chk 'extraction: missing transcript → 0 (fail-safe silent)' "$([ "$ctx" = "0" ] && echo 1 || echo 0)"

  rm -rf "$fix"
  if [ "$ok" = "1" ]; then
    printf '\n\xe2\x9c\x93 context-budget-guard self-test: two-tier ladder + RED (WARN\xe2\x89\xa0HARD) + extraction all pass.\n'
  else
    printf '\n\xe2\x9c\x97 context-budget-guard self-test FAILED\n'; exit 1
  fi
}

# ── main ──────────────────────────────────────────────────────────────────────
if [ "${1:-}" = "--self-test" ]; then _self_test; exit 0; fi

INPUT=$(cat)
CTX=$(printf '%s' "$INPUT" | _ctx_from_stdin)
case "$CTX" in ''|*[!0-9]*) CTX=0;; esac

WINDOW=${CONTEXT_WINDOW:-200000}
WARN_PCT=${CONTEXT_BUDGET_PCT:-25}
HARD_PCT=${CONTEXT_HARD_PCT:-40}
WARN_BUDGET=$(( WINDOW * WARN_PCT / 100 ))
HARD_BUDGET=$(( WINDOW * HARD_PCT / 100 ))
[ "$HARD_BUDGET" -lt "$WARN_BUDGET" ] && HARD_BUDGET=$WARN_BUDGET   # HARD never below WARN

_emit_tier "$CTX" "$WARN_BUDGET" "$HARD_BUDGET" "$WINDOW"
exit 0
