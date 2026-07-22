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

# --- spool-based Telegram delivery (HK-11, 2026-07-15) ---
# The FAST critical path: append one JSON line to a spool (microseconds) and
# return immediately. A background Rust binary (telemetry-spool) drains it at
# the kernel-derived 3.5s pace, so the agent's work is NEVER blocked behind
# the network. With the drainer down, tg_deliver falls back to synchronous
# tg_send so reporting never goes silent.

# Ensure exactly one spool drainer is running. Returns 0 if the drainer can run.
tg_spool_ensure() {
  pgrep -f 'target/release/telemetry-spool' >/dev/null 2>&1 && return 0
  local bin="$TELEMETRY_DIR/rust-spool/target/release/telemetry-spool"
  if [ -x "$bin" ] && _load_token; then
    TELEGRAM_BOT_TOKEN="$TELEGRAM_BOT_TOKEN" nohup "$bin" >/tmp/telemetry-spool/drainer.log 2>&1 &
    return 0
  fi
  return 1
}

# Append a message to the spool (topic defaults to HERMES 267).
tg_spool() {
  local text="$1" topic="${2:-${TELEGRAM_TOPIC_ID:-267}}"
  local spool="/tmp/telemetry-spool/queue.jsonl"
  mkdir -p "$(dirname "$spool")"
  local esc
  esc="$(printf '%s' "$text" | _jesc)"
  printf '{"chat_id":"%s","topic_id":%s,"text":"%s"}\n' "$CHAT_ID" "$topic" "$esc" >> "$spool"
}

# Deliver to Telegram: spool (fast) when possible, else sync fallback.
# Honors TELEMETRY_NO_TG:1 everywhere (never touches network or spool).
tg_deliver() {
  local text="$1" topic="${2:-${TELEGRAM_TOPIC_ID:-267}}"
  [ "${TELEMETRY_NO_TG:-0}" = "1" ] && return 0
  if tg_spool_ensure 2>/dev/null; then
    tg_spool "$text" "$topic"
  else
    _tg_deliver_alerted "$text" "$topic"
  fi
}

_tg_deliver_alerted() {
  local text="$1" topic="$2"
  local resp rc
  resp="$(tg_send "$text" "$topic" 2>&1)"; rc=$?
  log_event alert "channel=telegram" "succ=$([ "$rc" -eq 0 ] && echo 1 || echo 0)" "resp=$(printf '%s' "$resp" | sed 's/^ //')" >/dev/null 2>&1 || true
  [ "$rc" -eq 0 ] && return 0
  # Transient recovery: bounded inexpensive retries for likely-temporary failures.
  local attempt
  for attempt in 1 2 3; do
    sleep "$((2 ** attempt))"
    resp="$(tg_send "$text" "$topic" 2>&1)"; rc=$?
    log_event alert "channel=telegram" "succ=$([ "$rc" -eq 0 ] && echo 1 || echo 0)" "resp=$(printf '%s' "$resp" | sed 's/^ //')" "attempt=$attempt" >/dev/null 2>&1 || true
    [ "$rc" -eq 0 ] && return 0
  done
  return "$rc"
}

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

