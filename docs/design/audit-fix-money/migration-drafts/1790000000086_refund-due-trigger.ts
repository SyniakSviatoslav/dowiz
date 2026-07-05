// ⛔ OPERATOR ACTION REQUIRED: place this file VERBATIM at
//    packages/db/migrations/1790000000086_refund-due-trigger.ts
// (agent-authored out-of-tree because packages/db/migrations/ is red-line hard-gated — the same
// flow as migrations 078/083). Before ANY apply: run `node scripts/ci-migration-preflight.mjs`
// (SOURCE=prod) and `node scripts/ci-schema-drift.mjs`. Staging DB first per Ship Discipline.
//
// MONEY audit fix M-1 (ADR-audit-fix-money / proposal.md §3.2 L-C) — the refund_due STRUCTURAL
// FLOOR: an AFTER UPDATE OF status trigger on orders that records the refund obligation for every
// paid payment whenever ANY writer (funnel, DEFINER sweep fn, raw UPDATE, future sinner) moves an
// order into CANCELLED/REJECTED. Covers the writers the app-fold (L-A) cannot see; its misses are
// deterministically caught by the L-D reconciler (M-3 / 1790000000087).
//
// ── N8 (breaker-findings-r2): THIS TRIGGER IS DELIBERATELY NON-THROWING ──────────────────────
// It INVERTS the repo's trigger convention (prevent_cash_mutation / prevent_payout_mutation /
// orders_promised_window_set_once all RAISE EXCEPTION to enforce). Here the body swallows every
// error (BEGIN…EXCEPTION WHEN OTHERS → RAISE WARNING) BY DESIGN: this trigger fires inside
// app_sweep_timeout_orders()'s fleet-wide atomic CTE — a throwing body would let one poisoned row
// wedge cancellation for every tenant (breaker C1, the v1 CRITICAL). Liveness of any batch must
// never couple to a money-ledger write; the swallow's misses are the L-D reconciler's job, which
// alarms (DRIFT + Sentry + operator alert) until the obligation lands. DO NOT "fix" this to the
// throwing template. The swallow pattern is the SAVEPOINT precedent in orderStatusService.ts:152.
//
// ── N3: subtransaction cost is gated ─────────────────────────────────────────────────────────
// A plpgsql EXCEPTION block opens a subtransaction on EVERY entry. The sweep cancels overwhelmingly
// cash orders (zero paid payments); a timeout-backlog flush would open hundreds of subxacts in one
// xact (pg_subtrans SLRU contention). So the exception-wrapped body runs ONLY behind
// IF EXISTS (SELECT 1 FROM payments WHERE order_id = NEW.id AND status='paid') — cash cancels pay
// one cheap index probe (payments_order_idx), no subtransaction.
//
// ── N5: idempotency does NOT lean on the nullable provider_payment_id unique ─────────────────
// payment_events_idem_unique(provider, provider_payment_id, type) treats NULL provider_payment_id
// as distinct → three concurrent writers (L-A/L-C/L-D) could each insert a refund_due for a
// NULL-ref payment. This migration adds a partial unique on (payment_id) WHERE type='refund_due'
// — at most ONE refund_due obligation per payment, regardless of provider ref. All refund_due
// writers use bare ON CONFLICT DO NOTHING (arbiter-less → any unique conflict is a no-op).
//
// ── N6: GUC save/restore semantics (EMPIRICALLY VERIFIED on PostgreSQL 16.14) ────────────────
// save := current_setting('app.current_tenant', true) reads NULL when unset. A true-NULL RESTORE
// via set_config(name, NULL, true) is IMPOSSIBLE: PG coerces the NULL to '' — verified 2026-07-03
// on PG 16.14 (current_setting(...) IS NULL → false, = '' → true after set_config(NULL)). So:
//   • success path: restore COALESCE(saved, '') — '' is semantically identical to unset for EVERY
//     GUC consumer in this schema: all dual policies read NULLIF(current_setting(...,true),'')::uuid
//     ('' → NULL → arm inert) and all legacy strict policies read current_setting(...)::uuid which
//     hard-errors on BOTH unset (42704) and '' (22P02) — fail-closed either way. Audited via
//     `grep current_setting('app.current_tenant'` across packages/db/migrations 2026-07-03.
//   • error path: NO restore call — the EXCEPTION subtransaction rollback restores the EXACT
//     pre-block GUC state (including true unset), which is strictly better than any explicit set.
//   • session residue (also PG16-verified): after the tx COMMITs, the custom GUC placeholder stays
//     DEFINED on that pooled connection and reads '' (never the tenant value) for the rest of the
//     session — same residue class the webhook precedent (payments-webhook.ts:41) already leaves;
//     harmless per the consumer audit above. Pinned by tests/refund-due-spine.test.ts (N6).
//
// DEFINER guardrail: SECURITY DEFINER + pinned search_path (ITEM1). payment_events is FORCE RLS,
// so even the fn owner needs the tenant GUC arm of the dual policy (1790000000083:77-81) — that is
// what the per-row set_config dance is for; works pre- and post-B3 with no BYPASSRLS dependency.
// Recursion-safe: the body only INSERTs into payment_events, never UPDATEs orders.
// Forward-only. Trigger fns cannot be invoked via SQL, so no GRANT EXECUTE is needed; REVOKE ALL
// FROM PUBLIC re-emitted per the 078 pattern anyway.
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    -- N5: one refund_due obligation per payment, immune to NULL provider_payment_id.
    -- Safe on existing data: the single historical writer (deliveryCompletion.ts §5) was keyed by
    -- the (provider, provider_payment_id, type) unique with per-payment-constant values, and the
    -- crypto vertical is dark (flags off) — no duplicate rows can exist.
    CREATE UNIQUE INDEX IF NOT EXISTS payment_events_refund_due_per_payment
      ON payment_events (payment_id) WHERE type = 'refund_due';

    CREATE OR REPLACE FUNCTION app_refund_due_on_terminal() RETURNS trigger
      LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
    DECLARE
      v_saved_tenant text;
    BEGIN
      -- N3 gate: only orders that actually carry a paid payment pay the subtransaction cost.
      IF EXISTS (SELECT 1 FROM payments p WHERE p.order_id = NEW.id AND p.status = 'paid') THEN
        v_saved_tenant := current_setting('app.current_tenant', true); -- NULL when unset (sweep path)
        BEGIN
          PERFORM set_config('app.current_tenant', NEW.location_id::text, true);
          INSERT INTO payment_events
            (payment_id, location_id, provider, provider_payment_id, type, amount_minor, currency_code, signature_verified)
          SELECT p.id, p.location_id, p.provider, p.provider_payment_id, 'refund_due', p.amount_minor, p.currency_code, true
            FROM payments p WHERE p.order_id = NEW.id AND p.status = 'paid'
          ON CONFLICT DO NOTHING;  -- arbiter-less: idem_unique OR refund_due_per_payment (N5)
          -- N6 success-path restore: '' ≡ unset for every consumer (see header; true NULL cannot
          -- be restored via set_config — PG16-verified).
          PERFORM set_config('app.current_tenant', COALESCE(v_saved_tenant, ''), true);
        EXCEPTION WHEN OTHERS THEN
          -- N8: DELIBERATELY non-throwing (see header). The subxact rollback has already restored
          -- the exact pre-block GUC state (incl. true unset) — no explicit restore here (N6).
          RAISE WARNING 'app_refund_due_on_terminal swallowed (order %, loc %): % — L-D reconciler will record or alarm',
            NEW.id, NEW.location_id, SQLERRM;
        END;
      END IF;
      RETURN NULL; -- AFTER trigger: return value ignored
    END $fn$;
    REVOKE ALL ON FUNCTION app_refund_due_on_terminal() FROM PUBLIC;

    DROP TRIGGER IF EXISTS trg_orders_refund_due_on_terminal ON orders;
    CREATE TRIGGER trg_orders_refund_due_on_terminal
      AFTER UPDATE OF status ON orders
      FOR EACH ROW
      WHEN (NEW.status IN ('CANCELLED', 'REJECTED') AND OLD.status IS DISTINCT FROM NEW.status)
      EXECUTE FUNCTION app_refund_due_on_terminal();
  `);
}

export async function down(): Promise<void> {
  // Forward-only per migration discipline.
}
