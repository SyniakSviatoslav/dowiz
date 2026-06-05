# Hardening Audit — Non-Tenant API Surface Lockdown

**Date:** 2026-06-05
**Finding:** Supabase Security Advisor `0013_rls_disabled_in_public`
**Tables:** `users`, `ops_worker_heartbeat`, `auth_refresh_tokens`
**Migration:** `1780421100065_lockdown-nontenant-api-surface.ts`

---

## Pre-Fix State (inventory)

### RLS state
| Table | RLS State | Data API Grants | Risk |
|-------|-----------|-----------------|------|
| `users` | **Never configured** (defaults OFF) | `anon`/`authenticated`/`service_role` have full SELECT/INSERT/UPDATE/DELETE via auto-grants | PII leak: `phone`, `email`, `totp_secret_enc`; account takeover via `auth_refresh_tokens` FK chain |
| `ops_worker_heartbeat` | Explicitly DISABLED | Same auto-grants | Infrastructure visibility (worker status, instance IDs) |
| `auth_refresh_tokens` | Explicitly DISABLED | Same auto-grants | Token hijack if `token_hash` readable via Data API |

### App access patterns (grep audit)
All three tables are accessed **exclusively during unauthenticated flows**:
- `users`: INSERT/UPDATE/SELECT in Google OAuth callback, courier activation, token refresh
- `auth_refresh_tokens`: INSERT/SELECT/UPDATE/DELETE in `/auth/refresh`, `/auth/google/callback`, `/auth/courier/activate`
- `ops_worker_heartbeat`: INSERT from heartbeat workers, SELECT from `/health` (unauthenticated)

**No `app.user_id` is set during any of these queries.** The `withTenant()` wrapper is never used for these tables.

---

## Fix Applied (migration `1780421100065`)

### STEP A1 — Revoke Data API grants
```
REVOKE ALL PRIVILEGES ON users, ops_worker_heartbeat, auth_refresh_tokens FROM anon, authenticated, service_role
```

### STEP A2 — ENABLE + FORCE RLS (deny-by-default, no policies)
```
ALTER TABLE users/ops_worker_heartbeat/auth_refresh_tokens ENABLE ROW LEVEL SECURITY
ALTER TABLE users/ops_worker_heartbeat/auth_refresh_tokens FORCE  ROW LEVEL SECURITY
```
No permissive policies added. App roles with `BYPASSRLS` (operational/session pools) continue to access normally. All other roles (anon/authenticated/service_role) are denied — both because of revoked grants (A1) and FORCE RLS (A2).

### STEP A3 — Default privileges opt-in
```
ALTER DEFAULT PRIVILEGES IN SCHEMA public REVOKE ALL ON TABLES/SEQUENCES/FUNCTIONS FROM anon, authenticated
```
Future tables will not auto-expose to Data API.

### STEP A4 — Schema access revoke
```
REVOKE USAGE ON SCHEMA public FROM anon, authenticated
```

---

## Post-Fix Verification

### RLS state
| Table | RLS | FORCE | API Grants |
|-------|-----|-------|------------|
| `users` | ENABLED | YES | 0 for anon/authenticated/service_role |
| `ops_worker_heartbeat` | ENABLED | YES | 0 for anon/authenticated/service_role |
| `auth_refresh_tokens` | ENABLED | YES | 0 for anon/authenticated/service_role |

### verify-rls.ts extended
- Checks `zero API grants` for all three tables (query `information_schema.role_table_grants`)
- Checks `relrowsecurity = true` (RLS enabled)
- Checks `relforcerowsecurity = true` (FORCE RLS)

### Regression test checklist
- [ ] Owner Google OAuth login succeeds
- [ ] Token refresh succeeds
- [ ] Courier activation succeeds
- [ ] Worker heartbeat writes succeed
- [ ] `/health` endpoint returns worker status
- [ ] Supabase Security Advisor: zero `0013` findings on these tables
- [ ] `verify:rls` green — all API grant checks pass

---

## Why no RLS policies?

RLS policies on these tables would break all auth flows because:
1. Auth flows (login, signup, refresh) are **unauthenticated** — no JWT, no `app.user_id`
2. `withTenant()` wrapper is never used for these tables
3. Permissive policies (`USING (true)`) would defeat the purpose of RLS

Instead, the defense is **perimeter-based**: Data API roles are completely revoked from these tables. Only app-level DB roles (operational/session pools with BYPASSRLS) can access them. This is the correct architecture per ADR-006 (no PostgREST, custom Fastify pooler).

---

## Owner manual step (outside agent scope)

**Supabase Dashboard → Project Settings → Data API → Disable Data API** (or move default schema from `public` to `api`). This eliminates the entire `0013` class of vulnerabilities for all current and future tables. The migration above serves as belt-and-suspenders in case Data API is re-enabled.
