#!/usr/bin/env bash
# report.sh — autopilot progress reporter for the autonomous session.
# Posts a milestone/status message to the Telegram "Reports" topic (257)
# via the shared tg_send primitive. NEVER prints the token.
# Usage: report.sh <topic-id> <"message text">
set -uo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$DIR/lib.sh"
TELEGRAM_TOPIC_ID="${1:-257}"
shift 2>/dev/null || true
text="$*"
if [ -z "$text" ]; then
  echo "report.sh: no message body" >&2
  exit 1
fi
tg_send "$text" || { echo "report.sh: tg_send failed (logged to stderr)";
  # fall back to local log so we never lose the milestone
  echo "[$(date -u +%s)] topic=$TELEGRAM_TOPIC_ID $text" >> "$LOG_DIR/autopilot-report.log"; }