# Send a plain-text message to Telegram.
# Rate control: (1) a global minimum inter-message gap (TG_MIN_GAP, default 3.5s) so bulk
# posters (e.g. plans batches) self-space and never trip the forum ~20msg/min limit; (2) a
# retry loop that honors Telegram's 429 retry_after via bounded backoff. Returns 0 on ok:true.
# Never prints the token. Honors TELEGRAM_TOPIC_ID (forum topic / message_thread_id).
# Default hermes topic = 267.
tg_send() {
  local text="$1"
  if ! _load_token; then
    echo "tg_send: no TELEGRAM_BOT_TOKEN (set env or dowiz/.env)" >&2
    return 1
  fi
  # global throttle: ensure >= TG_MIN_GAP seconds since last actual successful send.
  # Uses an atomic flock so the 6 concurrent daemons can't all pass the check at
  # once (race fix). The timestamp is written ONLY after a real send succeeds.
  local gap="${TG_MIN_GAP:-3.5}"
  local statef="/tmp/.tg_send_last"
  local lockf="/tmp/.tg_send_lock"
  exec 9>"$lockf" 2>/dev/null || true
  flock 9 2>/dev/null || true
  local now last delta
  now="$(date +%s.%N 2>/dev/null || date +%s)"
  if [ -f "$statef" ]; then
    last="$(cat "$statef" 2>/dev/null)"
    delta="$(awk -v n="$now" -v l="$last" 'BEGIN{d=n-l; if(d<0)d=0; printf "%.2f", d}')"
    if awk -v d="$delta" -v g="$gap" 'BEGIN{exit !(d<g)}'; then
      sleep "$(awk -v d="$delta" -v g="$gap" 'BEGIN{printf "%.2f", g-d}')"
    fi
  fi
  # do NOT stamp here; stamp after a confirmed-OK send below.

  local url="https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage"
  local thread="${TELEGRAM_TOPIC_ID:-267}"
  local attempt resp sleep_s=2
  for attempt in 1 2 3 4 5 6; do
    resp="$(curl -sS --max-time 10 "$url" \
      --data-urlencode "chat_id=${CHAT_ID}" \
      --data-urlencode "text=${text}" \
      --data-urlencode "disable_web_page_preview=true" \
      ${thread:+--data-urlencode "message_thread_id=${thread}"} 2>&1)"
    if printf '%s' "$resp" | grep -q '"ok":true'; then
      date +%s.%N 2>/dev/null > "$statef" || date +%s > "$statef"
      exec 9>&- 2>/dev/null || true
      return 0
    fi
    # honor 429 retry_after if present, else exponential backoff (cap 30s)
    local ra
    ra="$(printf '%s' "$resp" | grep -o '"retry_after":[0-9]*' | grep -o '[0-9]*$')"
    if [ -n "$ra" ]; then sleep_s="$ra"; else sleep_s=$((sleep_s * 2)); fi
    [ "$sleep_s" -gt 30 ] && sleep_s=30
    sleep "$sleep_s"
  done
  echo "tg_send: failed after retries: $resp" >&2
  exec 9>&- 2>/dev/null || true
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
  log_event bench "name=$name" "ms=$ms" "rss_mb=$rss_mb" "rc=$rc" "note=$note" >/dev/null
  # TORVALDS-09 fix: (a) drop the pre-_jesc — log_event already escapes each
  # value once (double-escaping produced a doubly-escaped payload); (b) the
  # metric kind is emitted positionally by log_event as "kind":"metric", so a
  # literal "kind=resource" here created a DUPLICATE key (last-wins clobbered
  # the metric kind). Rename the dimension to "rt=resource" instead.
  log_event metric "rt=resource" "op=$name" "ms=$ms" "rss_mb=$rss_mb" >/dev/null
  if [ "${TELEMETRY_NO_TG:-0}" != "1" ]; then
    tg_send "⏱️ bench/$name ${ms}ms rss=${rss_mb}MB rc=$rc${note:+ | $note}" \
      || echo "telemetry: bench logged locally, Telegram send failed" >&2
  fi
  printf 'bench/%s ms=%s rss_mb=%s rc=%s\n' "$name" "$ms" "$rss_mb" "$rc"
  return "$rc"
}

# ---- spec-driven DOD reporting helpers (2026-07-15) ----
_plan_dir() { echo "$LOG_DIR"; }

# latest done count for a plan id from plan_step.jsonl
_plan_latest_step() {
  local id="$1"
  grep -F "\"id\":\"$id\"" "$LOG_DIR/plan_step.jsonl" 2>/dev/null \
    | python3 -c 'import sys,json
rows=[json.loads(l) for l in sys.stdin if l.strip()]
print(rows[-1]["done"] if rows else 0)' 2>/dev/null || echo 0
}
_plan_total() {
  local id="$1"
  grep -F "\"id\":\"$id\"" "$LOG_DIR/plan_step.jsonl" 2>/dev/null \
    | python3 -c 'import sys,json
rows=[json.loads(l) for l in sys.stdin if l.strip()]
print(rows[-1].get("total",0) if rows else 0)' 2>/dev/null || echo 0
}
# get a field from the plan.jsonl row for id
_plan_get() {
  local id="$1" field="$2"
  grep -F "\"id\":\"$id\"" "$LOG_DIR/plan.jsonl" 2>/dev/null \
    | python3 -c 'import sys,json
for l in sys.stdin:
    if l.strip():
        d=json.loads(l)
        if d.get("id")=="'"$id"'":
            print(d.get("'"$field"'","")); break' 2>/dev/null
}
# open (unresolved) alert count for id
_plan_open_alerts() {
  local id="$1"
  grep -F "\"id\":\"$id\"" "$LOG_DIR/alert.jsonl" 2>/dev/null \
    | python3 -c 'import sys,json
rows=[json.loads(l) for l in sys.stdin if l.strip()]
# unresolved = resolved not truthy (handles bool false AND string "false" from log_event)
def is_resolved(r):
    v=r.get("resolved",False)
    return str(v).lower() in ("true","1")
opened=[r for r in rows if not is_resolved(r)]
print(len(opened))' 2>/dev/null || echo 0
}
# rolling mean abs ETA-error % across all finalized trajectory rows (improvement/degradation)
_plan_rolling_acc() {
  python3 -c 'import json
try:
    rows=[json.loads(l) for l in open("'"$LOG_DIR"'/trajectory.jsonl") if l.strip()]
except FileNotFoundError:
    print(""); raise SystemExit
if not rows: raise SystemExit
acc=[abs(r.get("eta_err_pct",0)) for r in rows if "eta_err_pct" in r]
print(round(sum(acc)/len(acc),1) if acc else "")' 2>/dev/null || echo ""
}
