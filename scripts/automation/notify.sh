#!/usr/bin/env bash
# notify.sh — post an ops message to Telegram-ops (best-effort; never fails caller).
set -euo pipefail
MSG="${1:?usage: notify.sh <message>}"
[ -n "${TELEGRAM_BOT_TOKEN:-}" ] && [ -n "${TELEGRAM_OPS_CHAT_ID:-}" ] || {
  echo "[notify] TELEGRAM_BOT_TOKEN / TELEGRAM_OPS_CHAT_ID not set — skipping" >&2; exit 0; }
curl -s -X POST "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage" \
  --data-urlencode "chat_id=${TELEGRAM_OPS_CHAT_ID}" \
  --data-urlencode "text=${MSG}" \
  -d disable_web_page_preview=true >/dev/null || true
