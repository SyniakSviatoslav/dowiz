// STAGED → operator/lead places at packages/db/migrations/1790000000073_deliver-v2-cash-as-proof.ts
// (migrations dir is a protected governance zone). deliver v2 (Cash-as-Proof) — ADR-deliver-v2-cash-as-proof.
// Forward-only, additive + idempotent. Council-hardened (proposal §5). REQUIRES the live head 072.
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // 1. Offer sub-states on the binding entity (A2 — keeps the customer order_status enum untouched).
  pgm.sql(`
    ALTER TABLE courier_assignments DROP CONSTRAINT IF EXISTS courier_assignments_status_check;
    ALTER TABLE courier_assignments ADD  CONSTRAINT courier_assignments_status_check
      CHECK (status IN ('offered','assigned','accepted','picked_up','delivered',
                        'cancelled','rejected','offered_expired'));
    ALTER TABLE courier_assignments
      ADD COLUMN IF NOT EXISTS offered_at         timestamptz,
      ADD COLUMN IF NOT EXISTS offered_expires_at timestamptz;

    -- C-1 FIX (the no-trap red-line): the live courier_assignments_order_uniq (1780421100041:23) is a FULL
    -- unique on order_id → one row per order FOREVER → a rejected/cancelled row permanently blocks re-offer
    -- (today: 0% redispatch for any order that ever had a row). Replace with a PARTIAL unique on ACTIVE
    -- states only: terminal rows never block a re-offer; at most ONE active binding per order at the DB level.
    DROP INDEX IF EXISTS courier_assignments_order_uniq;
    CREATE UNIQUE INDEX courier_assignments_order_active_uniq
      ON courier_assignments (order_id)
      WHERE status IN ('offered','assigned','accepted','picked_up');

    -- Sweep predicate.
    CREATE INDEX IF NOT EXISTS courier_assignments_offered_due_idx
      ON courier_assignments (offered_expires_at) WHERE status = 'offered';

    -- Extend the single-active-assignment guard (1790000000066:128-130) to include 'offered'.
    DROP INDEX IF EXISTS courier_one_active_assignment;
    CREATE UNIQUE INDEX courier_one_active_assignment ON courier_assignments (courier_id)
      WHERE status IN ('offered','assigned','accepted','picked_up');
  `);

  // R-1 FIX (M-1): FORCE RLS to canon. GROUNDING CORRECTION vs proposal §5: courier_assignments is accessed
  // by BOTH couriers (set_config('app.current_tenant', locationId) — assignments.ts:79) AND owners
  // (withTenant → app.user_id member context — dashboard.ts). Couriers are NOT members (courier_locations,
  // not memberships) → aligning the policy to app_member_location_ids() ALONE would break ALL courier access
  // under FORCE. The correct policy admits BOTH contexts. FORCE closes the owner/BYPASS bypass; the
  // cross-courier-same-location vector stays closed by the inline `AND courier_id=$me` predicate in app code.
  pgm.sql(`
    ALTER TABLE courier_assignments FORCE ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS isolate_courier_assignments ON courier_assignments;
    CREATE POLICY isolate_courier_assignments ON courier_assignments
      USING (
        location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid   -- courier context
        OR location_id = ANY (SELECT app_member_location_ids())                        -- owner/member context
      );
  `);

  // 2. Passive crumbs onto the immutable delivery_trace (append-only; already ENABLE+FORCE).
  pgm.sql(`
    ALTER TABLE delivery_trace
      ADD COLUMN IF NOT EXISTS payment_outcome   payment_outcome,
      ADD COLUMN IF NOT EXISTS cash_amount       integer CHECK (cash_amount IS NULL OR cash_amount >= 0),
      ADD COLUMN IF NOT EXISTS gps_lat           double precision,
      ADD COLUMN IF NOT EXISTS gps_lng           double precision,
      ADD COLUMN IF NOT EXISTS name_snapshot     jsonb,
      ADD COLUMN IF NOT EXISTS price_snapshot    integer CHECK (price_snapshot IS NULL OR price_snapshot >= 0);
  `);

  // R3-1 FIX (anonymize-not-delete MUST actually fire): delivery_trace is tenant-scoped FORCE
  // (1790000000027) → a context-free operational-pool UPDATE sees 0 rows → GPS retained forever. The sweep
  // runs through a SECURITY DEFINER fn. PRECISE MECHANISM (R4): SECURITY DEFINER alone does NOT bypass FORCE;
  // the sweep reaches all-tenant rows because the function OWNER (the migration privileged/superuser role)
  // carries BYPASSRLS — the standard Supabase/Fly deploy. Pinned search_path. R4-2: floor p_window to the
  // 7-day dispute window so a mis-set env can never anonymize evidence INSIDE the dispute window.
  pgm.sql(`
    CREATE OR REPLACE FUNCTION anonymize_stale_delivery_trace(p_window interval)
    RETURNS integer LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = public AS $$
    DECLARE
      v_count  integer;
      v_window interval := GREATEST(p_window, interval '7 days');
    BEGIN
      WITH anon AS (
        UPDATE delivery_trace
           SET gps_lat = NULL, gps_lng = NULL, name_snapshot = NULL, price_snapshot = NULL
         WHERE delivered_at < now() - v_window
           AND (gps_lat IS NOT NULL OR gps_lng IS NOT NULL
                OR name_snapshot IS NOT NULL OR price_snapshot IS NOT NULL)
        RETURNING 1)
      SELECT count(*)::int INTO v_count FROM anon;
      RETURN v_count;
    END;
    $$;
    REVOKE ALL ON FUNCTION anonymize_stale_delivery_trace(interval) FROM PUBLIC;
  `);
  // Grant EXECUTE to whatever role can already EXECUTE read_public_menu_all_locales (operational role mirror).
  pgm.sql(`
    DO $$
    DECLARE r record;
    BEGIN
      FOR r IN
        SELECT DISTINCT grantee FROM information_schema.role_routine_grants
        WHERE routine_schema = 'public' AND routine_name = 'read_public_menu_all_locales' AND grantee <> 'PUBLIC'
      LOOP
        EXECUTE format('GRANT EXECUTE ON FUNCTION anonymize_stale_delivery_trace(interval) TO %I', r.grantee);
      END LOOP;
    END
    $$;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP FUNCTION IF EXISTS anonymize_stale_delivery_trace(interval);
    ALTER TABLE delivery_trace
      DROP COLUMN IF EXISTS payment_outcome, DROP COLUMN IF EXISTS cash_amount,
      DROP COLUMN IF EXISTS gps_lat, DROP COLUMN IF EXISTS gps_lng,
      DROP COLUMN IF EXISTS name_snapshot, DROP COLUMN IF EXISTS price_snapshot;
    DROP INDEX IF EXISTS courier_assignments_order_active_uniq;
    DROP INDEX IF EXISTS courier_assignments_offered_due_idx;
    ALTER TABLE courier_assignments
      DROP COLUMN IF EXISTS offered_at, DROP COLUMN IF EXISTS offered_expires_at;
    -- (CHECK + one_active + FORCE/policy left forward; revert manually pre-launch if needed.)
  `);
}
