#!/usr/bin/env bash
# tier2-overnight.sh — Tier 2 of the dowiz agent-automation subsystem.
#
# Overnight deep-audit. Runs a series of READ-ONLY `claude -p` routines against a
# FRESH THROWAWAY CLONE (invariant A6: never the working tree or a live repo, so
# it cannot corrupt state), aggregates a morning report, and ships it to a log +
# Telegram-ops (A9). Makes NO code changes to the working tree, never auto-merges,
# never touches product runtime/PII (A1, A2, A8). Designed for an EXTERNAL nightly
# cron on the VPS (A7: survives restarts; CC in-session schedules do not).
#
# Usage:  scripts/automation/tier2-overnight.sh [routine ...]   # default: overnight-audit weekly-recap
# Env (optional):
#   TIER2_MODEL        model alias (default: claude-haiku-4-5-20251001)        [A5 cheap tier]
#   TIER2_MAX_TURNS    per-routine agent-loop cap (default: 25)                 [A5 budget]
#   TIER2_NIGHT_MAX_USD  hard cap for the whole run; skip remaining routines
#                        once exceeded (default: 1.00)                          [A5 budget]
#   TIER2_CLONE_URL    what to clone (default: origin's URL, else local repo)   [A6]
#   TIER2_CLONE_REF    branch/ref to audit (default: main)                      [A6]
#   TELEGRAM_BOT_TOKEN + TELEGRAM_OPS_CHAT_ID   → posts the report              [A9]
#   OTEL_EXPORTER_OTLP_ENDPOINT                 → enables OTel trace            [A9]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
AUTO="$ROOT/scripts/automation"
LOG_DIR="$AUTO/logs"
mkdir -p "$LOG_DIR"
TS="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
LOG="$LOG_DIR/overnight-$(date -u +%Y%m%d).log"

ROUTINES=("$@")
[ ${#ROUTINES[@]} -eq 0 ] && ROUTINES=(overnight-audit weekly-recap)

MODEL="${TIER2_MODEL:-claude-haiku-4-5-20251001}"
MAX_TURNS="${TIER2_MAX_TURNS:-25}"
NIGHT_CAP="${TIER2_NIGHT_MAX_USD:-1.00}"
REF="${TIER2_CLONE_REF:-main}"
CLONE_URL="${TIER2_CLONE_URL:-$(cd "$ROOT" && git remote get-url origin 2>/dev/null || echo "file://$ROOT")}"

# A6: fresh, isolated clone in a throwaway dir; always cleaned up (idempotent — A7).
CLONE="$(mktemp -d -t dowiz-t2-XXXXXX)"
cleanup() { rm -rf "$CLONE"; }
trap cleanup EXIT
git clone --quiet --depth 50 --branch "$REF" "$CLONE_URL" "$CLONE" 2>>"$LOG.err" \
  || git clone --quiet --depth 50 "$CLONE_URL" "$CLONE" 2>>"$LOG.err"

# A9: emit OTel trace only when an endpoint is configured.
[ -n "${OTEL_EXPORTER_OTLP_ENDPOINT:-}" ] && export CLAUDE_CODE_ENABLE_TELEMETRY=1

# Parse {result, total_cost_usd, num_turns} from claude's JSON output.
parse() { node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{try{const j=JSON.parse(d);process.stdout.write(JSON.stringify({r:j.result||'',c:j.total_cost_usd??0,t:j.num_turns??0}))}catch(e){process.stdout.write(JSON.stringify({r:d.slice(0,2000),c:0,t:0}))}})" 2>/dev/null; }

SPENT=0
SUMMARY="🌙 tier2/overnight @ $TS (ref $REF)"

for NAME in "${ROUTINES[@]}"; do
  PROMPT_FILE="$AUTO/prompts/$NAME.md"
  [ -f "$PROMPT_FILE" ] || { echo "[skip] no prompt: $NAME" | tee -a "$LOG"; continue; }

  # A5: stop launching routines once the night cap is hit.
  if [ "$(echo "$SPENT >= $NIGHT_CAP" | bc -l 2>/dev/null || echo 0)" = "1" ]; then
    NOTE="[cap] night budget \$$NIGHT_CAP reached after \$$SPENT — skipping $NAME"
    echo "$NOTE" | tee -a "$LOG"; SUMMARY+=$'\n'"$NOTE"; continue
  fi

  # A3: NO --bare (hooks/guardrails apply). A4: read-only whitelist + Edit/Write
  # disallowed; not --dangerously-skip-permissions. Runs INSIDE the clone (A6).
  OUT="$(cd "$CLONE" && timeout 900 claude -p "$(cat "$PROMPT_FILE")" \
    --output-format json \
    --model "$MODEL" \
    --allowed-tools "Read" "Grep" "Glob" "Bash(git:*)" "Bash(pnpm audit:*)" "Bash(gh pr list:*)" "Bash(gh run list:*)" "mcp__repowise__get_dead_code" \
    --disallowed-tools "Edit" "Write" "NotebookEdit" \
    --max-turns "$MAX_TURNS" 2>>"$LOG.err" || true)"

  P="$(printf '%s' "$OUT" | parse)"
  REPORT="$(printf '%s' "$P" | node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{const j=JSON.parse(d);process.stdout.write(j.r)})")"
  COST="$(printf '%s'  "$P" | node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{const j=JSON.parse(d);process.stdout.write(String(j.c))})")"
  TURNS="$(printf '%s' "$P" | node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{const j=JSON.parse(d);process.stdout.write(String(j.t))})")"
  SPENT="$(echo "$SPENT + $COST" | bc -l 2>/dev/null || echo "$SPENT")"

  BLOCK="===== $NAME @ $TS [cost \$$COST, turns $TURNS] ====="$'\n'"$REPORT"
  { echo "$BLOCK"; echo; } | tee -a "$LOG"
  SUMMARY+=$'\n\n'"$BLOCK"
done

SPENT_FMT="$(printf '%.4f' "$SPENT" 2>/dev/null || echo "$SPENT")"
SUMMARY+=$'\n\n'"— night total: \$$SPENT_FMT (cap \$$NIGHT_CAP)"
{ echo "— night total: \$$SPENT_FMT (cap \$$NIGHT_CAP)"; echo; } | tee -a "$LOG"

# A9: ship the aggregated morning report to Telegram-ops (best-effort).
if [ -n "${TELEGRAM_BOT_TOKEN:-}" ] && [ -n "${TELEGRAM_OPS_CHAT_ID:-}" ]; then
  "$AUTO/notify.sh" "$SUMMARY" || true
fi

# A2/A8: this tier is read-only by default — it emits a report, never a commit.
# An optional draft-PR is a documented, opt-in escalation (see README, "Draft-PR"),
# kept OFF here so the working tree stays untouched and the run is trivially safe.
