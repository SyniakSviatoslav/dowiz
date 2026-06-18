#!/usr/bin/env bash
# notify.sh — post an ops message to Telegram-ops (best-effort; never fails caller).
set -euo pipefail
MSG="${1:?usage: notify.sh <message>}"
[ -n "${***REDACTED***:-}" ] && [ -n "${TELEGRAM_OPS_CHAT_ID:-}" ] || {
  echo "[notify] ***REDACTED*** / TELEGRAM_OPS_CHAT_ID not set — skipping" >&2; exit 0; }
curl -s -X POST "https://api.telegram.org/bot${***REDACTED***}/sendMessage" \
  --data-urlencode "chat_id=${TELEGRAM_OPS_CHAT_ID}" \
  --data-urlencode "text=${MSG}" \
  -d disable_web_page_preview=true >/dev/null || true
