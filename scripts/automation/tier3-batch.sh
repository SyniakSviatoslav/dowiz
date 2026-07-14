#!/usr/bin/env bash
# tier3-batch.sh — Tier 3 of the dowiz agent-automation subsystem.
#
# Bounded fan-out for MECHANICAL, well-scoped sweeps only (cross-workspace
# rename/codemod, dependency-bump, lint-fix). NOT architectural change. Mutation
# happens ONLY inside a FRESH THROWAWAY CLONE (invariant A6 — never the working
# tree or live repo). Each per-target diff is checked by a READ-ONLY adversarial
# reviewer subagent (A4) — a PRE-FILTER, not the approver. Surviving diffs are
# aggregated into a PROPOSAL (OpenSpec spec / draft-PR / patch) for HUMAN approval
# — NEVER an auto-merge to main (A2). Touches no product runtime / PII (A1).
#
# Usage:  scripts/automation/tier3-batch.sh <sweep-name> <target-file> [target-file ...]
#   e.g.  scripts/automation/tier3-batch.sh ts-ignore-to-expect-error \
#           attic/apps-api/src/lib/libretranslate-provider.ts attic/apps-api/src/lib/ai-ocr-parser.ts
# Env (optional):
#   TIER3_MODEL          model alias (default: claude-haiku-4-5-20251001)        [A5]
#   TIER3_MAX_TURNS      per-agent loop cap (default: 14)                         [A5]
#   TIER3_MAX_PARALLEL   hard parallelism cap — UNITS, not hundreds (default: 3)  [A5]
#   TIER3_BATCH_MAX_USD  hard cap for the whole sweep; stop launching once hit
#                        (default: 1.00)                                         [A5]
#   TIER3_CLONE_URL      what to clone (default: origin's URL, else local repo)   [A6]
#   TIER3_CLONE_REF      branch/ref to sweep (default: main)                      [A6]
#   ***REDACTED*** + TELEGRAM_OPS_CHAT_ID   → posts the summary               [A9]
#   OTEL_EXPORTER_OTLP_ENDPOINT                 → enables OTel trace              [A9]
set -euo pipefail

