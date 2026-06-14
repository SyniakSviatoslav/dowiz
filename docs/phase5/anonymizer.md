# Phase 5 — Anonymizer (Stage 30)

## Purpose

The Anonymizer provides PII anonymization for compliance with data protection regulations (GDPR, LGPD, etc.). It implements a **single-mechanism-two-triggers** architecture: one `AnonymizerService` core, two independent callers.

| Trigger | Source | Cadence | Scope |
|---------|--------|---------|-------|
| **RetentionTrigger** | pg-boss cron (`anonymizer.retention`) | Daily (`0 3 * * *` via `retention_cron_schedule`) | All customers whose `anonymized_at IS NULL` AND `anonymization_due_at <= now()` |
| **GdprErasureTrigger** | `POST /api/owner/locations/:id/gdpr-requests` | On-demand, rate-limited | Single customer, immediate execution |

Both callers invoke the same `AnonymizerService.anonymize(customerIds[])` — code path is identical.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      AnonymizerService                        │
│                                                              │
│  anonymize(customerIds[], reason)                            │
│    → BEGIN                                                   │
│    → FOR EACH id IN customerIds:                             │
│        → SELECT ... WHERE id = $1 AND anonymized_at IS NULL  │
│        → IF NOT FOUND: CONTINUE (idempotent skip)            │
│        → UPDATE customers SET PII = NULL/anon-token          │
│        → UPDATE orders SET PII = NULL                        │
│        → DELETE storage media (customer avatar)              │
│        → INSERT anonymization_audit_log (append-only)        │
│        → UPDATE gdpr_erasure_requests (if GDPR flow)         │
│    → COMMIT                                                   │
│                                                              │
│  Each customer is anonymized atomically within a single TX.  │
│  Batch failure → per-customer rollback, not batch abort.     │
└──────────────────────────────────────────────────────────────┘
         ▲                      ▲
         │                      │
  RetentionTrigger        GdprErasureTrigger
  (cron, daily)           (owner API, on-demand)
