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

## Tier 2 — overnight deep-audit on a fresh clone (BUILT)

Nightly READ-ONLY audit routines run against a **throwaway `git clone`** (A6 — never
the working tree or live repo), aggregated into a **morning report** → log +
Telegram-ops. Durable via an external nightly cron + `claude -p` (A7).

**Files**
- `prompts/overnight-audit.md` — deep audit: dependency CVEs (`pnpm audit`), dead code (repowise), test-drift, doc-drift. Emits an `AUDIT:` report.
- `prompts/weekly-recap.md` — last-7-days shipped/churn/open-PR recap. Emits a `RECAP:` report.
- `tier2-overnight.sh [routine ...]` — the runner: fresh clone → series of read-only `claude -p` routines (no `--bare`, A3; read whitelist + Edit/Write disallowed, A4; Haiku + per-routine `--max-turns`, A5), **night budget cap** that skips remaining routines once exceeded (A5), OTel pass-through (A9), aggregate report → log + Telegram, throwaway clone always cleaned up (A7).

**Run manually**
```bash
scripts/automation/tier2-overnight.sh                  # overnight-audit + weekly-recap
scripts/automation/tier2-overnight.sh overnight-audit  # one routine
```

**Durable schedule (the VPS cron — A7)** — install in the box's crontab:
```cron
# 03:30 nightly audit (Mon–Sat) + Sunday adds the weekly recap → Telegram-ops
30 3 * * 1-6 cd /root/dowiz && TELEGRAM_OPS_CHAT_ID=<id> bash scripts/automation/tier2-overnight.sh overnight-audit >> scripts/automation/logs/cron.log 2>&1
30 3 * * 0   cd /root/dowiz && TELEGRAM_OPS_CHAT_ID=<id> bash scripts/automation/tier2-overnight.sh overnight-audit weekly-recap >> scripts/automation/logs/cron.log 2>&1
```

**Draft-PR (opt-in escalation — A2, default OFF).** T2 is read-only by default: it
proposes in a report, it does not act. A draft-PR is the gated next step, never
auto-merge: from the throwaway clone, branch → commit a single mechanical change →
`gh pr create --draft` for human review. It is intentionally NOT wired into the
runner (so the working tree stays trivially untouched); enable it only as a
deliberate, reviewed extension. **No tier ever merges to `main` (A2).**

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

## Invariant compliance (T2)
| | How |
|---|---|
| A1 dev/ops≠product | audits repo/lockfile/git only; no DB/PII/product-runtime access |
| A2 propose, not merge | read-only report; draft-PR is opt-in & default OFF; never auto-merge |
| A3 guardrails in automation | no `--bare` → `protect-paths`/`require-classification` hooks fire (clone has `.claude/`) |
| A4 explicit perms | read whitelist + `--disallowed-tools Edit Write NotebookEdit`; no `--dangerously-skip-permissions` |
| A5 budget | Haiku tier + per-routine `--max-turns` + **night `$` cap** (skips remaining routines); `timeout 900` wall-clock |
| A6 fresh clone | `mktemp -d` throwaway clone, runs inside it, `trap` cleanup on exit |
| A7 durable | external nightly cron + `claude -p`; idempotent (per-run clone, dated logs) |
| A8 read-mostly | T2 is read + analysis + report; mutation (draft-PR) is gated & off |
| A9 observability | per-day log + aggregated Telegram-ops report; OTel when endpoint set |

## Tier 3 (NOT built — gated)
- **T3 `/batch`**: bounded fan-out (≤ a few subagents) for mechanical sweeps only, each diff adversarially reviewed (read-only reviewer), result = PR via OpenSpec `propose→apply` for human approval. Build last (highest risk/cost), after the T2 gate.
