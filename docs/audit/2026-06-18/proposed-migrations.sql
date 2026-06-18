-- Proposed migrations for the 2026-06-18 deep-check (D1, D5).
-- These touch packages/db/migrations, which is governance-protected and needs
-- MANUAL APPROVAL (per the f446539 commit note). Do NOT auto-apply — review,
-- then create the tracked node-pg-migrate file via `pnpm migrate:create`.

-- ============================================================================
-- D1 — courier_payouts.paid_at  [SAFE, additive, apply first]
-- ----------------------------------------------------------------------------
-- owner/settlements.ts references paid_at (SELECT at :30/:64 and
-- `UPDATE courier_payouts SET status='paid', paid_at = now()` at :179), but NO
-- committed migration defines it. Per commit f446539 the column was added to the
-- LIVE DB out-of-band, so prod works but a fresh rebuild / new environment fails
-- owner settlements with: column "paid_at" does not exist.
-- This restores parity. Idempotent; matches the live column.

ALTER TABLE courier_payouts ADD COLUMN IF NOT EXISTS paid_at timestamptz;

-- (Optional) backfill paid_at for rows already marked paid, if any predate the column:
-- UPDATE courier_payouts SET paid_at = approved_at WHERE status = 'paid' AND paid_at IS NULL;


-- ============================================================================
-- D5 — operational-pool write grants  [INVESTIGATE FIRST — do NOT blind-apply]
-- ----------------------------------------------------------------------------
-- Migration 1790000000015 created `deliveryos_operational_user` with LOGIN
-- NOBYPASSRLS and granted SELECT ONLY ("operational queries are read-only"),
-- with a note to switch DATABASE_URL_OPERATIONAL to it. But the hot path WRITES
-- through the operational pool (POST /orders, status transitions, courier
-- assignment), and createOperationalPool() actively destroys any connection
-- authenticating as `postgres`. So the role DATABASE_URL_OPERATIONAL actually
-- uses today is some OTHER, write-capable, non-postgres role that is NOT defined
-- by these migrations — i.e. its privileges are unmanaged/out-of-band.
--
-- STEP 1 (no SQL): confirm the real role:
--     SELECT current_user;   -- run on a connection from DATABASE_URL_OPERATIONAL
-- and inspect its grants:
--     \du   and   SELECT * FROM information_schema.role_table_grants
--                 WHERE grantee = '<that role>' LIMIT 50;
--
-- STEP 2: bring that role under migration management. IF (and only if)
-- deliveryos_operational_user is meant to be the operational role, it needs DML.
-- Prefer SCOPED grants over blanket ALL TABLES (the operational pool should not
-- be able to write audit-only / append-only tables). Blanket form shown for
-- reference only:
--
--   GRANT INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public
--     TO deliveryos_operational_user;
--   GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public
--     TO deliveryos_operational_user;
--   ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public
--     GRANT INSERT, UPDATE, DELETE ON TABLES TO deliveryos_operational_user;
--   ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public
--     GRANT USAGE, SELECT ON SEQUENCES TO deliveryos_operational_user;
--
-- NOTE: NOBYPASSRLS means RLS policies still apply after granting DML, so tenant
-- isolation is preserved — but verify with packages/db/scripts/verify-rls.ts
-- (which must first be extended to exercise the app.current_tenant regime; see
-- D7 in the deep-check report).