SWEEP="${1:?usage: tier3-batch.sh <sweep-name> <target-file> ...}"; shift
[ $# -ge 1 ] || { echo "no targets given" >&2; exit 2; }
TARGETS=("$@")

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
AUTO="$ROOT/scripts/automation"
LOG_DIR="$AUTO/logs"; mkdir -p "$LOG_DIR"
TS="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
LOG="$LOG_DIR/batch-$(date -u +%Y%m%d).log"

SWEEP_FILE="$AUTO/prompts/sweeps/$SWEEP.md"
REVIEW_FILE="$AUTO/prompts/sweep-review.md"
[ -f "$SWEEP_FILE" ]  || { echo "no sweep rule: $SWEEP_FILE" >&2; exit 2; }
[ -f "$REVIEW_FILE" ] || { echo "no reviewer prompt: $REVIEW_FILE" >&2; exit 2; }

MODEL="${TIER3_MODEL:-claude-haiku-4-5-20251001}"
MAX_TURNS="${TIER3_MAX_TURNS:-14}"
MAX_PAR="${TIER3_MAX_PARALLEL:-3}"
CAP="${TIER3_BATCH_MAX_USD:-1.00}"
REF="${TIER3_CLONE_REF:-main}"
CLONE_URL="${TIER3_CLONE_URL:-$(cd "$ROOT" && git remote get-url origin 2>/dev/null || echo "file://$ROOT")}"

# A6: fresh, isolated, WRITABLE clone (this is where mutation is allowed); cleaned up.
CLONE="$(mktemp -d -t dowiz-t3-XXXXXX)"
WORK="$(mktemp -d -t dowiz-t3w-XXXXXX)"
cleanup() { rm -rf "$CLONE" "$WORK"; }
trap cleanup EXIT
git clone --quiet --depth 50 --branch "$REF" "$CLONE_URL" "$CLONE" 2>>"$LOG.err" \
  || git clone --quiet --depth 50 "$CLONE_URL" "$CLONE" 2>>"$LOG.err"
BRANCH="t3/${SWEEP}-$(date -u +%Y%m%d%H%M%S)"
git -C "$CLONE" checkout --quiet -b "$BRANCH"

[ -n "${OTEL_EXPORTER_OTLP_ENDPOINT:-}" ] && export CLAUDE_CODE_ENABLE_TELEMETRY=1

# Parse {result, cost} from claude JSON; tolerate non-JSON.
jget() { node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{try{const j=JSON.parse(d);process.stdout.write(JSON.stringify({r:j.result||'',c:j.total_cost_usd??0}))}catch(e){process.stdout.write(JSON.stringify({r:d.slice(0,1500),c:0}))}})" 2>/dev/null; }
field() { node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{process.stdout.write(String(JSON.parse(d).$1))})"; }

SWEEP_RULE="$(cat "$SWEEP_FILE")"
REVIEW_RULE="$(cat "$REVIEW_FILE")"

# One target: executor (writes, scoped to its file) → read-only adversarial review →
# keep or revert. Writes status + cost into $WORK. A4: executor mutates, reviewer is
# a SEPARATE read-only process (Edit/Write disallowed).
run_target() {
  local idx="$1" file="$2"
  local sf="$WORK/$idx.status" cf="$WORK/$idx.cost"
  if [ ! -f "$CLONE/$file" ]; then echo "MISSING $file" >"$sf"; echo 0 >"$cf"; return; fi

  local exec_out exec_cost
  exec_out="$(cd "$CLONE" && timeout 300 claude -p \
    "$SWEEP_RULE"$'\n\n'"Apply the sweep rule above to EXACTLY this one file: $file. Use the Edit tool. Change nothing else. Then stop." \
    --output-format json --model "$MODEL" \
    --allowed-tools "Read" "Edit" "Bash(git diff:*)" \
    --disallowed-tools "Write" "NotebookEdit" "Bash(git commit:*)" "Bash(git push:*)" \
    --max-turns "$MAX_TURNS" 2>>"$LOG.err" || true)"
  exec_cost="$(printf '%s' "$exec_out" | jget | field c)"

  local diff
  diff="$(git -C "$CLONE" diff -- "$file")"
  if [ -z "$diff" ]; then
    echo "NOOP $file (no change produced)" >"$sf"; echo "${exec_cost:-0}" >"$cf"; return
  fi

  local rev_out rev_cost verdict
  rev_out="$(timeout 300 claude -p \
    "$REVIEW_RULE"$'\n\n'"SWEEP RULE:"$'\n'"$SWEEP_RULE"$'\n\n'"GIT DIFF for $file:"$'\n'"$diff" \
    --output-format json --model "$MODEL" \
    --allowed-tools "Read" \
    --disallowed-tools "Edit" "Write" "NotebookEdit" \
    --max-turns 4 2>>"$LOG.err" || true)"
  rev_cost="$(printf '%s' "$rev_out" | jget | field c)"
  verdict="$(printf '%s' "$rev_out" | jget | field r)"

  echo "$(echo "${exec_cost:-0} + ${rev_cost:-0}" | bc -l 2>/dev/null || echo 0)" >"$cf"
  if printf '%s' "$verdict" | grep -qi 'VERDICT:[[:space:]]*PASS'; then
    echo "PASS $file" >"$sf"
  else
    git -C "$CLONE" checkout --quiet -- "$file"   # A4: reviewer rejected → revert
    echo "REJECT $file :: $(printf '%s' "$verdict" | grep -i '^reason:' | head -1)" >"$sf"
  fi
}

echo "===== tier3/$SWEEP @ $TS (ref $REF, branch $BRANCH) =====" | tee -a "$LOG"
SPENT=0; i=0; running=0
for file in "${TARGETS[@]}"; do
  # A5: stop launching once the batch cap is hit.
  if [ "$(echo "$SPENT >= $CAP" | bc -l 2>/dev/null || echo 0)" = "1" ]; then
    echo "[cap] batch budget \$$CAP reached (\$$SPENT) — skipping remaining" | tee -a "$LOG"; break
  fi
  run_target "$i" "$file" &                       # A5: bounded fan-out
  i=$((i+1)); running=$((running+1))
  if [ "$running" -ge "$MAX_PAR" ]; then wait -n 2>/dev/null || wait; running=$((running-1))
    SPENT="$(cat "$WORK"/*.cost 2>/dev/null | paste -sd+ | bc -l 2>/dev/null || echo "$SPENT")"
  fi
done
wait
SPENT="$(cat "$WORK"/*.cost 2>/dev/null | paste -sd+ | bc -l 2>/dev/null || echo 0)"
SPENT_FMT="$(printf '%.4f' "$SPENT" 2>/dev/null || echo "$SPENT")"

PASSED=0; SUMMARY="🔧 tier3/$SWEEP @ $TS (branch $BRANCH)"
for f in "$WORK"/*.status; do
  [ -f "$f" ] || continue; line="$(cat "$f")"
  SUMMARY+=$'\n'"  $line"; echo "  $line" | tee -a "$LOG"
  case "$line" in PASS*) PASSED=$((PASSED+1));; esac
done

# A2: surviving diffs become a PROPOSAL for human approval — NEVER an auto-merge.
PROPOSAL_NOTE=""
if [ "$PASSED" -gt 0 ]; then
  git -C "$CLONE" add -A
  # --no-verify: this is the internal PROPOSAL-packaging commit, not the merge.
  # Repo gates (husky/CI) need a built workspace a shallow clone lacks; they run
  # for real at the human's PR (A2). Agent CC guardrails already fired (no --bare).
  git -C "$CLONE" -c user.name="dowiz-t3" -c user.email="ops@dowiz" \
    commit --quiet --no-verify -m "chore(sweep): $SWEEP — $PASSED file(s) [tier3, adversarially reviewed]"
  PATCH="$LOG_DIR/${BRANCH//\//-}.patch"
  git -C "$CLONE" format-patch --quiet -1 --stdout > "$PATCH" 2>/dev/null || git -C "$CLONE" diff "$REF"..HEAD > "$PATCH"
  if command -v openspec >/dev/null 2>&1; then
    PROPOSAL_NOTE="proposal: OpenSpec available — run 'openspec' propose→apply on branch $BRANCH (human approves spec)"
  elif command -v gh >/dev/null 2>&1; then
    ( cd "$CLONE" && git push --quiet -u origin "$BRANCH" \
      && gh pr create --draft --fill --base "$REF" --head "$BRANCH" ) >>"$LOG" 2>>"$LOG.err" \
      && PROPOSAL_NOTE="proposal: draft PR opened from $BRANCH (never auto-merged)" \
      || PROPOSAL_NOTE="proposal: push/gh failed — patch saved: $PATCH"
  else
    PROPOSAL_NOTE="proposal: no OpenSpec/gh here — patch saved for human review: $PATCH"
  fi
else
  PROPOSAL_NOTE="proposal: none — 0 diffs survived adversarial review"
fi
SUMMARY+=$'\n'"$PROPOSAL_NOTE"$'\n'"— batch total: \$$SPENT_FMT (cap \$$CAP), $PASSED passed of ${#TARGETS[@]}"
{ echo "$PROPOSAL_NOTE"; echo "— batch total: \$$SPENT_FMT (cap \$$CAP), $PASSED passed of ${#TARGETS[@]}"; echo; } | tee -a "$LOG"

if [ -n "${***REDACTED***:-}" ] && [ -n "${TELEGRAM_OPS_CHAT_ID:-}" ]; then
  "$AUTO/notify.sh" "$SUMMARY" || true
fi
# A2/A8: NEVER merges. Output is a branch/patch/draft-PR proposal a human must approve.
