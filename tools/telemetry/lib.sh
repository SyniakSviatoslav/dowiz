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

# Sample host resources as a flat JSON string (no trailing newline issues).
# Keys: load1, load5, mem_pct, mem_used_mb, mem_total_mb, disk_pct, disk_free_gb, nproc.
# Best-effort; any missing /proc file degrades silently to null.
resource_sample() {
  local load1 load5 mem_pct mem_used mem_total disk_pct disk_free nproc
  load1="$(awk '{print $1}' /proc/loadavg 2>/dev/null || echo null)"
  load5="$(awk '{print $2}' /proc/loadavg 2>/dev/null || echo null)"
  nproc="$(nproc 2>/dev/null || echo null)"
  if [ -r /proc/meminfo ]; then
    mem_total="$(awk '/^MemTotal:/{print int($2/1024)}' /proc/meminfo)"
    mem_free="$(awk '/^MemAvailable:/{print int($2/1024)}' /proc/meminfo)"
    mem_used="$((mem_total - mem_free))"
    mem_pct="$(awk -v u="$mem_used" -v t="$mem_total" 'BEGIN{printf "%.1f", (t>0)?100*u/t:0}')"
  else
    mem_pct=null mem_used=null mem_total=null
  fi
  disk_pct="$(df -P / 2>/dev/null | awk 'NR==2{gsub("%","",$5);print $5}')"
  disk_free="$(df -Pm / 2>/dev/null | awk 'NR==2{print $4}')"
  printf '{"load1":%s,"load5":%s,"mem_pct":%s,"mem_used_mb":%s,"mem_total_mb":%s,"disk_pct":%s,"disk_free_gb":%s,"nproc":%s}' \
    "$load1" "$load5" "$mem_pct" "$mem_used" "$mem_total" "${disk_pct:-null}" "${disk_free:-null}" "$nproc"
}

# Run a command, measure wall-clock (ms) + peak RSS (MB via getrusage via `bash`/`time`),
# and emit a `bench` + `metric` event. Usage: bench_run <name> [note] -- <cmd...>
# Returns the command's exit code. Logs rx_rss_mb / rx_ms for real-time resource tracking.
bench_run() {
  local name="$1"; shift
  local note=""
  if [ "$1" != "--" ]; then note="$1"; shift; fi
  [ "$1" = "--" ] && shift
  local start_ms peak_kb rc out
  start_ms="$(date +%s%3N)"
  if command -v /usr/bin/time >/dev/null 2>&1; then
    # Capture combined output; rc is the child's exit (the time binary returns it).
    out="$(/usr/bin/time -v "$@" 2>&1)"; rc=$?
    peak_kb="$(printf '%s' "$out" | awk -F': ' '/Maximum resident set size/{gsub(/[^0-9]/,"",$2);print $2; exit}')"
  else
    "$@"; rc=$?
    peak_kb=0
  fi
  local end_ms; end_ms="$(date +%s%3N)"
  local ms; ms=$((end_ms - start_ms))
  local rss_mb=0; [ -n "$peak_kb" ] && [ "$peak_kb" != "0" ] && rss_mb=$((peak_kb / 1024))
  log_event bench "name=$name" "ms=$ms" "rss_mb=$rss_mb" "rc=$rc" "note=$(printf '%s' "$note" | _jesc)" >/dev/null
  log_event metric "kind=resource" "op=$name" "ms=$ms" "rss_mb=$rss_mb" >/dev/null
  if [ "${TELEMETRY_NO_TG:-0}" != "1" ]; then
    tg_send "⏱️ bench/$name ${ms}ms rss=${rss_mb}MB rc=$rc${note:+ | $note}" \
      || echo "telemetry: bench logged locally, Telegram send failed" >&2
  fi
  printf 'bench/%s ms=%s rss_mb=%s rc=%s\n' "$name" "$ms" "$rss_mb" "$rc"
  return "$rc"
}
