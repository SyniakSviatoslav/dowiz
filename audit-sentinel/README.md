# Audit Sentinel

Autonomous dual-tier audit agent for dowiz/DeliveryOS.

## Architecture

- **Tier 1 · Watchdog** — Deterministic probes every 5-15 min (TLS, headers, /health, cache, rate-limit). No LLM, ~$0 cost.
- **Tier 2 · Auditor** — Full LLM audit on triggers (post-deploy, nightly, anomaly). Runs the complete deployed-service audit prompt.

## Quick start

```bash
cd audit-sentinel
corepack enable
pnpm install

# Run watchdog against staging
ENV=staging BASE_URL=https://dowiz.fly.dev pnpm watchdog

# Run watchdog against prod (observe-only)
ENV=prod BASE_URL=https://app.dowiz.org pnpm watchdog
```

## Environment variables

| Variable | Required | Description |
|----------|----------|-------------|
| `BASE_URL` | Yes | Deployed app URL |
| `ENV` | Yes | `staging` or `prod` |
| `MENU_URL` | No | Menu SSR URL for cache probes |
| `SITE_URL` | No | Static site URL |
| `TEST_TENANT` | No | Tenant slug (default: `demo`) |
| `TELEGRAM_BOT_TOKEN` | No | For ops alerts |
| `TELEGRAM_CHAT_ID` | No | For ops alerts |
| `AUDIT_AGENT_ENABLED` | No | Kill-switch (default: `true`) |

## CI

Two GitHub Actions workflows:

- `.github/workflows/watchdog.yml` — Tier-1, every 15 min
- `.github/workflows/nightly-audit.yml` — Tier-2, nightly + post-deploy + on anomaly

## Read-only safety

- Zero write access to the deployed system
- Only test tenant credentials
- Same safe-on-live protocol as the manual audit prompt
- Network allowlist: only dowiz.org, fly.dev, Anthropic API, Telegram API, GitHub API
