# Backup Verification — Restore-Test Worker (P32)

## Architecture

```
┌──────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  pg-boss     │────▶│  backup.verify  │────▶│  Sandbox DB     │
│  daily 04:00 │     │  .restore       │     │  (ephemeral)    │
└──────────────┘     └────────┬────────┘     └─────────────────┘
                              │
                              ▼
                     ┌──────────────────┐
                     │  Smoke-Checks    │
                     │  (8 checks)      │
                     └──────────────────┘
                              │
                    ┌─────────┴─────────┐
                    ▼                   ▼
             ┌──────────┐     ┌──────────────┐
             │ Success  │     │ Failure      │
             │ → audit  │     │ → Sentry     │
             │   log    │     │ → Telegram   │
             └──────────┘     │ → location   │
                              │   _alert     │
                              └──────────────┘
```

## Worker: `backup.verify.restore`

- **Schedule:** `0 4 * * *` UTC (configurable via `RESTORE_VERIFY_CRON`)
- **Singleton:** pg-boss `singletonKey` + advisory lock 3 (N=2 safety)
- **Timeout:** 30 min total, 25 min for pg_restore

### Stages:
1. **lock** — acquire advisory lock
2. **select** — pick latest daily backup from `backup_metadata`
3. **download** — fetch encrypted blob from R2 to temp dir
4. **decrypt** — AES-256-GCM decrypt with `BACKUP_ENCRYPTION_KEY`
5. **checksum** — SHA256 verify against manifest
6. **restore** — `pg_restore --clean --if-exists --no-owner --jobs=2` to sandbox DB
7. **smoke_check** — 8 smoke checks (see below)
8. **cleanup** — drop sandbox DB, `shred` temp files

### Failure Modes:
- Lock busy → skip (another instance running)
- Download fail → Sentry + Telegram + location_alert
- Decrypt fail → Sentry + Telegram + location_alert
- Checksum mismatch → Sentry + Telegram + location_alert (corruption detected)
- pg_restore fail → Sentry + Telegram + location_alert
- Smoke-check fail → Sentry + Telegram + location_alert

## Smoke-Checks

| # | Check | What it verifies |
|---|-------|-----------------|
| 1 | `schema_validation` | All expected tables present (whitelist of ~28 tables) |
| 2 | `row_counts` | Count > 0, < 10x baseline for critical tables |
| 3 | `fk_integrity` | >= 20 FK constraints in public schema |
| 4 | `menu_versions_unique` | No location has > 1 active menu_version |
| 5 | `payout_sums` | courier_payouts.total_earned == sum(settlement_items.amount) |
| 6 | `order_totals` | subtotal + delivery_fee + tax_total - discount_total == total |
| 7 | `time_order` | delivered_at >= created_at (no time-travel) |
| 8 | `pii_free` | 100 random rows × 3 tables — 0 PII matches |
| 9* | `sample_hashes` | SHA256 of 10 random orders (opt-in, env `RESTORE_VERIFY_FULL_HASH=true`) |

## R2 Sample Verify

- **Schedule:** `0 */6 * * *` (separate, lighter)
- Samples 3 random backups from last 7 days
- Downloads manifest → checksum verify → `pg_restore --list`
- Checks R2 bucket lifecycle policy drift
- No full restore (lightweight)

## Alerts

| Event | Channel | Priority |
|-------|---------|----------|
| `backup.verify.failed` (any stage) | Sentry + Telegram + location_alert | P1 |
| `backup.verify.stale` (>25h since last verify) | Health endpoint degrades | P2 |
| `r2.lifecycle.drift` | Sentry + Telegram | P2 |
| `r2.manifest.corruption` | Sentry + Telegram | P1 |

## CLI Commands

```bash
# Manual restore-test
pnpm backup:verify                          # Latest daily
pnpm backup:verify --backup-id=<ID>         # Specific backup
pnpm backup:verify --full-hash              # With data integrity hash

# Full DR drill (generates report)
pnpm backup:drill --full

# List recent backups
pnpm backup:list                            # Last 30 days
pnpm backup:list --since=7d                 # Last 7 days
pnpm backup:list --since=24h                # Last 24 hours
```

## Env Vars

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL_ADMIN` | — | Admin connection for sandbox DB management (postgres://postgres@host:5432/postgres) |
| `RESTORE_VERIFY_CRON` | `0 4 * * *` | Cron schedule for restore-test |
| `RESTORE_VERIFY_FULL_HASH` | `false` | Enable full data integrity hash check |
| `RESTORE_POOL_SIZE` | `2` | Sandbox connection pool size |
