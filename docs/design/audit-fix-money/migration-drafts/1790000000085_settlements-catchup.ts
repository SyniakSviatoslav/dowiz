// ⛔ OPERATOR ACTION REQUIRED: place this file VERBATIM at
//    packages/db/migrations/1790000000085_settlements-catchup.ts
// (agent-authored out-of-tree because packages/db/migrations/ is red-line hard-gated — the same
// flow as migrations 078/083, whose headers read "operator places into packages/db/migrations/").
// Before ANY apply: run `node scripts/ci-migration-preflight.mjs` (SOURCE=prod) and
// `node scripts/ci-schema-drift.mjs` per docs/lessons/2026-07-03-prod-staging-schema-drift.md.
// Staging DB first per Ship Discipline. Do NOT auto-apply.
//
// MONEY audit fix M-2 (ADR-audit-fix-money / docs/design/audit-fix-money/proposal.md §4) —
// settlement generation becomes CATCH-UP, IMMUTABLE-ONCE-PAID, IDEMPOTENT. Forward-only
// CREATE OR REPLACE of app_generate_settlements (exact same signature/return per breaker L2),
// plus the operator-gated historical backfill pair and the additive `backfilled` flag column.
//
// ── ⚠️ N2 WATERMARK PIN — RE-CHECK AT APPLY TIME (breaker-findings-r2 N2) ─────────────────────
// The catch-up scan is lower-bounded by the LITERAL deploy watermark `2026-07-10 00:00:00+00`
// baked into the fn bodies below. That literal MUST be >= the ACTUAL prod apply date:
//   • erring LATE  (literal after the real deploy)  → SAFE: rows in the gap merely defer to the
//     operator backfill flow (app_backfill_historical_settlements) instead of self-healing;
//   • erring EARLY (literal before the real deploy) → DOUBLE-PAYS: rows processed by the OLD
//     buggy fn while it was still live (SKIP-LOCKED-dropped and plausibly reconciled in person)
//     would be auto-swept into a fresh pending payout — the exact C2 hazard this fix closes.
// If prod apply slips past 2026-07-10, the operator MUST bump the literal (all THREE occurrences
// below: generate pair-scan + item-scan, precount upper bound, backfill upper bound) BEFORE apply.
//
// What changed vs 1790000000078 (verified against its body):
//   1. Pair discovery + item selection scan `delivered_at >= WATERMARK` (catch-up) instead of
//      `>= p_period_start` — any post-watermark row missed by a run (SKIP LOCKED skip, crashed
//      2 AM job, whole missed day) is swept by the NEXT run. Period params become a LABEL for
//      new payouts, never a filter that can lose money. SKIP LOCKED is KEPT (a skip is now a
//      deferral, not a loss). Pre-watermark rows are NEVER touched by cron (proof P9b).
//   2. Paid-payout immutability: payout row is locked FOR UPDATE after the upsert; if
//      status <> 'pending' the pair is SKIPPED this run (items stay unsettled and roll into the
//      next period's fresh pending payout). A paid payout's numbers never move again (P10).
//   3. Aggregate-recompute totals: deliveries_count/total_earned are recomputed as aggregates
//      over settlement_items (idempotent; immune to the ON-CONFLICT-DO-NOTHING phantom-count
//      bug in the old incremental bump) — double-guarded by status='pending' (P11).
//   4. Single-flight: pg_advisory_xact_lock(hashtext('app_generate_settlements')) serializes
//      cron + /regenerate + the operator backfill (same key).
//   5. Historical backfill (C2 fix): pre-watermark rows move ONLY through the operator-gated
//      app_backfill_historical_settlements() — NEVER called by cron or any worker (P13) — after
//      reviewing app_backfill_precount_settlements() (read-only magnitude report). Backfilled
//      items carry settlement_items.backfilled = true and land on 'pending' payouts labelled
//      [epoch, WATERMARK); /pay stays human-gated. No path auto-pays historical cash.
//
// DEFINER guardrail: every fn is SECURITY DEFINER with pinned search_path (ITEM1) and re-emits
// REVOKE ALL FROM PUBLIC + GRANT EXECUTE TO dowiz_app per the 078 pattern (breaker L2).
// DEP-2 note: these fns' cross-tenant writes remain pre-B3 semantics — post-B3 RLS strategy is a
// B3-flip checklist item (resolution.md L1), NOT changed here.
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    -- Additive flag: marks operator-backfilled (pre-watermark) settlement items so owner UI and
    -- courier view can render "N caught-up deliveries from before <fix-date>" (proposal §4.1.1b).
    ALTER TABLE settlement_items ADD COLUMN IF NOT EXISTS backfilled boolean NOT NULL DEFAULT false;

    -- Perf (optional-but-cheap): keeps the catch-up anti-join bounded as history grows.
    CREATE INDEX IF NOT EXISTS courier_assignments_unsettled_cash_idx
      ON courier_assignments (courier_id, location_id, delivered_at)
      WHERE status = 'delivered' AND cash_collected = true AND settlement_item_id IS NULL;

    CREATE OR REPLACE FUNCTION app_generate_settlements(p_period_start timestamptz, p_period_end timestamptz)
      RETURNS void LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
    DECLARE
      -- N2 watermark literal — must be >= actual prod apply date (see migration header).
      c_watermark CONSTANT timestamptz := '2026-07-10 00:00:00+00';
      v_pair record; v_payout_id uuid; v_payout_status text; v_item record; v_si_id uuid;
      v_added_items integer; v_added_total integer;
    BEGIN
      -- (4) single-flight across cron / manual /regenerate / operator backfill.
      PERFORM pg_advisory_xact_lock(hashtext('app_generate_settlements'));
      FOR v_pair IN
        SELECT DISTINCT ca.courier_id, ca.location_id FROM courier_assignments ca
        WHERE ca.status = 'delivered' AND ca.cash_collected = true
          AND ca.delivered_at >= c_watermark AND ca.delivered_at < p_period_end
          AND NOT EXISTS (SELECT 1 FROM settlement_items si WHERE si.assignment_id = ca.id)
      LOOP
        INSERT INTO courier_payouts (courier_id, location_id, period_start, period_end, status)
        VALUES (v_pair.courier_id, v_pair.location_id, p_period_start, p_period_end, 'pending')
        ON CONFLICT (courier_id, location_id, period_start, period_end) DO UPDATE SET status = courier_payouts.status
        RETURNING id INTO v_payout_id;
        -- (2) paid-payout immutability: lock, then skip the whole pair unless still pending.
        SELECT status INTO v_payout_status FROM courier_payouts WHERE id = v_payout_id FOR UPDATE;
        IF v_payout_status <> 'pending' THEN
          CONTINUE; -- unsatisfied items stay unsettled (NOT EXISTS still true) → next period's payout
        END IF;
        v_added_items := 0; v_added_total := 0;
        FOR v_item IN
          SELECT ca.id, ca.cash_amount, loc.currency_code FROM courier_assignments ca
          JOIN locations loc ON loc.id = ca.location_id
          WHERE ca.courier_id = v_pair.courier_id AND ca.location_id = v_pair.location_id
            AND ca.status = 'delivered' AND ca.cash_collected = true
            AND ca.delivered_at >= c_watermark AND ca.delivered_at < p_period_end
            AND NOT EXISTS (SELECT 1 FROM settlement_items si WHERE si.assignment_id = ca.id)
          FOR UPDATE OF ca SKIP LOCKED   -- KEPT: a skip is a deferral now, not a loss (P8)
        LOOP
          v_si_id := NULL;
          INSERT INTO settlement_items (payout_id, assignment_id, location_id, amount, currency_code)
          VALUES (v_payout_id, v_item.id, v_pair.location_id, v_item.cash_amount, v_item.currency_code)
          ON CONFLICT (assignment_id) DO NOTHING
          RETURNING id INTO v_si_id;
          UPDATE courier_assignments
             SET settlement_item_id = (SELECT si.id FROM settlement_items si WHERE si.assignment_id = v_item.id)
           WHERE id = v_item.id;
          -- count only genuine inserts (audit metadata) — no phantom counts on conflict.
          IF v_si_id IS NOT NULL THEN
            v_added_items := v_added_items + 1; v_added_total := v_added_total + v_item.cash_amount;
          END IF;
        END LOOP;
        -- (3) aggregate recompute — idempotent by construction; guarded pending-only.
        UPDATE courier_payouts p
           SET deliveries_count = (SELECT count(*)::int FROM settlement_items si WHERE si.payout_id = p.id),
               total_earned     = (SELECT COALESCE(sum(si.amount), 0)::int FROM settlement_items si WHERE si.payout_id = p.id)
         WHERE p.id = v_payout_id AND p.status = 'pending';
        IF v_added_items > 0 THEN
          INSERT INTO settlement_audit_log (payout_id, location_id, action, actor_kind, metadata)
          VALUES (v_payout_id, v_pair.location_id, 'generated', 'system',
                  jsonb_build_object('added_items', v_added_items, 'added_total', v_added_total));
        END IF;
      END LOOP;
    END $fn$;
    REVOKE ALL ON FUNCTION app_generate_settlements(timestamptz, timestamptz) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_generate_settlements(timestamptz, timestamptz) TO dowiz_app;

    -- Read-only PRE-COUNT (operator gate 1 of 3): magnitude report of pre-watermark eligible rows
    -- per courier×location pair — reviewed BEFORE anything is created (proposal §4.1.1b).
    CREATE OR REPLACE FUNCTION app_backfill_precount_settlements()
      RETURNS TABLE(o_courier_id uuid, o_location_id uuid, o_eligible_items bigint, o_eligible_total bigint)
      LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        SELECT ca.courier_id, ca.location_id, count(*)::bigint, COALESCE(sum(ca.cash_amount), 0)::bigint
        FROM courier_assignments ca
        WHERE ca.status = 'delivered' AND ca.cash_collected = true
          AND ca.delivered_at < '2026-07-10 00:00:00+00'::timestamptz  -- N2 watermark literal
          AND NOT EXISTS (SELECT 1 FROM settlement_items si WHERE si.assignment_id = ca.id)
        GROUP BY ca.courier_id, ca.location_id
      $fn$;
    REVOKE ALL ON FUNCTION app_backfill_precount_settlements() FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_backfill_precount_settlements() TO dowiz_app;

    -- OPERATOR-ONLY historical backfill (gate 2 of 3; /pay stays gate 3). NEVER called by cron or
    -- any worker (P13 asserts no code path reaches it). Creates 'pending' payouts labelled
    -- [epoch, WATERMARK) with backfilled=true items. The operator MAY legitimately backfill only a
    -- subset or none (pairs already reconciled in person → documented-no-action in the ops record).
    CREATE OR REPLACE FUNCTION app_backfill_historical_settlements()
      RETURNS TABLE(o_courier_id uuid, o_location_id uuid, o_items_added integer, o_total_added integer)
      LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
    DECLARE
      c_watermark CONSTANT timestamptz := '2026-07-10 00:00:00+00'; -- N2 watermark literal
      c_epoch     CONSTANT timestamptz := '1970-01-01 00:00:00+00';
      v_pair record; v_payout_id uuid; v_payout_status text; v_item record; v_si_id uuid;
    BEGIN
      PERFORM pg_advisory_xact_lock(hashtext('app_generate_settlements')); -- serialize vs cron
      FOR v_pair IN
        SELECT DISTINCT ca.courier_id, ca.location_id FROM courier_assignments ca
        WHERE ca.status = 'delivered' AND ca.cash_collected = true
          AND ca.delivered_at < c_watermark
          AND NOT EXISTS (SELECT 1 FROM settlement_items si WHERE si.assignment_id = ca.id)
      LOOP
        INSERT INTO courier_payouts (courier_id, location_id, period_start, period_end, status)
        VALUES (v_pair.courier_id, v_pair.location_id, c_epoch, c_watermark, 'pending')
        ON CONFLICT (courier_id, location_id, period_start, period_end) DO UPDATE SET status = courier_payouts.status
        RETURNING id INTO v_payout_id;
        SELECT status INTO v_payout_status FROM courier_payouts WHERE id = v_payout_id FOR UPDATE;
        IF v_payout_status <> 'pending' THEN CONTINUE; END IF; -- a paid backfill payout never mutates
        o_courier_id := v_pair.courier_id; o_location_id := v_pair.location_id;
        o_items_added := 0; o_total_added := 0;
        FOR v_item IN
          SELECT ca.id, ca.cash_amount, loc.currency_code FROM courier_assignments ca
          JOIN locations loc ON loc.id = ca.location_id
          WHERE ca.courier_id = v_pair.courier_id AND ca.location_id = v_pair.location_id
            AND ca.status = 'delivered' AND ca.cash_collected = true
            AND ca.delivered_at < c_watermark
            AND NOT EXISTS (SELECT 1 FROM settlement_items si WHERE si.assignment_id = ca.id)
          FOR UPDATE OF ca SKIP LOCKED
        LOOP
          v_si_id := NULL;
          INSERT INTO settlement_items (payout_id, assignment_id, location_id, amount, currency_code, backfilled)
          VALUES (v_payout_id, v_item.id, v_pair.location_id, v_item.cash_amount, v_item.currency_code, true)
          ON CONFLICT (assignment_id) DO NOTHING
          RETURNING id INTO v_si_id;
          UPDATE courier_assignments
             SET settlement_item_id = (SELECT si.id FROM settlement_items si WHERE si.assignment_id = v_item.id)
           WHERE id = v_item.id;
          IF v_si_id IS NOT NULL THEN
            o_items_added := o_items_added + 1; o_total_added := o_total_added + v_item.cash_amount;
          END IF;
        END LOOP;
        UPDATE courier_payouts p
           SET deliveries_count = (SELECT count(*)::int FROM settlement_items si WHERE si.payout_id = p.id),
               total_earned     = (SELECT COALESCE(sum(si.amount), 0)::int FROM settlement_items si WHERE si.payout_id = p.id)
         WHERE p.id = v_payout_id AND p.status = 'pending';
        IF o_items_added > 0 THEN
          INSERT INTO settlement_audit_log (payout_id, location_id, action, actor_kind, metadata)
          VALUES (v_payout_id, v_pair.location_id, 'generated', 'system',
                  jsonb_build_object('backfilled', true, 'added_items', o_items_added, 'added_total', o_total_added));
        END IF;
        RETURN NEXT;
      END LOOP;
    END $fn$;
    REVOKE ALL ON FUNCTION app_backfill_historical_settlements() FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_backfill_historical_settlements() TO dowiz_app;
  `);
}

export async function down(): Promise<void> {
  // Forward-only per migration discipline.
}
