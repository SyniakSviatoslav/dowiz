# Phase 5 — GDPR Erasure Requests (Stage 30)

## Purpose

Implements the GDPR Right to Erasure ("Right to be Forgotten") via an owner-triggered workflow. There is no customer-facing self-service — the owner submits requests on behalf of customers (Phase 5+ may add self-service).

---

## Endpoint

### `POST /api/owner/locations/:locationId/gdpr-requests`

**Auth:** Owner JWT (RS256)

**Body:**
```json
{
  "customerId": "uuid"
}
```

**Zod schema:** `.strict()` — rejects unexpected fields.

**Response (201):**
```json
{
  "id": "uuid",
  "customerId": "uuid",
  "locationId": "uuid",
  "status": "pending",
  "createdAt": "2026-06-03T10:00:00Z"
}
```

**Errors:**

| Code | Condition |
|------|-----------|
| 404 | Customer not found in this location (cross-tenant → 404, not 403) |
| 429 | Rate limit exceeded (1 request per customer per 24h) |
| 409 | Duplicate in-flight request (UNIQUE partial index) |
| 422 | Customer already anonymized (anonymized_at IS NOT NULL) |

---

## Rate Limit

- **Limit:** 1 request per customer per 24h.
- **Enforcement:** DB-level via UNIQUE partial index (see below).
- **Scope:** Per location. Customer with IDs in two locations → can have one request per location per 24h.
- **Window:** Rolling 24h window from the most recent request's `created_at`.
- **Bypass:** Owner can clear the rate limit by manually deleting the blocking row (audited operation, not exposed via API).

### Implementation

```sql
CREATE UNIQUE INDEX uq_gdpr_requests_customer_24h
  ON gdpr_erasure_requests (customer_id, date_trunc('day', created_at))
  WHERE status IN ('pending', 'completed');
```

This prevents more than 1 request per customer per calendar day (simplified from rolling 24h; production uses a `created_at > now() - interval '24 hours'` check in the application layer for true rolling window).

---

## Dedup

A UNIQUE partial index prevents multiple in-flight requests for the same customer:

```sql
CREATE UNIQUE INDEX uq_gdpr_requests_customer_inflight
  ON gdpr_erasure_requests (customer_id)
  WHERE status = 'pending';
```

This guarantees:
- At most one pending request per customer at any time.
- A completed or failed request does not block a new one (subject to rate limit).
- Concurrent inserts for the same customer → one succeeds, one gets a unique violation (409).

---

## Flow

```
Owner UI → POST /api/owner/locations/:lid/gdpr-requests
                │
                ▼
          Create gdpr_erasure_requests row
          (status = 'pending')
                │
                ▼
          pg-boss worker picks up job
          (gdpr.erasure, singleton per request)
                │
                ▼
          AnonymizerService.anonymize([customerId], 'gdpr_erasure')
                │
          ┌─────┴─────┐
          ▼           ▼
      success      failure
          │           │
          ▼           ▼
    status =    status = 'failed'
    'completed' error_message
          │      (PII-free, retry 3x)
          ▼           │
    WS event     ┌────┘
    gdpr.        ▼
    request_  Retry? → yes → backoff → repeat
    completed   no  → final status = 'failed'
```

### States

| State | Meaning | Next |
|-------|---------|------|
| `pending` | Request created, worker not yet picked up | → `processing` |
| `processing` | Worker is executing AnonymizerService | → `completed` or `failed` |
| `completed` | Anonymization succeeded | Terminal |
| `failed` | Anonymization failed after 3 retries | Terminal |

---

## Worker

### Implementation

- **Queue:** `gdpr.erasure` (pg-boss)
- **Singleton key:** `gdpr-erasure-{requestId}` (per-request singleton, not global)
- **Concurrency:** N=2 (max 2 simultaneous GDPR erasures)
- **Retry:** 3 attempts with exponential backoff (10s, 30s, 60s)
- **Timeout:** 30s per attempt

### Error Handling

- `error_message` in `gdpr_erasure_requests` is **PII-free**: stores only error codes and generic messages:
  - `ANONYMIZATION_FAILED` — generic anonymization error
  - `CUSTOMER_NOT_FOUND` — customer deleted between request creation and processing
  - `TIMEOUT` — operation exceeded 30s
  - `STORAGE_DELETE_FAILED` — avatar deletion failed (anonymization still proceeds)
- Raw error details (stack traces, DB error messages) go to structured logs, not the DB.
- After 3 failed retries: status = `failed`, no automatic retry. Owner must re-create the request.

### Idempotency

- Worker checks `gdpr_erasure_requests.status` before processing:
  - If already `completed` → no-op (job ack, return)
  - If already `failed` and retry exhausted → no-op
- `AnonymizerService.anonymize()` itself is idempotent (checks `anonymized_at`).

---

## Owner UI

### `/admin/gdpr`

**Page sections:**

1. **Request List** — paginated table of all GDPR requests for the current location
   - Columns: Customer ID (truncated), Status, Created At, Completed At, Error (if failed)
   - Filters: status, date range
   - Sort: created_at DESC (default)

2. **Create New Request** — form with customer ID input (UUID or phone lookup)
   - Phone lookup: resolves phone → customer ID (E.164 format, tenant-scoped)
   - Confirmation dialog: "This will permanently anonymize this customer's PII. This action cannot be undone."

3. **Request Detail** — single request view
   - Full audit trail: `anonymization_audit_log` entries for this customer
   - Retry button (only if status = failed)
   - Customer info: ID, anonymized_at (if completed)

### No Customer-Facing Self-Service

- Phase 5: Owner-only. No `GET /me/gdpr` or `DELETE /me/data`.
- Rationale: Albanian market context — most interactions are mediated by the restaurant. Owner acts as data controller.
- Phase 5+ may add a `POST /customer/gdpr-request` endpoint that creates a ticket for the owner to review.

---

## Cross-Tenant Behavior

| Operation | Cross-Tenant Behavior |
|-----------|----------------------|
| `POST /gdpr-requests` with `customerId` from another tenant | → 404 (not 403) |
| `GET /gdpr-requests` listing | RLS filters to current tenant only |
| Worker processing | Checks `customer.location_id` matches request's `location_id` |
| Audit log query | RLS restricts to tenant-scoped rows |

**Rationale:** Returning 404 (instead of 403) prevents existence oracle attacks. An attacker cannot distinguish between "this customer doesn't exist" and "this customer exists in another tenant."

---

## RBAC

| Role | Can create? | Can view? | Can retry? |
|------|------------|-----------|------------|
| Owner | ✅ | ✅ | ✅ |
| Courier | ❌ | ❌ | ❌ |
| Client | ❌ | ❌ | ❌ |
| Admin (internal) | ✅ | ✅ | ✅ |

- Enforcement via middleware: `req.role === 'owner'` on all `/gdpr-requests` routes.
- Admin (internal ops team) is authorized via a separate `admin` role with explicit scoping.
