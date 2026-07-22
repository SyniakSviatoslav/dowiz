#!/usr/bin/env bash
# hydra_metrics_sender.sh — non-blocking 5-minute Hydra metrics reporter.
# Reads live JSONL artifacts from tools/telemetry/logs.
# Usage: ./hydra_metrics_sender.sh
# Cron: */5 * * * * /root/dowiz/hydra_metrics_sender.sh >/tmp/hydra_metrics_sender.log 2>&1
set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
METRICS_DIR="$REPO/tools/telemetry/logs"
HY_JSONL="$METRICS_DIR/hydra_closed_loop.jsonl"
KM_JSONL="$METRICS_DIR/kernel_metrics.jsonl"
SPAN_JSONL="$METRICS_DIR/metric.jsonl"
TRACK="$REPO/track_record.jsonl"

ts() { date -u +%Y-%m-%dT%H:%M:%SZ; }
trim_path() { printf '%s' "$1" | sed "s|$REPO/||"; }

build_msg() {
  cat <<'BUNDLE'
HYDRA FULL METRICS SUITE
Sections: system // organism // closed_loop // entropy // anneal // branch // kalman // dmd // token // scheduler // temporal_tmr // typed_probes // span_histograms // breach // tests
BUNDLE

  if [ -f "$HY_JSONL" ]; then
    echo "— organism + commit (latest) —"
    tail -n 2 "$HY_JSONL" 2>/dev/null | while IFS= read -r row; do
      [ -n "$row" ] && echo "  $row"
    done || true
  else
    echo "— organism + commit: no hydra_closed_loop.jsonl —"
  fi

  if [ -f "$KM_JSONL" ]; then
    echo "— tests (latest test_suite rows) —"
    grep '"kind":"test_suite"' "$KM_JSONL" 2>/dev/null | tail -n 20 | while IFS= read -r row; do
      [ -n "$row" ] && echo "  $row"
    done || true
  else
    echo "— tests: no kernel_metrics.jsonl —"
  fi

  echo "— typed probes (P08) —"
  if [ -f "$METRICS_DIR/health.jsonl" ]; then
    tail -n 3 "$METRICS_DIR/health.jsonl" 2>/dev/null | while IFS= read -r row; do
      [ -n "$row" ] && echo "  $row"
    done || true
  fi

  echo "— runtime probes —"
  echo "  token_bucket: live values unavailable from static sender; add kernel/src/bin/hydra_runtime_probe.rs for runtime exposure"
  echo "  scheduler: runtime values only via probe tool"
  echo "  temporal_tmr: runtime values only via probe tool"

  if [ -f "$SPAN_JSONL" ]; then
    echo "— span histograms (P83 last 12 rows) —"
    tail -n 12 "$SPAN_JSONL" 2>/dev/null | while IFS= read -r row; do
      [ -n "$row" ] && echo "  $row"
    done || true
  else
    echo "— span histograms: no metric.jsonl —"
  fi

  echo "— files —"
  for f in "$HY_JSONL" "$KM_JSONL" "$SPAN_JSONL"; do
    if [ -f "$f" ]; then
      sz=$(stat -c%s "$f" 2>/dev/null || stat -f%z "$f" 2>/dev/null || echo 0)
      echo "  $(trim_path "$f"): ${sz}B"
    fi
  done
  echo "ts=$(ts)"
}

# Non-blocking sender: print to stdout for local cron, try Telegram after.
msg="$(build_msg)"
[ -n "$msg" ] || { echo "metrics sender: no data available"; exit 0; }
printf '%s\n' "$msg"

# Optional Telegram delivery when lib.sh is present and not disabled.
if [ -f "$REPO/tools/telemetry/lib.sh" ] && [ "${TELEMETRY_NO_TG:-0}" != "1" ]; then
  if command -v tg_deliver >/dev/null 2>&1; then
    _tg_deliver_alerted "$msg" 267 >/dev/null 2>&1 || true
  fi
fi
