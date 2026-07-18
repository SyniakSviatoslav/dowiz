# Backup & Disaster Recovery Runbooks

## DR Targets

| Metric | Target | Notes |
|--------|--------|-------|
| RTO (Recovery Time Objective) | **4 hours** | From detection of primary DB loss to full read-write operation on restored instance |
| RPO (Recovery Point Objective) | **1 hour** | Maximum data loss measured in time. Hourly backups guarantee ≤1h of lost data. |

## 1. Pre-requisites

- `BACKUP_ENCRYPTION_KEY` — 32-byte base64 key, must match the key used when backup was created
- `R2_ACCESS_KEY_ID` / `R2_SECRET_ACCESS_KEY` — with read access to the backup bucket
- R2 bucket with lifecycle policy:
  - Hourly: 24h retention
  - Daily: 30d retention  
  - Weekly: 90d retention
  - Monthly: 7y retention

## 2. Restoration Drill (Dry-Run)

This procedure verifies that a snapshot can be successfully downloaded and decrypted, without overwriting production data. Run monthly.

1. **List recent snapshots**:
   ```bash
   pnpm backup:restore --list
   ```
2. **Execute Restore Script (dry-run)**:
   ```bash
   pnpm backup:restore --dry-run --snapshot=<backupId>
   ```
3. **Verify Output**: Script downloads → decrypts → verifies checksum → validates row counts against `backup_metadata.row_counts`. All steps must pass.
4. **On failure**: If checksum mismatch or decryption error, file a critical incident and re-run backup immediately.

## 3. Emergency Restore to Fresh Instance (RTO ≤ 4h)

If primary DB is lost:

1. **Provision a new PostgreSQL instance** (≤ 30 min).
2. **Apply all migrations** to create schema:
   ```bash
   pnpm migrate:up
   ```
3. **Set environment variables** on the restore machine:
   - `BACKUP_ENCRYPTION_KEY` (same key as when backup was created)
   - `R2_ENDPOINT`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_BUCKET`
   - `DATABASE_URL_MIGRATIONS` pointing to the new instance
4. **Restore from latest snapshot**:
   ```bash
   # List snapshots to find the latest completed one
   pnpm backup:restore --list
   
   # Full restore (non-dry-run)
   pnpm backup:restore --snapshot=<backupId>
   ```
   
   For manual restore via `pg_restore`:
   ```bash
   # Download and decrypt manually
   # Then run:
   pg_restore -d "$DATABASE_URL_MIGRATIONS" --clean --if-exists --no-owner --no-acl <decrypted_dump>
   ```
5. **Verify application connectivity** (≤ 30 min).
6. **Update DNS/Wiring** to point API servers to the new database instance.
7. **Verify data integrity**: Run `pnpm verify:db` and spot-check recent orders/settlements.

## 4. Key Rotation

To rotate the `BACKUP_ENCRYPTION_KEY`:

1. Generate a new 32-byte base64 key:
   ```bash
   node -e "console.log(require('crypto').randomBytes(32).toString('base64'))"
   ```
2. Update `BACKUP_ENCRYPTION_KEY` on all instances.
3. Restart API servers.
4. **Old key must be archived** (e.g., in a password manager or secure vault) because old backups remain encrypted with the previous key.
5. Trigger a fresh backup manually to verify new key works:
   ```bash
   # Publish backup.hourly job via pg-boss
   ```

## 5. Monitoring & Alerts

- `backup.failed` event → Telegram owner alert (immediate)
- Backup drift > 70 min → daily digest alert
- Restore dry-run failure → immediate escalation
- R2 reachability → `/health` endpoint exposes `backup.r2_reachable`

## 6. P45-W3 off-Hetzner immutable copy (copy 3) — OPERATOR-GATED

Topology per OPS-14 (`tools/ops-alert/offsite-copy.sh`): copy 3a = rsync.net
(SSH-only, zero-egress, credential-isolated); copy 3b = Object-Lock COMPLIANCE
bucket (immutable leg — early deletion impossible even for the key holder, OPS-14
adversarial case a). Both are OPTIONAL and guarded: the script no-ops each target
until its env is set, so it is safe to cron BEFORE provisioning.

**Operator provisioning (human-only — agent does NOT create accounts/buckets):**

1. rsync.net account + SSH key. Then set on the box:
   `RSYNC_NET_HOST`, `RSYNC_NET_USER`, `RSYNC_NET_PATH` (= `/dowiz-backup/copy3`).
2. Object-Lock bucket in COMPLIANCE mode — **set at bucket CREATION, not
   retrofittable**. Then set `OFFSITE_BUCKET` + `AWS_REGION` + standard
   `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY`. Use a credential-isolated IAM
   principal (separate from the Hetzner blast radius, OPS-15 key custody).
3. Cron: `tools/ops-alert/offsite-copy.sh` on the backup interval (≤ RPO 1h).

**Freshness metric (§4a.3 monitoring hook):** the script writes Prometheus
textfile at `$METRICS_FILE`:
- `dowiz_ops_backup_last_success_seconds{subject="kernel_state",copy="rsyncnet"|"objectlock"}`
- `dowiz_ops_backup_bytes_written{...}`

Ride existing pager rules (no new alerting code): age > `BACKUP_STALENESS_FACTOR`
× interval ⇒ **S0** (pager rule 5 — backup failure is the silent failure); a
sudden bytes-written drop ⇒ **S1** (pager rule 7). Absence of a copy's metric
rows on a provisioned host means that copy's last run failed (fail-closed: a
0-byte or errored run emits NOTHING for that copy).

**RED test (R2, W3):** with Hetzner credentials deliberately withheld, an
`aws s3 cp` / `rsync` to copy 3 from a *different* host (or after revoking the
Hetzner IAM principal) still restores — copy 3 survives Hetzner-unreachable.

**Honest residual:** copy 3 shares the box's `TELEGRAM_BOT_TOKEN` until W3's
separate infra bot (§4e.2) lands; a revoked/blocked bot mutes the metric's alert
lane. Named, not hidden.

