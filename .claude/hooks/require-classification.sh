#!/usr/bin/env bash
# require-classification.sh — Stop gate:
#  (1) CHANGE-MANIFEST with a CLASSIFICATION for any code change (Agent Operating Model §1);
#  (2) reflection CONTRACT: an unfilled WHY placeholder in docs/reflections/INBOX always blocks;
#  (3) reflection PULSE (meta-loop P1 2026-07-02): a QUALIFIED change (>=3 code files or a
#      red-line surface, uncommitted or in a <30-min-old HEAD) must leave a *.reflection.md in
#      INBOX (any within the last 8h satisfies). The advisory arm died 06-23 because nothing
#      forced step 4 of the self-improvement loop; this is its deterministic forcing function.
set -uo pipefail

PROJECT_ROOT="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
MANIFEST="$PROJECT_ROOT/agent/CHANGE-MANIFEST.md"
INBOX="$PROJECT_ROOT/docs/reflections/INBOX"

# P1 telemetry (meta-loop 2026-07-02): one JSONL line per decision — the advisory layer was
# unmeasurable (no lesson hit-counts, no deny/allow rates), so pruning/promotion ran blind.
# Advisory: never fails the hook.
HEV_LOG="$PROJECT_ROOT/.claude/logs/harness-events.jsonl"
_hev() {
  mkdir -p "$(dirname "$HEV_LOG")" 2>/dev/null || true
  printf '{"ts":"%s","hook":"%s","event":"%s","target":"%s","detail":"%s"}\n' \
    "$(date -Iseconds)" "$1" "$2" \
    "$(printf '%s' "${3:-}" | tr '"\\' '..' | tr '\n' ' ' | cut -c1-200)" \
    "$(printf '%s' "${4:-}" | tr '"\\' '..' | tr '\n' ' ' | cut -c1-200)" \
    >>"$HEV_LOG" 2>/dev/null || true
}

_block() {
  _hev require-classification block "" "$1"
  printf '{"decision":"block","reason":"%s"}\n' "$1"
  exit 0
}

command -v git >/dev/null 2>&1 || exit 0

# ---- (1) classification gate ----
if ! git -C "$PROJECT_ROOT" diff --quiet -- apps/ packages/ spikes/ 2>/dev/null; then
  if [ ! -f "$MANIFEST" ]; then
    _block "Code changed without agent/CHANGE-MANIFEST.md. Add it with a CLASSIFICATION line: spike | build | audit | challenge (+ FINDING-id + touched files)."
  fi
  if ! grep -qiE '^CLASSIFICATION:[[:space:]]*(spike|build|audit|challenge)\b' "$MANIFEST"; then
    _block "CHANGE-MANIFEST is missing a valid CLASSIFICATION line. Add exactly one of: CLASSIFICATION: spike | build | audit | challenge (par.1)."
  fi
fi

# ---- (2) reflection contract: unfilled WHY placeholder always blocks ----
if [ -d "$INBOX" ] && grep -rl 'fill in):' "$INBOX" >/dev/null 2>&1; then
  _block "docs/reflections/INBOX has a reflection with an UNFILLED WHY placeholder. Fill the causal WHY (why it arose, not just where) before stopping."
fi

# ---- (3) reflection pulse for a QUALIFIED change ----
REDLINE='(^|/)(auth|jwt|otp|session|login)|rls|(^|/)(price|money|payment|cash|ledger|refund)|packages/db/migrations/|\.claude/hooks/|\.claude/settings'
CHANGED="$(git -C "$PROJECT_ROOT" diff --name-only HEAD -- apps/ packages/ scripts/ tools/ .claude/ 2>/dev/null)"
if [ -z "$CHANGED" ]; then
  last_commit_ts="$(git -C "$PROJECT_ROOT" log -1 --format=%ct 2>/dev/null || echo 0)"
  head_age=$(( $(date +%s) - last_commit_ts ))
  if [ "$head_age" -lt 1800 ]; then
    CHANGED="$(git -C "$PROJECT_ROOT" log -1 --name-only --format= -- apps/ packages/ scripts/ tools/ .claude/ 2>/dev/null)"
  fi
fi
[ -z "$CHANGED" ] && exit 0

nfiles="$(printf '%s\n' "$CHANGED" | grep -c . || true)"
qualified=0
[ "$nfiles" -ge 3 ] && qualified=1
printf '%s\n' "$CHANGED" | grep -Eiq "$REDLINE" && qualified=1
[ "$qualified" -eq 0 ] && exit 0

if [ -d "$INBOX" ] && [ -n "$(find "$INBOX" -name '*.reflection.md' -mmin -480 2>/dev/null)" ]; then
  _hev require-classification pass "" qualified-with-reflection
  exit 0
fi
_block "QUALIFIED change (>=3 code files or a red-line surface) with no fresh reflection. Write docs/reflections/INBOX/<date>-<slug>.reflection.md (CONTEXT/DECISIONS/WHERE/WHY-causal/CONFIDENCE/NEXT-TIME/LINK), then stop."
