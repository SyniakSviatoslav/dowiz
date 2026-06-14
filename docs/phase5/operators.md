# Phase 5 — Operator Runbook (Stage 30)

## 1. Manual Retention Cron Trigger

### Via pg-boss

```sql
-- Schedule an immediate one-shot execution
SELECT pg_boss.schedule(
  'anonymizer.retention',
  'now',
  NULL,
  jsonb_build_object('singletonKey', 'anonymizer.retention')
);
```

This bypasses the cron schedule and runs the retention batch immediately. The `singletonKey` prevents concurrent execution.

### Via Env Trigger

Restart the API server with:

```env
TRIGGER_RETENTION_ON_STARTUP=true
```

On startup, the server will execute one retention cycle before accepting requests. Useful after a deployment to catch any backlog.

### Verify Execution

```sql
-- Check pg-boss archive for completion
SELECT * FROM pg_boss.archive
WHERE name = 'anonymizer.retention'
ORDER BY completed_on DESC
LIMIT 5;
```

```sql
-- Check if customers were anonymized
SELECT count(*) AS anonymized_today
FROM customers
WHERE anonymized_at IS NOT NULL
  AND anonymized_at::date = CURRENT_DATE;
```

---

## 2. Mass-Anonymize a Customer (Post-Data-Breach)

Use this procedure when a specific customer requests erasure outside the normal flow, e.g., after a data breach notification.

### Step 1: Identify the Customer

```sql
SELECT id, name, phone, email, anonymized_at
FROM customers
WHERE phone = '+3556Xxxxxxxx';  -- E.164 format
```

- If `anonymized_at IS NOT NULL` → customer is already anonymized. Verify and stop.
- If no rows returned → customer not found in this tenant. Check phone format.

### Step 2: Create GDPR Request

```http
POST /api/owner/locations/{locationId}/gdpr-requests
Content-Type: application/json
Authorization: Bearer {owner-jwt}

{
  "customerId": "uuid-from-step-1"
}
```

Expected response: `201 Created` with `status: "pending"`.

If `409 Conflict`: a request already exists for this customer in the last 24h. Check existing request:

```sql
SELECT id, status, created_at, error_message
FROM gdpr_erasure_requests
WHERE customer_id = 'uuid'
ORDER BY created_at DESC
LIMIT 1;
```

### Step 3: Monitor Processing

```sql
SELECT status, completed_at, error_message
FROM gdpr_erasure_requests
WHERE customer_id = 'uuid'
ORDER BY created_at DESC
LIMIT 1;
```

Run every 10 seconds. Expected progression: `pending` → `processing` → `completed`.

If status stays `pending` > 30s, check pg-boss:

```sql
SELECT * FROM pg_boss.job
WHERE name = 'gdpr.erasure'
  AND data->>'customerId' = 'uuid'
ORDER BY created_on DESC;
```

### Step 4: Verify Anonymization

```sql
SELECT id, name, phone, email, avatar_url, anonymized_at, anonymized_by
FROM customers
WHERE id = 'uuid';
```

Expected result:
| Column | Expected |
|--------|----------|
| `name` | `NULL` |
| `phone` | `anon-{12 chars}` |
| `email` | `NULL` |
| `avatar_url` | `NULL` |
| `anonymized_at` | Non-NULL timestamp |
| `anonymized_by` | `'gdpr_erasure'` |

### Step 5: Check Audit Trail

```sql
SELECT id, subject_type, trigger, fields_cleared, created_at
FROM anonymization_audit_log
WHERE subject_id = 'uuid'
ORDER BY created_at ASC;
```

Export this as evidence for compliance:

```bash
# Using psql
psql $DATABASE_URL -c "\COPY (
  SELECT id, subject_type, trigger, fields_cleared, performed_by, created_at
  FROM anonymization_audit_log
  WHERE subject_id = 'uuid'
  ORDER BY created_at ASC
) TO 'compliance-export-{uuid}.csv' WITH CSV HEADER;"
```

---

## 3. Export Audit Log for Compliance

### Full Export

