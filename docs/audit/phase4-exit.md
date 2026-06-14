# Phase 4 Exit Audit — VERDICT: **NO-GO**

Generated: 2026-06-02
Auditor: static code analysis (agent)
Scope: Stages 23–29 (dwell monitor, anti-fake signals, preflight, push, onboarding)

---

## 8 Findings — 1 Critical, 4 High, 3 Medium

### 🔴 CRITICAL

#### FINDING-1: Phase 4 RLS policies lack membership check (B1/B8)

**Files:**
- `packages/db/migrations/1780421100057_anti-fake-signals.ts:40,69,96`
- `packages/db/migrations/1780421100054_anti-fake-seam.ts:40`

**Tables: `customer_signals`, `velocity_events`, `customer_otp_sessions`, `phone_otp`**

All four Phase 4 tables use:
```sql
USING (location_id = current_setting('app.location_id', true)::uuid)
```

Phase 1–3 tables use the correct pattern:
```sql
USING (location_id IN (SELECT app_member_location_ids()))
```

The `app.location_id` pattern trusts the client to supply the correct location UUID **without verifying the user actually has an active membership at that location**. An authenticated user can `SET LOCAL app.location_id` to any UUID and read any tenant's signals, velocity events, OTP sessions, and phone OTP records.

**Fix:** Replace all four policies with:
```sql
USING (location_id IN (SELECT app_member_location_ids()))
WITH CHECK (location_id IN (SELECT app_member_location_ids()))
```

---

### 🟠 HIGH

#### FINDING-2: `customer_devices` missing `FORCE ROW LEVEL SECURITY` (B1)

**File:** `packages/db/migrations/1780421100059_push-notifications.ts:18`

```sql
ALTER TABLE customer_devices ENABLE ROW LEVEL SECURITY;
-- FORCE ROW LEVEL SECURITY MISSING
```

Without `FORCE`, the table owner (superuser) bypasses RLS and can read all Web Push subscriptions across all tenants.

**Fix:** Add `ALTER TABLE customer_devices FORCE ROW LEVEL SECURITY;`

---

#### FINDING-3: `verify-rls.ts` does not test Phase 4 tables (B1)

**File:** `packages/db/scripts/verify-rls.ts:27-49`

The `TENANT_TABLES` array lists 26 Phase 1–3 tables but **zero** Phase 4 tables:
- `customer_signals` ❌
- `velocity_events` ❌
- `customer_otp_sessions` ❌
- `phone_otp` ❌
- `customer_devices` ❌

Broken RLS on any Phase 4 table would pass verification undetected.

**Fix:** Add all 5 Phase 4 tenant-scoped tables to `TENANT_TABLES`.

---

#### FINDING-4: No `test:phase4` or `verify:privacy` scripts (B9)

**File:** `package.json`

Missing from `scripts`:
- `test:phase4` — unified test runner for all Phase 4 stages
- `verify:privacy` — PII leak detector script (JSON recursive check exists in `pii-leak-detector.test.ts` but not wired as a runnable verify script)

**Fix:** Add both scripts to `package.json`.

---

#### FINDING-5: Missing location membership checks in onboarding + owner push routes (B8)

**Files:**
- `apps/api/src/routes/owner/onboarding.ts:27-35` — auth hook checks role only, never verifies `user.activeLocationId` matches `:locationId`
- `apps/api/src/routes/owner/push.ts:17-25` — auth hook checks role only, never verifies `user.activeLocationId`

An authenticated owner can read/modify onboarding state and push subscriptions for **any location** by guessing its UUID.

Note: The auth plugin provides `requireLocationAccess` (auth.ts:43) but it is not used by these routes.

**Fix:** Add `requireLocationAccess` or inline `activeLocationId` check to all onboarding and owner push route handlers.

---

### 🟡 MEDIUM

#### FINDING-6: Missing test files for Stages 27, 28, 29

- `test-stage27.ts` (preflight) — **does not exist**
- `test-stage28.ts` (push) — **does not exist**
- `test-stage29.ts` (onboarding) — **does not exist**

The AGENTS.md summary claims these test files exist, but they do not.

**Fix:** Write and commit tests for all three stages.

---

#### FINDING-7: `verify:n2` references wrong file path

**File:** `package.json:30`

```json
"verify:n2": "tsx --env-file=.env scripts/verify-n2.ts",
```

Actual file exists at `apps/api/tests/verify-n2.ts`, not `scripts/verify-n2.ts`.

**Fix:** Update path to `apps/api/tests/verify-n2.ts`.

---

#### FINDING-8: `customer_devices` RLS uses Supabase-specific JWT claim path

**File:** `packages/db/migrations/1780421100059_push-notifications.ts:26`

```sql
USING (customer_id = (current_setting('request.jwt.claim.sub', true))::uuid)
```

This is incompatible with the `app.user_id` convention used everywhere else. The `verify-rls.ts` test uses `SET LOCAL app.user_id` which has no effect on this policy.

**Fix:** Standardize to `app.user_id` pattern or add a separate verify test that tests `request.jwt.claim.sub` path.

---

## ✅ Passed Invariants (verified compliant)

| Check | Status | Evidence |
|---|---|---|
| B2: SIGTERM child forwarding | ✅ | `shutdown.ts:33-36` forwards to children; API + worker both handle SIGTERM |
| B3: RS256 JWT only | ✅ | `jwt.ts:29,40-46` — RS256 enforced, other algs rejected, kid checked |
| B4: Rate limits on owner actions | ✅ | `dashboard.ts:188,198,212` — confirm=30/min, reject=30/min, assign=10/min |
| B5: Preflight before idempotency | ✅ | `orders.ts:133` preflight at step 4, `:269` idempotency at step 5 |
| B6: Migration naming | ✅ | Timestamp prefix per node-pg-migrate convention (not hex blob) |
| B7: Zod `.strict()` on all P4 endpoints | ✅ | Present on push, otp, signals, onboarding, order-meta, locations |
| B8: Cross-tenant 404 (not 403) | ✅ | `dashboard.ts:17`, `signals.ts:17` both return 404 |
| E26: Signal never blocks | ✅ | Soft-confirm rolls back without idempotency key (`orders.ts:250-258`) |
| 0 cookies | ✅ | No cookie usage anywhere in P4 code |
| PII masking | ✅ | maskName/maskPhone used in dashboard, alerts, signals |

---

## Remediation Order

1. **CRITICAL** Fix RLS policies on `customer_signals`, `velocity_events`, `customer_otp_sessions`, `phone_otp` to use `app_member_location_ids()`
2. **HIGH** Add `FORCE RLS` to `customer_devices`
3. **HIGH** Add Phase 4 tables to `verify-rls.ts`
4. **HIGH** Add location membership checks to onboarding + owner push routes
5. **HIGH** Create `test:phase4` and `verify:privacy` scripts in `package.json`
6. **MEDIUM** Write `test-stage27.ts`, `test-stage28.ts`, `test-stage29.ts`
7. **MEDIUM** Fix `verify:n2` script path
8. **MEDIUM** Standardize `customer_devices` RLS policy to `app.user_id`

After fixing all items, re-run `verify:rls`, `verify:n2`, and `test:phase4` to green before marking GO.
