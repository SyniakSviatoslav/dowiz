# Free Tier Operations Guide (G3)

## Supabase Free Tier Limits

| Resource | Free Limit | 80% Warning | Critical |
|----------|-----------|-------------|----------|
| Database size | 500 MB | 400 MB | 475 MB |
| Row count | Infinite (but perf degrades) | — | — |
| Storage (assets) | 1 GB | 800 MB | 950 MB |
| Egress (monthly) | 2 GB | 1.6 GB | 1.9 GB |
| Connections (pooler) | ~15 (soft) | 12 | 14 |
| API requests/day | No hard limit | — | — |
| Auto-pause | After ~1 week idle | — | — |

## Monitoring (free-tier-watch)

A pg-boss scheduled worker runs **every hour** and checks:

1. **DB size** — `SELECT pg_database_size(current_database())`
2. **Storage** — Sum of file sizes from `upload_audit` table
3. **Active connections** — `SELECT count(*) FROM pg_stat_activity`
4. **Egress estimate** — Tracked via `ops_worker_heartbeat` request counts (rough proxy)

At **80% of any limit**, the worker:
- Logs a `degraded` status to `ops_worker_heartbeat` (worker_id: `free-tier-watch`)
- Publishes `free_tier.warning` on MessageBus → Telegram alert to owner
- Stores the metric snapshot in `free_tier_snapshots` table

At **95% of any limit**, the worker:
- Publishes `free_tier.critical` on MessageBus
- Health endpoint reports `degraded` for the `free_tier` check
- Triggers a recommended upgrade flow

## Health Endpoint

The `/health` endpoint includes a `free_tier` check:
- `ok` — all metrics below 80%
- `degraded` — one or more metrics above 80%
- `down` — one or more metrics above 100%

## Upgrade Trigger

When any metric crosses **80%**:

1. Console alert + Telegram notification sent
2. Owner reviews the upgrade options:
   - **Pro tier** ($25/mo): PITR, 8 GB DB, 100 GB storage, 250 GB egress
   - **Team tier** ($75/mo): everything above + SOC2, daily backups, priority support
3. Upgrade is **non-destructive** — no data loss, no downtime
4. After upgrade: update `verify-launch.ts` gate to expect Pro tier

## Keep-Alive

Free tier auto-pauses after ~1 week of inactivity. Mitigation:

- **Health endpoint ping**: Every 5 minutes via UptimeRobot → keeps DB awake
- **Worker heartbeat**: All pg-boss workers keep periodic connections
- **Backup schedule**: Even if paused, backup job will trigger → wakes DB

If auto-pause occurs:
1. Next health check → 503 → UptimeRobot alert → manual wake
2. Wake: `curl https://api.dowiz.org/health` → auto-wakes within 5s
3. Recovery: warm caches, workers reconnect, resume normal operations

## Verification

```bash
# Check current free tier metrics
curl https://api.dowiz.org/health | jq .checks.free_tier

# Run free-tier-watch manually
pnpm free-tier:watch
```