```sql
SELECT * FROM anonymization_audit_log
WHERE created_at > '2026-01-01'
ORDER BY created_at ASC;
```

Export to CSV:

```bash
psql $DATABASE_URL -c "\COPY (
  SELECT id, subject_type, subject_id, trigger,
         array_to_string(fields_cleared, ',') AS fields_cleared,
         performed_by, created_at
  FROM anonymization_audit_log
  WHERE created_at > '2026-01-01'
  ORDER BY created_at ASC
) TO 'audit-log-export-2026.csv' WITH CSV HEADER;"
```

### Verify PII-Free

Run the PII leak detector on the exported CSV:

```bash
npx pii-leak-detector audit-log-export-2026.csv
```

Expected: 0 PII leaks found.

If leaks are found:
1. Identify the row and column with PII.
2. Check if the app layer wrote PII to the audit log (bug).
3. File a critical incident and patch the source.
4. The leaked data is already in the audit log — it cannot be edited (append-only). Document the leak in the compliance report.

### Periodic Export

Recommended schedule:
- Monthly: full export to encrypted S3 bucket for compliance team
- Quarterly: verify PII-free + cross-reference with consumer erasure requests
- Annually: submit to DPA (Data Protection Authority) if requested

---

## 4. R2 Retention Drift Alert

### Detection

The application emits a warning on startup if:

```
R2_RETENTION_OVERRIDE_DAYS < max(locations.retention_days)
```

Monitor for the log line:

```
WARN [retention] R2 retention override ({override}d) is less than max DB retention ({maxDb}d). R2 will expire data before DB anonymization window.
```

### Manual Cleanup Procedure

If R2 max effective retention > DB max retention (orphan PII risk):

#### Step 1: Check Current R2 Lifecycle Rules

```bash
# Using wrangler (Cloudflare R2)
npx wrangler r2 bucket lifecycle --bucket deliveryos-backups
```

#### Step 2: Calculate Required Override

```sql
SELECT max(retention_days) AS max_db_retention FROM locations;
```

Set `R2_RETENTION_OVERRIDE_DAYS` to this value (or lower).

#### Step 3: Apply New Lifecycle Policy

```bash
# Set override env on all API instances
flyctl secrets set R2_RETENTION_OVERRIDE_DAYS=365

# Restart to apply
flyctl deploy --no-build
```

#### Step 4: Verify

Check logs for:

```
INFO [retention] R2 retention override applied: 365 days
```

#### Step 5: Clean Up Orphaned Objects (if needed)

If backups already exist past the new window, manually delete:

```bash
# List old monthly backups
npx wrangler r2 object list --bucket deliveryos-backups --prefix monthly/ | grep "2025"

# Delete objects older than override
npx wrangler r2 object delete --bucket deliveryos-backups --prefix monthly/2025-01-*
```

---

## 5. Configure Retention Per Location

### Via UI

1. Navigate to `/admin/settings/retention`
2. Select location from dropdown (if multi-location owner)
3. Set `retentionDays` slider or input (range: 30–3650, step: 30)
4. Click Save
5. Confirm dialog: "This will update the anonymization schedule for all customers at this location. Existing customers will be anonymized on their new due date."

### Via API

```http
PUT /api/owner/locations/{locationId}/settings/retention
Content-Type: application/json
Authorization: Bearer {owner-jwt}

{
  "retentionDays": 180
}
```

Response: `200 OK` with updated settings.

### Via SQL (Emergency)

```sql
-- Check current value
SELECT id, name, retention_days FROM locations WHERE id = 'uuid';

-- Update
UPDATE locations SET retention_days = 180 WHERE id = 'uuid';

-- Verify
SELECT id, name, retention_days FROM locations WHERE id = 'uuid';

-- Recalculate due dates for all customers at this location
UPDATE customers
SET anonymization_due_at = created_at + INTERVAL '1 day' * 180
WHERE location_id = 'uuid'
  AND anonymized_at IS NULL;
```

### Mass Update for All Locations

