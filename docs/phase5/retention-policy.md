# Phase 5 — Retention Policy (Stage 30)

## Purpose

Defines how long PII is retained across DB and backup layers before automated anonymization or erasure. The retention policy ensures that no PII outlives its configured window in any storage tier.

---

## Retention Table

| Data | DB Retention | R2 Bucket | R2 Effective Erasure |
|------|-------------|-----------|---------------------|
| Customers PII | `retention_days` (default 365) | n/a (DB only) | After DB anonymization |
| Orders PII | `retention_days` (default 365) | n/a (DB only) | After DB anonymization |
| Storage media (customer avatar) | `retention_days` | n/a (Storage delete on anonymization) | Immediate on anonymization |
| R2 backups (full DB) | `retention_days` | `daily/` 24h, `weekly/` 30d, `monthly/` 90d, `yearly/` 7y | R2 effective ≤ DB retention |

### DB Retention

- `retention_days` is a per-location setting stored in `locations.retention_days` (default: 365).
- The AnonymizerService queries:
  ```sql
  SELECT id FROM customers c
  JOIN locations l ON l.id = c.location_id
  WHERE c.anonymized_at IS NULL
    AND c.created_at + (l.retention_days || ' days')::interval <= now();
  ```
- The computed due date is materialized as `customers.anonymization_due_at` (updated on `created_at` or `location.retention_days` change) to avoid repeated joins.

### R2 Backup Lifecycle

| Schedule | Retention | Path Prefix |
|----------|-----------|-------------|
| Hourly | 24h | `hourly/` |
| Daily | 30d | `daily/` |
| Weekly | 90d | `weekly/` |
| Monthly | 7y | `monthly/` |

R2 lifecycle rules auto-delete objects past these windows. Deletion is eventual (SLA: 24h).

### Storage Media

- Customer avatar uploaded to R2/S3 is deleted synchronously during anonymization.
- The DB column is NULLed and the underlying object is deleted in the same transaction (deferred action via pg-boss if async needed).

---

## Critical Constraint: R2 Must Not Outlive DB

```
min(R2 max retention) ≤ max(DB retention)
```

**Rationale:** If R2 backups retain PII longer than the DB retention window, restoring from an old backup would reintroduce PII that should have been anonymized. This is an orphan PII risk.

**Enforcement:**

| Layer | Check |
|-------|-------|
| Migration | `CHECK (locations.retention_days <= 3650)` (max 10 years) |
| R2 lifecycle | `daily/` 30d, `weekly/` 90d, `monthly/` 7y — max effective = 7y |
| env override | `R2_RETENTION_OVERRIDE_DAYS` — reduces lifecycle, never increases |

**Examples of constraint satisfaction:**

| DB retention_days | Max R2 retention | Satisfies? |
|-------------------|------------------|------------|
| 365 (default) | 2555 (7y) | ✅ R2 > DB — see Override below |
| 180 | 180 | ✅ Equal |
| 730 | 365 | ❌ FAIL — DB > R2, data lost before anonymization |
| 365 | 90 | ❌ FAIL — DB > R2, but override expected; safe if intended |

> **Note:** Default retention (365d) vs R2 monthly (7y) violates the constraint. The override mechanism below exists specifically for this scenario.

---

## R2 Retention Override

### Purpose

The `R2_RETENTION_OVERRIDE_DAYS` env variable ensures that R2 lifecycle rules never retain PII beyond the maximum DB retention across all locations.

### Rules

1. **Override reduces R2 lifecycle, never increases it.**
2. If `R2_RETENTION_OVERRIDE_DAYS < max(locations.retention_days)` → startup warning (non-fatal, but logged).
3. If `R2_RETENTION_OVERRIDE_DAYS` is not set → R2 follows default lifecycle (7y max), operator is responsible for alignment.

### How It Works

```typescript
// On application startup:
const overrideDays = parseInt(process.env.R2_RETENTION_OVERRIDE_DAYS || '0');
if (overrideDays > 0) {
  // Apply reduced lifecycle to R2 bucket
  //   hourly/  → min(24h, overrideDays * 1h)
  //   daily/   → min(30d, overrideDays)
  //   weekly/  → min(90d, overrideDays)
  //   monthly/ → min(7y, overrideDays)
}
```

### Recommended Configuration

| Scenario | R2_RETENTION_OVERRIDE_DAYS | Effect |
|----------|---------------------------|--------|
| Default (365d retention) | 365 | Monthly backups expire at 365d instead of 7y |
| Short retention (180d) | 180 | All tiers ≤ 180d |
| Maximum (10y) | not set | Use default lifecycle (7y monthly), but DB is 10y — disable monthly or accept gap |

---

## Backup Manifest

The backup `manifest.json` file contains:

```json
{
  "version": 2,
  "createdAt": "2026-06-03T03:00:00Z",
  "snapshotId": "bak_abc123",
  "rowCounts": {
    "customers": 15234,
    "orders": 89201,
    "order_items": 245678
  },
  "checksums": {
    "customers": "sha256:abc...",
    "orders": "sha256:def..."
  },
  "redactionWindowDays": 365,
  "encryptionAlgorithm": "aes-256-gcm"
}
```

### PII-Free Guarantee

- `redactionWindowDays` is the retention_days in effect at backup time.
- No customer names, phones, emails, or addresses appear in the manifest.
- Only row counts and checksums — used for integrity verification after restore.
- Because the manifest is PII-free, R2 erasure via lifecycle is safe: the manifest can outlive DB PII without creating compliance risk.

### Redaction Window

- `redactionWindowDays` is recorded at backup time from `max(locations.retention_days)`.
- On restore, if `redactionWindowDays` < current max retention, all restored PII must be re-anonymized at the earlier window.
- The restore script checks this delta and issues a warning if restored data contains rows whose `created_at + redactionWindowDays < now()`.
