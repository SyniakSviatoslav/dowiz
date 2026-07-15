#!/usr/bin/env bash
# telemetry lib — core primitives shared by all subcommands. Source, don't exec.
# Zero deps beyond curl + coreutils. Secrets come from dowiz/.env (gitignored).
set -uo pipefail

TELEMETRY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$TELEMETRY_DIR/../.." && pwd)"
LOG_DIR="${TELEMETRY_LOG_DIR:-$REPO_ROOT/tools/telemetry/logs}"
# Non-secret. Override with TELEGRAM_CHAT_ID env. Default = "Dowiz-Reporting".
CHAT_ID="${TELEGRAM_CHAT_ID:--1003901655568}"

# Load bot token from .env without echoing it.
_load_token() {
  [ -n "${TELEGRAM_BOT_TOKEN:-}" ] && return 0
  if [ -f "$REPO_ROOT/.env" ]; then
    TELEGRAM_BOT_TOKEN="$(grep -E '^TELEGRAM_BOT_TOKEN=' "$REPO_ROOT/.env" | head -1 | cut -d= -f2- | tr -d '\r"'"'"'')"
  fi
  [ -n "${TELEGRAM_BOT_TOKEN:-}" ]
}

_now()      { date -u +%Y-%m-%dT%H:%M:%SZ; }
_host()     { hostname -s 2>/dev/null || echo host; }

# JSON-escape one string value (stdin -> stdout, no surrounding quotes).
_jesc() { python3 -c 'import json,sys;print(json.dumps(sys.stdin.read())[1:-1])'; }

# Append one structured event to a per-kind JSONL ledger.
# args: <kind> <k=v>...  (values may contain spaces)
log_event() {
  local kind="$1"; shift
  mkdir -p "$LOG_DIR"
  local f="$LOG_DIR/${kind}.jsonl"
  local ts host line
  ts="$(_now)"; host="$(_host)"
  line="{\"ts\":\"$ts\",\"kind\":\"$kind\",\"host\":\"$host\""
  local kv k v
  for kv in "$@"; do
    k="${kv%%=*}"; v="${kv#*=}"
    line="$line,\"$k\":\"$(printf '%s' "$v" | _jesc)\""
  done
  line="$line}"
  printf '%s\n' "$line" >> "$f"
  printf '%s\n' "$line"   # echo back for piping/inspection
}

# Send a plain-text message to Telegram with a 3-try retry loop.
# Returns 0 on ok:true, 1 otherwise. Never prints the token.
tg_send() {
  local text="$1"
  if ! _load_token; then
    echo "tg_send: no TELEGRAM_BOT_TOKEN (set env or dowiz/.env)" >&2
    return 1
  fi
  local url="https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage"
  local attempt resp
  for attempt in 1 2 3; do
    resp="$(curl -sS --max-time 10 "$url" \
      --data-urlencode "chat_id=${CHAT_ID}" \
      --data-urlencode "text=${text}" \
      --data-urlencode "disable_web_page_preview=true" 2>&1)"
    if printf '%s' "$resp" | grep -q '"ok":true'; then
      return 0
    fi
    sleep 2
  done
  echo "tg_send: failed after 3 attempts: $resp" >&2
  return 1
}