```sql
-- Set all locations to 365 days
UPDATE locations SET retention_days = 365;

-- Recalculate all customer due dates (heavy — run during maintenance window)
UPDATE customers c
SET anonymization_due_at = c.created_at + INTERVAL '1 day' * l.retention_days
FROM locations l
WHERE c.location_id = l.id
  AND c.anonymized_at IS NULL;
```

---

## Quick Reference

| Task | Command/Endpoint |
|------|-----------------|
| Trigger retention cron | `SELECT pg_boss.schedule('anonymizer.retention', 'now', NULL, '{"singletonKey":"anonymizer.retention"}');` |
| Create GDPR request | `POST /api/owner/locations/:id/gdpr-requests` |
| Check GDPR status | `SELECT status, completed_at FROM gdpr_erasure_requests WHERE customer_id = $1;` |
| Verify anonymization | `SELECT id, name, phone, anonymized_at FROM customers WHERE id = $1;` |
| Export audit log | `psql -c "\COPY (SELECT * FROM anonymization_audit_log WHERE created_at > $1 ORDER BY created_at ASC) TO 'audit.csv' CSV HEADER;"` |
| Check max DB retention | `SELECT max(retention_days) FROM locations;` |
| Set R2 override | `flyctl secrets set R2_RETENTION_OVERRIDE_DAYS=365` |

---

## Stage 31 — Observability & Worker Liveness

### Worker Heartbeats

All critical workers emit heartbeats every 15s (configurable via `WORKER_HEARTBEAT_INTERVAL_MS`):

| Worker | Heartbeat ID | pg-boss Job |
|--------|-------------|-------------|
| Settlement Cron | `settlement-cron` | `settlement.cron` |
| Dwell Monitor | `dwell-monitor` | `dwell.monitor` |
| Anonymizer Retention | `anonymizer-retention` | `anonymizer.retention` |

### Liveness Checker

`LivenessChecker` runs every 60s (`WORKER_LIVENESS_CHECK_MS`) as a pg-boss singleton:

1. Queries `ops_worker_heartbeat` for workers with `status != 'healthy'` or `last_seen_at > WORKER_LIVENESS_STALE_MS`
2. On first detection of stale `WORKER_CRITICAL_LIST` workers → publishes `worker.stale` / `worker.batch_stale` events → Telegram alert
3. On recovery → publishes `worker.recovered` event → auto-resolves
4. Batches alerts if >3 workers stale simultaneously

**Critical workers** (default): `dispatcher, settlement-cron, dwell-monitor, anonymizer-retention`

### Health Endpoint

`GET /health` returns:

```json
{
  "status": "healthy|degraded|unhealthy",
  "timestamp": "2026-06-03T12:00:00.000Z",
  "checks": {
    "postgres": { "status": "ok|degraded|down", "latencyMs": 5 },
    "workers": { "status": "ok", "entries": {
      "settlement-cron": { "status": "ok", "instanceId": "...", "staleSeconds": 3 }
    }},
    "messageBus": { "status": "ok", "latencyMs": 2 },
    "telegram": { "status": "ok", "latencyMs": 150 },
    "r2": { "status": "ok", "latencyMs": 80 },
    "settlement": { "status": "ok", "latencyMs": 3 },
    "anonymizer": { "status": "ok" },
    "backup": { "status": "ok" }
  }
}
```

- **200** = `healthy` or `degraded`
- **503** = any critical check (`postgres`) is `down`
- Per-check **2s timeout** → marks degraded, not down
- 0 PII in response

### Health Dashboard

`/admin/health.html` — Owner-only health monitoring UI:
- Real-time status of all checks
- Per-worker heartbeat status with stale seconds
- Auto-refresh toggle (30s default)
- Color-coded badges (green=ok, yellow=degraded, red=unhealthy)

### Sentry

If `SENTRY_DSN` is set:
- 100% error capture, 10% performance traces
- `beforeSend` — recursively redacts PII (email, phone, credit card, IBAN, URL query params, address patterns)
- `beforeBreadcrumb` — redacts PII from breadcrumb data
- User context: only UUID `id` retained, all other fields stripped
- Tags allowlist: `role`, `location_id`, `order_id`, `worker`, `db`, `error_code`
- Request: cookies stripped, headers allowlist-only

