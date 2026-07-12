# Disaster Recovery Runbook (P32, P35)

## Deployment Context

**Current tier: Supabase Free** (single pilot location)

On Supabase Free tier:
- ❌ **No PITR** (Point-in-Time Recovery is Pro-only)
- ❌ No superuser access (cannot pause writes via SQL on Free — use app scale-down)
- ✅ R2 logical backups are the **sole recovery net**
- ✅ Restore-test runs every 24h to verify backup integrity
- ⚠ Auto-pause risk after ~1 week idle (mitigated by health endpoint keep-alive)

> **When upgrading to Pro:** PITR becomes available, RPO drops to seconds.
> Until then, RPO = backup interval (**1h** — the base cadence is hourly; see below).

## Recovery Targets (RTO/RPO)

> Cadence is **hourly** (`BACKUP_HOURLY_CRON`, plus daily/weekly/monthly retention tiers in
> `apps/api/src/workers/backup/index.ts`). This table and `docs/backup/runbooks.md` are the single
> source of truth and must agree — RPO = the hourly interval.

| Metric | Target (Free) | Target (Pro) | Measured |
|--------|--------------|--------------|----------|
| **RTO** | ≤ 4 hours | ≤ 2 hours | See DR drill report |
| **RPO** | ≤ 1 hour (hourly logical backup) | ≤ 5 min (PITR + logical) | See DR drill report |
| **Backup cadence** | Hourly (+ daily/weekly/monthly tiers) | Hourly + continuous WAL | `backup_metadata` table |
| **Restore-test** | Daily at 04:00 UTC | Daily at 04:00 UTC | `backup_audit_log` table |

## Scenarios

### Scenario A: Primary DB Corrupted (data loss < 1h)

**Triggers:** Health endpoint → 503 unhealthy, `backup.failed` event, Sentry alert.

> For a **full restore to a fresh instance**, the authoritative step-by-step (provision → migrate →
> `pnpm backup:restore --snapshot=<id> --confirm` → verify → cutover) lives in
> `docs/backup/runbooks.md` §3. The steps below are the Free-tier drill/cutover variant.

**Procedure:**

1. **Identify**
   ```bash
   # Check DB health
   curl https://api.dowiz.org/health | jq .checks.postgres
   
   # List recent completed backups
   pnpm backup:list --since=1h
   ```

2. **Pause writes** (manual — use Supabase dashboard or scale app to 0)
   ```bash
   # Option 1: Revoke INSERT/UPDATE from app role temporarily
   # (requires manual SQL via Supabase dashboard SQL editor)
   
   # Option 2: Scale app instances to 0 (stop processing)
   # flyctl scale count 0
   
   # Option 3: Set DB to read-only mode
   # (requires superuser — NOT available on Free tier)
   # Use Supabase dashboard → Database → Connection pooling → Pause app connections
   ```

3. **Select backup**
   ```bash
   # Latest verified backup
   pnpm backup:list --since=12h | head -5
   ```

4. **Restore to staging**
   ```bash
   pnpm backup:verify --backup-id=<ID>
   ```

5. **Verify**
   ```bash
   pnpm backup:verify --backup-id=<ID> --full-hash
   ```

6. **Cutover** (DNS/LB change — manual)
   - Point DATABASE_URL to restored staging
   - Verify: `pnpm backup:verify` (smoke-checks against staging)

7. **Resume writes**
   - Re-enable app roles
   - Scale app back up
   - Verify test order flow

8. **Verify customer-facing**
   - Place test order via /s/demo
   - Check /health returns healthy

9. **Post-mortem**
   - Generate DR drill report: `pnpm backup:drill --full`
   - Document root cause
   - Update runbook

### Scenario B: R2 Bucket Loss

**Triggers:** R2 health check degraded, `r2.lifecycle.drift` alert.

**Procedure:**

1. **Verify loss**
   ```bash
   curl https://api.dowiz.org/health | jq .checks.r2
   ```

2. **Re-establish R2**
   - Create new bucket via Cloudflare dashboard
   - Update env vars: `R2_ENDPOINT`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_BUCKET`
   - Deploy new config: `flyctl secrets set R2_*=...`

3. **Verify new bucket**
   ```bash
   curl https://api.dowiz.org/health | jq .checks.r2
   ```

4. **Notify**
   - "Historical backups unavailable, last X hours affected."
   - No data loss if backup rotation was healthy.

### Scenario C: Region Down (Fly.io)

**Triggers:** UptimeRobot alert, health endpoint unreachable.

**Procedure:**

1. Verify via UptimeRobot dashboard
2. If N=1: manual redeploy to another region
   ```bash
   flyctl regions add <region>
   flyctl deploy
   ```
3. If N>1: auto-failover (Fly.io native)
4. Verify: health endpoint /admin/health.html
5. Restore backup if needed (Scenario A)

### Scenario D: Auto-Pause (Free Tier)

**Triggers:** Health endpoint 503, first request after idle period.

**Procedure:**

1. **Wake the DB**: Any query triggers auto-wake (takes 2-5s)
   ```bash
   curl https://api.dowiz.org/health
   ```
2. **Verify**: Health endpoint returns 200 within 10s
3. **Resume**: Workers reconnect, caches warm, normal operations resume
4. **Prevent recurrence**: Ensure keep-alive health pings are active

## DR Drill

**Command:** `pnpm backup:drill --full`

**What it does:**
1. R2 sample verify (3 random manifests, checksum, schema)
2. Full restore-test (latest daily → sandbox → smoke-checks)
3. Generates dr-drill-report.md with actual timings

**Last tested:** See [dr-drill-report.md](../../.tmp/dr-drill-report.md)

**Drill cadence:** Every 30 days minimum.

## Communication

Owner-facing incident messages in [incident-comms.md](./incident-comms.md).

## Metrics

| Metric | Source |
|--------|--------|
| backup.verify.runs | Prometheus / Sentry |
| backup.verify.duration_ms | Sentry tags |
| backup.verify.smoke_check.passed/failed | Health endpoint |
| r2.verify.manifests_checked | R2 verify worker |
| r2.lifecycle.drift_detected | R2 verify worker |
| free_tier.db_pct | free-tier-watch worker |
| free_tier.storage_pct | free-tier-watch worker |
| free_tier.connections_pct | free-tier-watch worker |
