# STOP-DESIGN-B · Step 1 — BYPASSRLS availability probe (result)

> Resolves the branch choice deferred to STOP-DESIGN-B / R-BR24 / R-ROLE.
> Decides whether the notif-worker escape deploys as **(a)** a `BYPASSRLS` role or **(b)** `policy TO deliveryos_notif_worker`.
> Date: 2026-06-22 · Owner: Backend/Infra+Arch

## Result: **Branch (b) — `policy TO <role>` (no BYPASSRLS).** Branch (a) ruled out.

The live empirical probe could **not** be executed from this sandbox: no `flyctl` (can't proxy staging DB), and the local `.env` (PROD, eu-central-1 Supabase) connects through Supavisor behind a `vestauth` secret proxy — every `psql` attempt returns Supavisor's auth-failure stand-in `FATAL: database "postgres" does not exist`. So the determination is made from **convergent platform + repo evidence** instead, with a live confirmation step specified below for whoever has staging credentials.

## Evidence (three independent, convergent lines)

1. **Supabase `postgres` is not a superuser.** Managed Supabase strips `SUPERUSER` from the `postgres` role (the role behind `***REDACTED***`, user `postgres.elxukhxvuycnftqwaghg`).
2. **PostgreSQL 16 attribute-grant rule.** A `CREATEROLE` role may confer `BYPASSRLS` on a new/existing role **only if it itself holds `BYPASSRLS`**. Supabase `postgres` holds `CREATEROLE`/`CREATEDB` but **not** `BYPASSRLS` → `CREATE ROLE deliveryos_notif_worker … BYPASSRLS` (and `ALTER ROLE … BYPASSRLS`) executed as the migrations role fails with `permission denied to create role with bypassrls` (or the ALTER equivalent).
3. **Repo proof — the platform already rejected this exact grant.** `packages/db/migrations/1780691681296_ops-location-alerts-policy.ts` runs `ALTER ROLE deliveryos_api_user BYPASSRLS` wrapped in `EXCEPTION WHEN OTHERS THEN -- Ignore if not allowed`. The council's **BR-1** independently verified (against live behaviour) that `deliveryos_api_user` does **not** effectively bypass RLS — i.e. that ALTER **silently failed in prod**. A swallowed-failure migration is only written because the author already observed the platform refusing the grant. This is the BR-1 anti-pattern itself, and it is direct evidence the migrations role cannot confer `BYPASSRLS`.

→ The same role runs the new `…052` migration, so branch (a)'s `CREATE ROLE … BYPASSRLS` will fail identically. **Branch (b) is mandatory.**

## Consequence for implementation (locks prior design openings)

- **`…052_notif-worker-role.ts`** creates `deliveryos_notif_worker` as a **plain LOGIN role (NO BYPASSRLS)** + least-privilege grants (per proposal §5.5).
- **Every RLS policy on the 4 new/touched tables** (`owner_notification_targets`, `notification_outbox_audit`, `notification_prefs_audit`, `telegram_action_nonces`) gets an explicit **`… TO deliveryos_notif_worker USING (true)`** escape clause (role-bound, not self-grantable; web role `deliveryos_api_user` can never match it).
- **Boot-assert = BR-24 functional variant** (NOT the attributive `rolbypassrls` check, which is legitimately `false` here): on the notif-worker pool, `SELECT 1 FROM owner_notification_targets WHERE location_id = $seedLoc` under FORCE **without any GUC** → row present, else FATAL-exit. Proves the `TO role` escape actually works, rather than assuming it.
- **`SET ROLE` stays forbidden** on the operational pool; notif-worker is a separate pool + credential (`DATABASE_URL_NOTIF_WORKER`), never a `SET ROLE` shortcut.
- Branch (a) code path is **not shipped**; the boot-mode selector can hard-fail if `rolbypassrls=true` is ever unexpectedly observed (signals platform change → revisit).
- **Cleanup:** the dead swallowed-BYPASSRLS attempt in `1780691681296` for `deliveryos_api_user` should be left as-is (forward-only) but noted — `deliveryos_api_user` must NOT gain bypass; FORCE + per-table `app.user_id` policy is the tenant guarantee.

## Empirical confirmation (run on STAGING first, then PROD pre-deploy)

Anyone with staging DB access (Supabase SQL editor, or `! psql "$STAGING_MIGRATIONS_URL"`) should run this **read-only** check to confirm before the `…052` migration deploys:

```sql
-- 1. Does the migrations role itself hold superuser / bypassrls? (branch-(a) viability)
SELECT rolname, rolsuper, rolcreaterole, rolbypassrls
FROM pg_roles WHERE rolname = current_user;
-- Expected on Supabase: rolsuper=f, rolcreaterole=t, rolbypassrls=f  → branch (b)

-- 2. Confirmatory: attempt the grant in a ROLLBACK'd probe txn (no lasting change).
BEGIN;
  DO $$ BEGIN
    EXECUTE 'CREATE ROLE _probe_bypassrls_xyz NOLOGIN BYPASSRLS';
    RAISE NOTICE 'BYPASSRLS GRANT SUCCEEDED → branch (a) viable';
  EXCEPTION WHEN insufficient_privilege OR others THEN
    RAISE NOTICE 'BYPASSRLS GRANT REJECTED (%) → branch (b) required', SQLERRM;
  END $$;
ROLLBACK;   -- discards the probe role regardless
```

If line 1 shows `rolbypassrls=t` (or `rolsuper=t`) **and** line 2 reports SUCCEEDED, branch (a) becomes available and this determination should be revisited. Otherwise branch (b) stands.
