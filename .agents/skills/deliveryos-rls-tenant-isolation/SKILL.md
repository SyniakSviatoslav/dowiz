---
name: deliveryos-rls-tenant-isolation
description: >-
  DeliveryOS RLS & tenant-isolation contract. ALWAYS load BEFORE creating or altering a Postgres
  table, migration, query, or data-access path, or reviewing schema/SQL — even if RLS isn't mentioned.
  Enforces: RLS enabled + FORCE on every tenant table, tenant context via SET LOCAL, parameterized
  SQL only, and the documented whitelist (`exchange_rates`, `users`, `ops_worker_heartbeat`,
  `auth_refresh_tokens`). Ships a catalog checker (`scripts/check-rls.mjs`) — run it on every
  migration or schema change before declaring done. Violations: new table without RLS+FORCE, query
  without tenant context, unparameterized SQL, undocumented exception.
---

# DeliveryOS RLS & Tenant-Isolation Contract

## Core Invariants (🔴 never violated)

### 1. Every tenant table: RLS enabled + FORCE
- **Every** table that holds tenant-scoped data must have `ALTER TABLE <name> FORCE ROW LEVEL SECURITY` in its migration.
- `FORCE` ensures RLS applies even when the table owner queries.
- Tenant-isolation is enforced at the Postgres level, not the application level.
- The API role (`deliveryos_api_user`) has `NOBYPASSRLS`.

### 2. Tenant context via SET LOCAL
- All queries run under `SET LOCAL app.tenant_id = <uuid>` (set per-request in Fastify hooks).
- RLS policies filter by `app.tenant_id` using `current_setting('app.tenant_id', true)`.
- **Never** pass `location_id` as a raw filter in application code as a substitute for RLS.

### 3. Parameterized SQL only
- All SQL uses parameterized queries (`$1`, `$2`, ...).
- **Never** concatenate user input into SQL strings.
- Even in migrations: use `pgm.sql()` with node-pg-migrate helpers where possible.

### 4. Whitelisted non-tenant tables (documented exceptions)
These tables are intentionally non-tenant and do NOT require RLS:
- `exchange_rates` — global reference table (EUR conversion rates, read-only)
- `users` — authentication/identity (non-tenant)
- `ops_worker_heartbeat` — operational/infrastructure
- `auth_refresh_tokens` — authentication tokens (non-tenant)

Any new non-tenant table must be added to this whitelist explicitly.

## Checklist (run against every migration/schema diff)

- [ ] New tenant table has `ALTER TABLE ... FORCE ROW LEVEL SECURITY`?
- [ ] New non-tenant table is whitelisted with explicit comment?
- [ ] All queries use parameterized SQL (`$N` placeholders)?
- [ ] Query runs under tenant context (`SET LOCAL app.tenant_id`)?
- [ ] RLS policies reference `current_setting('app.tenant_id', true)`?
- [ ] `deliveryos_api_user` has `NOBYPASSRLS`?

## Check Script

```bash
node .agents/skills/deliveryos-rls-tenant-isolation/scripts/check-rls.mjs [path]
```

Run without arguments to scan all migration files, or pass a specific migration file to check.