```

### RetentionTrigger (cron)

- Scheduled via pg-boss: `SELECT pg_boss.schedule('anonymizer.retention', '0 3 * * *', ...)`
- Singleton key: `anonymizer.retention` — never overlaps, N=2
- Queries: `SELECT id FROM customers WHERE anonymization_due_at <= now() AND anonymized_at IS NULL LIMIT 1000`
- Processes in batches of 1000 to keep TX short
- On completion: emits `anonymizer.retention_completed` event (message count, duration)

### GdprErasureTrigger (API)

- Endpoint: `POST /api/owner/locations/:locationId/gdpr-requests`
- Body: `{ customerId: uuid }`
- Rate limit: 1 request per customer per 24h (DB constraint)
- Creates `gdpr_erasure_requests` row (status=pending) → worker picks up
- Worker calls `AnonymizerService.anonymize([customerId], 'gdpr_erasure')`
- On completion: status=completed, WS event `gdpr.request_completed`
- On failure: status=failed, error_message=PII-free

---

## Anonymization Scope

### Customers Table

| Column | Action | Notes |
|--------|--------|-------|
| `id` | **Preserved** | FK reference for orders, courier_assignments, settlement_items |
| `name` | → `NULL` | Irreversible |
| `phone` | → `'anon-' || substr(md5(random()::text), 1, 12)` | Irreversible; format distinct from real phone |
| `email` | → `NULL` | Irreversible |
| `avatar_url` | → `NULL` (storage delete) | S3/R2 object deleted before column NULL |
| `anonymized_at` | → `now()` | Marks completion; used for idempotency |
| `anonymized_by` | → `'retention'` or `'gdpr_erasure'` | Trigger reason |
| All other columns | **Preserved** | `completed_count`, `no_show_count`, `last_no_show_at`, `created_at` — business fields kept |

### Orders Table

| Column | Action | Notes |
|--------|--------|-------|
| `customer_name` | → `NULL` | Snapshot field, PII removed |
| `customer_phone` | → `NULL` | Snapshot field, PII removed |
| `customer_address` | → `NULL` | Snapshot field, PII removed |
| `client_ip_hash` | → `NULL` | No longer needed; hash is not PII but removed per privacy |
| `delivery_notes` | → `NULL` | May contain PII |
| All financial columns | **Preserved** | `total`, `subtotal`, `delivery_fee`, `discount`, `payment_method`, `payment_status` — immutable financial records |
| Status columns | **Preserved** | `status`, `status_history`, `created_at`, `updated_at` |

### What is NOT Deleted

| Data | Reason |
|------|--------|
| `customers.id` | FK reference preserved — orders, courier_assignments, settlement_items still reference it |
| `order_items` | Snapshot of what was ordered — immutable business record |
| `courier_assignments` | Courier work history — immutable operational record |
| `settlement_items` | Financial settlement records — immutable accounting record |
| `customer_signals` | Anonymized (no PII) — retained for owner analytics |

### Storage Media (Customer Avatar)

- Before NULLing `customers.avatar_url`, the AnonymizerService deletes the underlying object from R2/S3.
- Delete is best-effort: if storage is unreachable, the column is still NULLed and the error logged. A follow-up cron (`anonymizer.storage_cleanup`) retries orphaned deletions.

---

## Idempotency

The AnonymizerService is fully idempotent:

```sql
-- Guard clause at the start of each customer anonymization
SELECT id, anonymized_at FROM customers WHERE id = $1;
-- IF anonymized_at IS NOT NULL → SKIP (log + return)
```

This ensures:
- Retention cron running on consecutive days → second run is no-op
- GDPR request for already-anonymized customer → silently succeeds (no error)
- Manual re-run after partial failure → skips completed, processes remaining
- Concurrent retention and GDPR on same customer → first TX wins, second skips

---

## Audit

### `anonymization_audit_log` (append-only)

```sql
CREATE TABLE anonymization_audit_log (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_type    TEXT NOT NULL CHECK (subject_type IN ('customer', 'order')),
    subject_id      UUID NOT NULL,
    trigger         TEXT NOT NULL CHECK (trigger IN ('retention', 'gdpr_erasure')),
    fields_cleared  TEXT[] NOT NULL,  -- array of column names that were NULLed
    performed_by    TEXT NOT NULL DEFAULT 'system',  -- 'system' or 'owner:<id>'
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

- **PII-free**: No names, phones, emails, addresses. Only column names and subject IDs.
- **Append-only**: INSERT-only. No UPDATE, no DELETE. `FORCE ROW LEVEL SECURITY` → read-only by compliance team.
- **Retention**: Never deleted. Permanent compliance record.
- **Query pattern**:
  ```sql
  SELECT * FROM anonymization_audit_log
  WHERE subject_id = $1
  ORDER BY created_at ASC;
  ```

---

## Security

### RLS + FORCE on All New Tables

| Table | RLS Policy | Notes |
|-------|-----------|-------|
| `anonymization_audit_log` | `USING (true)` for compliance roles; tenant-scoped for owners | `FORCE ROW LEVEL SECURITY` |
| `gdpr_erasure_requests` | `USING (location_id = current_setting('app.location_id')::uuid)` | `FORCE ROW LEVEL SECURITY` |

### Tenant Isolation

- `gdpr_erasure_requests` is scoped by `location_id`
- Cross-tenant query → 0 rows returned (RLS filter), never leaks existence
- Cross-tenant `INSERT` → blocked by RLS (no way to set another tenant's `app.location_id`)

### Authentication & Authorization

- RS256 JWT only — 0 cookies
- Owner-only endpoints — courier and client tokens are rejected at middleware
- Zod `.strict()` on all request bodies — no unexpected fields

### Red Lines (from CONVENTIONS.md)

| # | Rule | Rationale |
|---|------|-----------|
| 🔴 | **Anonymization is irreversible** | Once NULLed, PII cannot be recovered. No backup-restore bypass. |
| 🔴 | **No soft-delete** | `anonymized_at` is a marker, not a delete flag. Row stays, PII goes. |
| 🔴 | **0 PII in audit log** | `anonymization_audit_log` stores only column names + subject IDs. |
| 🔴 | **Append-only audit** | No UPDATE/DELETE on `anonymization_audit_log`. Immutable by policy and trigger. |
| 🔴 | **Idempotent by design** | `anonymized_at IS NOT NULL` → skip. No double-anonymization. |
| 🔴 | **No cross-tenant leak** | Cross-tenant GDPR query → 404, not 403. |
| 🔴 | **RLS + FORCE on all tables** | No bypass. Even superuser must set `app.location_id`. |
| 🔴 | **Batch failure isolation** | Per-customer TX. One failure cannot abort the batch. |
