-- B3 remediation PHASE 2 — cross-tenant system-sweep SECURITY DEFINER maintenance fns.
-- Pattern: a cross-tenant SYSTEM sweep (timeout recovery, reconciliation, settlement) legitimately must act
-- across ALL tenants in one pass. Under NOBYPASSRLS a single connection can't see every tenant, so the
-- cross-tenant write is encapsulated in a DEFINER fn (runs as the fn owner → bypasses RLS *internally only*,
-- for this audited operation), pinned search_path (ITEM1 guardrail), REVOKE PUBLIC + GRANT dowiz_app.
-- The worker calls the fn instead of issuing the raw cross-tenant UPDATE. NEVER worker-role BYPASSRLS.
-- Each fn mirrors EXACTLY the SQL the worker runs today (verified against the worker file).

-- ── order-timeout-sweep (EXEMPLAR) ──
-- Mirrors OrderTimeoutSweepWorker.run() (2): the guarded PENDING→CANCELLED recovery + the audit-history
-- insert, atomic, RETURNING the rows the worker needs for bus/notify. The WHERE status='PENDING' guard is
-- the transition authority (unchanged). order_status_history insert folded in (it was best-effort per-row;
-- here it is one set-based insert, same actor 'system:timeout').
CREATE OR REPLACE FUNCTION app_sweep_timeout_orders()
  RETURNS TABLE(id uuid, location_id uuid)
  LANGUAGE sql SECURITY DEFINER
  SET search_path = pg_catalog, public, pg_temp AS $fn$
    WITH cancelled AS (
      UPDATE orders SET status = 'CANCELLED', timeout_at = NULL
       WHERE status = 'PENDING' AND timeout_at IS NOT NULL AND timeout_at < now()
       RETURNING id, location_id
    ), hist AS (
      INSERT INTO order_status_history (order_id, location_id, from_status, to_status, actor)
      SELECT id, location_id, 'PENDING', 'CANCELLED', 'system:timeout' FROM cancelled
    )
    SELECT id, location_id FROM cancelled
  $fn$;
REVOKE ALL ON FUNCTION app_sweep_timeout_orders() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION app_sweep_timeout_orders() TO dowiz_app;
