// ⛔ OPERATOR ACTION REQUIRED: place this file VERBATIM at
//    packages/db/migrations/1790000000087_refund-due-reconciler.ts
// (agent-authored out-of-tree because packages/db/migrations/ is red-line hard-gated — the same
// flow as migrations 078/083). Before ANY apply: run `node scripts/ci-migration-preflight.mjs`
// (SOURCE=prod) and `node scripts/ci-schema-drift.mjs`. Staging DB first per Ship Discipline.
// NOTE: the OrderTimeoutSweepWorker calls app_reconcile_refund_due() each tick — deploy the code
// only after (or together with) this migration, or wrap-guard failures are logged (the worker
// call is try/caught, so a missing fn degrades to a logged error, never a crashed sweep).
//
// MONEY audit fix M-3 (ADR-audit-fix-money / proposal.md §3.2 L-D) — app_reconcile_refund_due():
// the DETERMINISTIC ALARM OF LAST RESORT for the LC6 refund invariant (two-tier, ADR-0017 C2):
// any terminal non-fulfilled (CANCELLED/REJECTED) order whose payment is 'paid' must carry a
// refund_due obligation within ≤1 reconciler tick (~60s), or a surfaced operator alert exists.
// Called by the existing timeout-sweep worker every tick AFTER the sweep. Returns one row per
// action so the worker can emit DRIFT counters + Sentry + the operator-visible alert:
//   o_action='inserted' — an obligation another layer (L-A fold / L-B webhook / L-C trigger)
//                         missed, recorded now (the miss itself is drift-worthy);
//   o_action='failed'   — the insert failed persistently (o_detail=SQLERRM) → alarms EVERY tick
//                         until resolved: un-recorded but never un-alarmed (proof P15);
//   o_action='mismatch' — P16: a terminal order carrying a 'mismatch' payment event (over/under-
//                         paid crypto on a dead order) — SURFACED, never auto-obligated: the
//                         obligation amount is ambiguous and mis-stating a money obligation is
//                         worse than alert-plus-human (M3 scope; follow-up: M3-mismatch-disposition).
//
// Per-row BEGIN/EXCEPTION isolation: one poisoned row never blocks the rest. GUC dance per row —
// same N6 semantics as M-1/1790000000086 (success path restores COALESCE(saved,''), which is
// semantically identical to unset for every consumer; error path relies on the subxact rollback
// restoring the exact pre-block state — PG16-verified, see the M-1 header).
// Bounded scan: payments(status='paid') driven through payments_order_idx; LIMIT 500 per tick
// (a healthy system has ~0 rows here; 500 bounds a pathological backlog without starving — the
// remainder lands next tick). DEFINER + pinned search_path (ITEM1); REVOKE/GRANT per 078 pattern.
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE OR REPLACE FUNCTION app_reconcile_refund_due()
      RETURNS TABLE(o_order_id uuid, o_payment_id uuid, o_action text, o_detail text)
      LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
    DECLARE
      v_row record;
      v_saved_tenant text;
    BEGIN
      FOR v_row IN
        SELECT p.id AS payment_id, p.order_id, p.location_id, p.provider,
               p.provider_payment_id, p.amount_minor, p.currency_code
        FROM payments p
        JOIN orders o ON o.id = p.order_id
        WHERE p.status = 'paid'
          AND o.status IN ('CANCELLED', 'REJECTED')
          AND NOT EXISTS (SELECT 1 FROM payment_events e WHERE e.payment_id = p.id AND e.type = 'refund_due')
        LIMIT 500
      LOOP
        v_saved_tenant := current_setting('app.current_tenant', true);
        BEGIN
          PERFORM set_config('app.current_tenant', v_row.location_id::text, true);
          INSERT INTO payment_events
            (payment_id, location_id, provider, provider_payment_id, type, amount_minor, currency_code, signature_verified)
          VALUES (v_row.payment_id, v_row.location_id, v_row.provider, v_row.provider_payment_id,
                  'refund_due', v_row.amount_minor, v_row.currency_code, true)
          ON CONFLICT DO NOTHING; -- arbiter-less (N5): any unique conflict is a benign race with L-A/L-C
          PERFORM set_config('app.current_tenant', COALESCE(v_saved_tenant, ''), true); -- N6 success restore
          o_order_id := v_row.order_id; o_payment_id := v_row.payment_id;
          o_action := 'inserted'; o_detail := NULL;
          RETURN NEXT;
        EXCEPTION WHEN OTHERS THEN
          -- Per-row isolation: subxact rollback already restored the exact pre-block GUC state (N6).
          o_order_id := v_row.order_id; o_payment_id := v_row.payment_id;
          o_action := 'failed'; o_detail := SQLERRM;
          RETURN NEXT;
        END;
      END LOOP;

      -- P16: mismatch-class terminal orders — surfaced every tick until disposed (refund_due or
      -- refund_sent recorded by the owner flow), NEVER auto-obligated (M3 scope).
      RETURN QUERY
        SELECT o.id, p.id, 'mismatch'::text,
               ('provider=' || p.provider || ' ref=' || COALESCE(p.provider_payment_id, '(none)'))::text
        FROM orders o
        JOIN payments p ON p.order_id = o.id
        WHERE o.status IN ('CANCELLED', 'REJECTED')
          AND EXISTS (SELECT 1 FROM payment_events e WHERE e.payment_id = p.id AND e.type = 'mismatch')
          AND NOT EXISTS (SELECT 1 FROM payment_events e2 WHERE e2.payment_id = p.id AND e2.type IN ('refund_due', 'refund_sent'));
    END $fn$;
    REVOKE ALL ON FUNCTION app_reconcile_refund_due() FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_reconcile_refund_due() TO dowiz_app;
  `);
}

export async function down(): Promise<void> {
  // Forward-only per migration discipline.
}
