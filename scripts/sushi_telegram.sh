#!/bin/bash
# Send a screenshot or message to Telegram channel
# Usage: ./scripts/sushi_telegram.sh screenshot <path> <caption>
#        ./scripts/sushi_telegram.sh message <text>
#        ./scripts/sushi_telegram.sh report <orders_count> <revenue>

set -euo pipefail
source /root/dowiz/tools/telemetry/lib.sh 2>/dev/null || true

MODE="${1:-message}"
CONTENT="${2:-}"

case "$MODE" in
  screenshot)
    CAPTION="${3:-Sushi Durres screenshot}"
    if command -v tg_send_photo &>/dev/null; then
      tg_send_photo "$CONTENT" "$CAPTION"
    else
      echo "[TELEGRAM] Would send photo: $CONTENT — $CAPTION"
    fi
    ;;
  message)
    if command -v tg_send &>/dev/null; then
      tg_send "$CONTENT"
    else
      echo "[TELEGRAM] Would send: $CONTENT"
    fi
    ;;
  report)
    ORDERS="${2:-0}"
    REVENUE="${3:-0}"
    MSG="📊 Sushi Durres Daily Report
━━━━━━━━━━━━━━━━━
🛒 Orders: $ORDERS
💰 Revenue: $REVENUE ALL
📅 Date: $(date '+%Y-%m-%d %H:%M')
━━━━━━━━━━━━━━━━━"
    if command -v tg_send &>/dev/null; then
      tg_send "$MSG"
    else
      echo "[TELEGRAM] $MSG"
    fi
    ;;
esac
