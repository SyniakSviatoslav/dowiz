#!/usr/bin/env bash
# tier1-run.sh — Tier 1 of the dowiz agent-automation subsystem.
#
# Durable, read-only, scheduled ops task via headless `claude -p`. Designed to be
# invoked by an EXTERNAL cron on the VPS (invariant A7: survives restarts; CC's
# in-session schedules do not). Emits a report to a log + Telegram-ops. Makes NO
# code changes, never auto-merges, never touches product runtime/PII (A1, A2, A8).
#
# Usage:  scripts/automation/tier1-run.sh <prompt-name>   # e.g. ops-watch
# Env (optional):
#   TIER1_MODEL        model alias (default: claude-haiku-4-5-20251001)  [A5 cheap tier]
#   TIER1_MAX_TURNS    agent-loop cap (default: 12)                       [A5 budget]
#   ***REDACTED*** + TELEGRAM_OPS_CHAT_ID   → posts the report        [A9]
#   OTEL_EXPORTER_OTLP_ENDPOINT                 → enables OTel trace      [A9]
set -euo pipefail

NAME="${1:-ops-watch}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PROMPT_FILE="$ROOT/scripts/automation/prompts/$NAME.md"
LOG_DIR="$ROOT/scripts/automation/logs"
mkdir -p "$LOG_DIR"
TS="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
LOG="$LOG_DIR/${NAME}-$(date -u +%Y%m%d).log"

[ -f "$PROMPT_FILE" ] || { echo "no prompt: $PROMPT_FILE" >&2; exit 2; }

# A3: NO --bare (hooks/guardrails must apply). A4: read-only whitelist, Edit/Write
# explicitly disallowed; dangerous bash is still vetoed by the guard-bash hook.
# A5: cheap model + turn cap. Not --dangerously-skip-permissions.
[ -n "${OTEL_EXPORTER_OTLP_ENDPOINT:-}" ] && export CLAUDE_CODE_ENABLE_TELEMETRY=1

OUT="$(cd "$ROOT" && timeout 600 claude -p "$(cat "$PROMPT_FILE")" \
  --output-format json \
  --model "${TIER1_MODEL:-claude-haiku-4-5-20251001}" \
  --allowed-tools "Read" "Grep" "Glob" "Bash(curl:*)" "Bash(git:*)" "Bash(gh run list:*)" "Bash(gh pr list:*)" \
  --disallowed-tools "Edit" "Write" "NotebookEdit" \
  --max-turns "${TIER1_MAX_TURNS:-12}" 2>>"$LOG.err" || true)"

# Parse result + cost from the JSON (fall back to raw if not JSON).
REPORT="$(printf '%s' "$OUT" | node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{try{const j=JSON.parse(d);process.stdout.write((j.result||'')+'\n[cost \$'+(j.total_cost_usd??'?')+', turns '+(j.num_turns??'?')+']')}catch(e){process.stdout.write(d.slice(0,2000))}})" 2>/dev/null || printf '%s' "$OUT")"

{ echo "===== $NAME @ $TS ====="; echo "$REPORT"; echo; } | tee -a "$LOG"

# A9: ship to Telegram-ops if configured (best-effort, never fail the run).
if [ -n "${***REDACTED***:-}" ] && [ -n "${TELEGRAM_OPS_CHAT_ID:-}" ]; then
  "$ROOT/scripts/automation/notify.sh" "🛰️ tier1/$NAME @ $TS"$'\n'"$REPORT" || true
fi
