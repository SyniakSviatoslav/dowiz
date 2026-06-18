# dowiz — Agent-Automation Subsystem (dev/ops, 3-tier)

> **dev/ops, not product.** Operates on repo/code. Never touches the product
> runtime, N=1, or customer PII (A1). AI proposes — a human approves (A2). One
> tier at a time: **T1 → gate → T2 → gate → T3**.

## Tier 1 — scheduled read-only ops (BUILT)

Light recurring tasks (deploy-watch, health, CI triage) with **zero mutation**.
Durable via an **external cron + `claude -p`** (A7: CC's in-session schedules die
with the session; cron survives restarts).

**Files**
- `prompts/ops-watch.md` — the read-only watch task (deploy drift, prod health, CI, commits).
- `tier1-run.sh <name>` — the runner: headless `claude -p`, **no `--bare`** (A3 — hooks/guardrails apply), read-only tool whitelist + Edit/Write disallowed (A4), Haiku + `--max-turns` budget cap (A5), OTel pass-through (A9), report → log + Telegram-ops.
- `notify.sh` — posts the report to Telegram-ops.
- `logs/` — per-day run logs.

**Run manually**
```bash
scripts/automation/tier1-run.sh ops-watch
```

**Durable schedule (the VPS cron — A7)** — add to the box's crontab, NOT a CC session:
```cron
# every 30 min, read-only deploy/health/CI watch → Telegram-ops
*/30 * * * * cd /root/dowiz && TELEGRAM_OPS_CHAT_ID=<id> bash scripts/automation/tier1-run.sh ops-watch >> scripts/automation/logs/cron.log 2>&1
```

## What you must provide (infra inputs)
| Var | For | Status |
|-----|-----|--------|
| `TELEGRAM_OPS_CHAT_ID` | report destination (A9) | **needed** (bot token already in `.env`) |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | Grafana trace export (A9) | optional — omit to skip |
| crontab entry on the VPS | durability (A7) | **you install** |

## Invariant compliance (T1)
| | How |
|---|---|
| A1 dev/ops≠product | checks repo/CI/prod-HTTP only; no DB/PII writes |
| A2 propose, not merge | read-only; emits a report, never commits/merges |
| A3 guardrails in automation | no `--bare` → `guard-bash`/`protect-paths`/`require-classification` hooks fire |
| A4 explicit perms | `--allowed-tools` read whitelist + `--disallowed-tools Edit Write`; no `--dangerously-skip-permissions` |
| A5 budget | Haiku tier + `--max-turns` cap; `timeout 600` wall-clock |
| A7 durable | external cron + `claude -p`, not a CC session |
| A8 read-mostly | T1 is pure read + report |
| A9 observability | per-day log + Telegram-ops; OTel when endpoint set |

## Tiers 2 & 3 (NOT built — gated)
- **T2 overnight**: VPS cron → fresh `git clone` (A6) → `claude -p` audit routines → morning report + optional **draft-PR** (never auto-merge). Build after the T1 gate.
- **T3 `/batch`**: bounded fan-out (≤ a few subagents) for mechanical sweeps only, each diff adversarially reviewed (read-only reviewer), result = PR for human approval. Build last (highest risk/cost), after the T2 gate.
