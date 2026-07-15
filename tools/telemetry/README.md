# tools/telemetry — Dowiz telemetry → Telegram bridge

Self-contained, zero-dep (curl + coreutils) telemetry stream. Every event is
written to a local JSONL ledger **and** pushed to the `Dowiz-Reporting` Telegram
channel. Complements the on-box `ops-gatus` uptime monitor and the GitHub Actions
`heartbeat-monitor.yml` dead-man's-switch.

## What it streams
- **internal logs** — `info` / `warn` / `error`
- **metrics** — named numeric values with a unit
- **benchmarks** — named timings in ms
- **tasks** — `started` / `done` / `failed` with a title
- **sessions** — a completed agent session summary

## Usage
```bash
tools/telemetry/telemetry log info "webhook redeployed"
tools/telemetry/telemetry metric disk_free_gb 7.1 GB
tools/telemetry/telemetry bench kernel_test 1840 "cargo test kernel"
tools/telemetry/telemetry task done "neutralize Claude gates + push"
tools/telemetry/telemetry session "gate removal + telemetry bridge shipped"
tools/telemetry/telemetry send "ad-hoc line"
tools/telemetry/telemetry selftest        # local-only, no network
```

## Secrets & config
- `TELEGRAM_BOT_TOKEN` — read from `dowiz/.env` (gitignored) or the environment.
  Never committed, never echoed. Same secret is deployed as the GitHub Actions
  repo secret for `heartbeat-monitor.yml`.
- `TELEGRAM_CHAT_ID` — non-secret; defaults to `-1003901655568` (Dowiz-Reporting).
- `TELEMETRY_NO_TG=1` — write the local ledger only, skip the Telegram send.
- `TELEMETRY_LOG_DIR` — override the ledger dir (default `tools/telemetry/logs/`).

## Ledger
`tools/telemetry/logs/<kind>.jsonl` — one JSON object per line:
`{"ts","kind","host",<fields...>}`. The `logs/` dir is gitignored.

## Design (innovate: minimal)
One `lib.sh` (log_event + tg_send with 3-try retry) + one `telemetry` dispatcher.
No daemon, no queue, no external deps — ceiling: if send volume ever needs
batching/backpressure, upgrade `tg_send` to a spooled sender. Trigger: >1 msg/s.