### Structured Logging (Pino)

Fastify logger configured with:
- `redact.paths` — strips cookies, authorization headers, API keys
- `serializers.req` — logs only method, url, hostname, ip
- PII redaction applied to all string fields via `PiiRedactor`
- Correlation ID via `x-correlation-id` header or auto-generated
- `AsyncLocalStorage` for cross-request correlation

Log level controlled by `LOG_LEVEL` env var (default: `info`).

### UptimeRobot

See [uptimerobot.md](./uptimerobot.md) for full monitor configuration.

### Env Vars

| Variable | Default | Description |
|----------|---------|-------------|
| `SENTRY_DSN` | — | Sentry project DSN (optional, disables Sentry if unset) |
| `LOG_LEVEL` | `info` | Pino log level: trace/debug/info/warn/error/fatal |
| `WORKER_HEARTBEAT_INTERVAL_MS` | `15000` | How often workers emit heartbeat |
| `WORKER_LIVENESS_CHECK_MS` | `60000` | How often liveness checker runs |
| `WORKER_LIVENESS_STALE_MS` | `60000` | Stale threshold before flagging worker |
| `WORKER_CRITICAL_LIST` | `dispatcher,settlement-cron,dwell-monitor,anonymizer-retention` | Workers that trigger alerts when stale |
| `GIT_SHA` | — | Git commit SHA for Sentry release tracking |
| Update location retention | `UPDATE locations SET retention_days = 180 WHERE id = $1;` |

---

## Stage 32 — Backup Verification

### Scheduled Restore-Test

`BackupVerifyWorker` runs daily at 04:00 UTC (configurable via `RESTORE_VERIFY_CRON`):

1. Picks latest daily backup
2. Downloads, decrypts, checksums
3. Restores to ephemeral sandbox DB (`dowiz_restore_sandbox_$timestamp`)
4. Runs 8 smoke checks (schema, counts, FK, invariants, PII-free)
5. Drops sandbox, cleans temp files
6. On failure: Sentry + Telegram + location_alert

### Trigger Manual Restore-Test

```bash
# Latest daily backup
pnpm backup:verify

# Specific backup
pnpm backup:verify --backup-id=<UUID>

# With full data integrity hash
pnpm backup:verify --full-hash
```

### Full DR Drill

```bash
pnpm backup:drill --full
# Generates .tmp/dr-drill-report.md
```

### List Backups

```bash
pnpm backup:list
pnpm backup:list --since=7d
```

### Owner UI

`/admin/backups.html` — Recent backups with restore-test badges, manual verify trigger.

### R2 Lifecycle Drift Detection

`backup.verify.r2` scheduled every 6h — checks:
- 3 random manifests from last 7 days (checksum + pg_restore --list)
- Bucket lifecycle policy matches expected rules
- Drift → alert immediate

### Sandbox DB Cleanup

After each restore-test:
- `DROP DATABASE` with `pg_terminate_backend`
- `shred` temp files (chmod 600)
- Verify: `SELECT datname FROM pg_database WHERE datname LIKE 'dowiz_restore_sandbox_%'` = 0 rows

### Env Vars (Stage 32)

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL_ADMIN` | — | Admin connection for sandbox DB (postgres://postgres@host:5432/postgres) |
| `RESTORE_VERIFY_CRON` | `0 4 * * *` | Scheduled restore-test cron |
| `RESTORE_VERIFY_FULL_HASH` | `false` | Enable full data integrity hash |
| `RESTORE_POOL_SIZE` | `2` | Sandbox connection pool size |

### Quick Reference

| Task | Command |
|------|---------|
| Restore-test | `pnpm backup:verify` |
| DR drill | `pnpm backup:drill --full` |
| List backups | `pnpm backup:list` |
| Run verify via API | `POST /api/admin/backups/verify` |
| Check restore health | `GET /health → checks.backup_restore` |
| Admin UI | `/admin/backups.html` |
| DR runbook | `docs/phase5/disaster-recovery.md` |
