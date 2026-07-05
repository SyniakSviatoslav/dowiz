// B3 remediation PHASE 1 — additive RLS policies (operator places into packages/db/migrations/).
// Provably INERT under today's bypass (dowiz_app BYPASSRLS) — permissive policies OR-combine, only ever
// admit rows; no policy is consulted while the role bypasses. Enforcement switches on at the Phase-3 flip.
// Plan: docs/design/pg-privilege-hardening/remediation-plan.md. Verified against live staging policies +
// grants (2026-06-30). Forward-only, idempotent (DROP IF EXISTS → CREATE). No money/integer changes.
//
// Tighter-than-baseline choices (operator/council-flagged, see plan R-b/R-c):
//   RC2 scoped TO dowiz_app (not bare PUBLIC USING(true)); RC4 command-split (SELECT/UPDATE on orders,
//   INSERT/SELECT on ledger+trace) so couriers can't INSERT/DELETE orders via this policy.
// Live-schema corrections applied: ops_worker_heartbeat already covered (skipped); courier_assignments
//   already missing-ok+dual (skipped); telegram_connect_tokens keys on owner_id (re-keyed on owner_id).
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    -- ── RC1: anon-checkout INSERT siblings (mirror orders/customers anonymous_insert) ──
    DROP POLICY IF EXISTS anonymous_insert ON velocity_events;
    CREATE POLICY anonymous_insert ON velocity_events FOR INSERT WITH CHECK (app_current_user() IS NULL);
    DROP POLICY IF EXISTS anonymous_insert ON order_item_modifiers;
    CREATE POLICY anonymous_insert ON order_item_modifiers FOR INSERT WITH CHECK (app_current_user() IS NULL);
    DROP POLICY IF EXISTS anonymous_insert ON customer_track_grants;
    CREATE POLICY anonymous_insert ON customer_track_grants FOR INSERT WITH CHECK (app_current_user() IS NULL);

    -- ── RC2: zero-policy auth tables (pre-auth, not tenant-scoped) — role-restricted to dowiz_app ──
    -- Protection is role-restriction (only the op role can reach these; non-tenant API surface was locked
    -- down in 1780421100065), not a row predicate. Mirrors the live ops_worker_heartbeat policy, scoped.
    DROP POLICY IF EXISTS ops_all ON users;
    CREATE POLICY ops_all ON users FOR ALL TO dowiz_app USING (true) WITH CHECK (true);
    DROP POLICY IF EXISTS ops_all ON auth_refresh_tokens;
    CREATE POLICY ops_all ON auth_refresh_tokens FOR ALL TO dowiz_app USING (true) WITH CHECK (true);

    -- ── RC3: owner-location resolver (DEFINER, pinned search_path) for getOwnerLocationId pre-withTenant ──
    CREATE OR REPLACE FUNCTION app_owner_location(p_user uuid, p_location uuid DEFAULT NULL)
      RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER
      SET search_path = pg_catalog, public, pg_temp AS $fn$
        SELECT location_id FROM memberships
         WHERE user_id = p_user AND role = 'owner' AND status = 'active'
           AND (p_location IS NULL OR location_id = p_location)
         LIMIT 1
      $fn$;
    REVOKE ALL ON FUNCTION app_owner_location(uuid, uuid) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_owner_location(uuid, uuid) TO dowiz_app;

    -- ── RC4: courier-context writes to orders + cash-as-proof siblings (command-split, additive) ──
    -- Couriers are not members (no memberships row) → app.current_tenant from their verified active shift.
    -- OR-combined with the existing member/anon policies. Couriers never INSERT/DELETE orders → no such policy.
    DROP POLICY IF EXISTS courier_tenant_select ON orders;
    CREATE POLICY courier_tenant_select ON orders FOR SELECT
      USING (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid);
    DROP POLICY IF EXISTS courier_tenant_update ON orders;
    CREATE POLICY courier_tenant_update ON orders FOR UPDATE
      USING (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid)
      WITH CHECK (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid);

    DROP POLICY IF EXISTS courier_tenant_select ON delivery_trace;
    CREATE POLICY courier_tenant_select ON delivery_trace FOR SELECT
      USING (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid);
    DROP POLICY IF EXISTS courier_tenant_insert ON delivery_trace;
    CREATE POLICY courier_tenant_insert ON delivery_trace FOR INSERT
      WITH CHECK (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid);

    DROP POLICY IF EXISTS courier_tenant_select ON courier_cash_ledger;
    CREATE POLICY courier_tenant_select ON courier_cash_ledger FOR SELECT
      USING (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid);
    DROP POLICY IF EXISTS courier_tenant_insert ON courier_cash_ledger;
    CREATE POLICY courier_tenant_insert ON courier_cash_ledger FOR INSERT
      WITH CHECK (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid);

    -- ── RC5: missing-ok rewrite of courier-table isolation (throw-on-unset → clean deny). 8 tables;
    -- courier_assignments already missing-ok+dual (skipped). All were FOR ALL, PUBLIC, USING only. ──
    DROP POLICY IF EXISTS isolate_courier_audit_log ON courier_audit_log;
    CREATE POLICY isolate_courier_audit_log ON courier_audit_log
      USING (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid);
    DROP POLICY IF EXISTS isolate_courier_dispatch_queue ON courier_dispatch_queue;
    CREATE POLICY isolate_courier_dispatch_queue ON courier_dispatch_queue
      USING (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid);
    DROP POLICY IF EXISTS isolate_courier_invites ON courier_invites;
    CREATE POLICY isolate_courier_invites ON courier_invites
      USING (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid);
    DROP POLICY IF EXISTS isolate_courier_locations ON courier_locations;
    CREATE POLICY isolate_courier_locations ON courier_locations
      USING (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid);
    DROP POLICY IF EXISTS isolate_courier_payouts ON courier_payouts;
    CREATE POLICY isolate_courier_payouts ON courier_payouts
      USING (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid);
    DROP POLICY IF EXISTS isolate_courier_positions ON courier_positions;
    CREATE POLICY isolate_courier_positions ON courier_positions
      USING (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid);
    DROP POLICY IF EXISTS isolate_courier_shifts ON courier_shifts;
    CREATE POLICY isolate_courier_shifts ON courier_shifts
      USING (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid);
    DROP POLICY IF EXISTS isolate_customer_track_grants ON customer_track_grants;
    CREATE POLICY isolate_customer_track_grants ON customer_track_grants
      USING (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid);

    -- ── RC6: re-key TO-authenticated (Supabase Data-API) policies to the operational app.* GUC model ──
    -- owner_notification_targets: owner CRUD via withTenant (app.user_id → app_member_location_ids()).
    DROP POLICY IF EXISTS owner_notification_targets_owner_all ON owner_notification_targets;
    CREATE POLICY tenant_isolation ON owner_notification_targets FOR ALL
      USING (location_id IN (SELECT app_member_location_ids()))
      WITH CHECK (location_id IN (SELECT app_member_location_ids()));
    -- telegram_connect_tokens keys on owner_id (NOT location_id) — re-key faithfully to app_current_user().
    DROP POLICY IF EXISTS telegram_connect_tokens_owner_all ON telegram_connect_tokens;
    CREATE POLICY owner_isolation ON telegram_connect_tokens FOR ALL
      USING (owner_id = app_current_user())
      WITH CHECK (owner_id = app_current_user());
    -- customer_devices: drop TO authenticated; customer/push.ts sets app.user_id.
    DROP POLICY IF EXISTS customer_devices_owner_all ON customer_devices;
    CREATE POLICY customer_owns ON customer_devices FOR ALL
      USING (customer_id = app_current_user())
      WITH CHECK (customer_id = app_current_user());
  `);
}

export async function down(): Promise<void> {
  // Forward-only per migration discipline. (Phase-1 policies are additive/inert; no down path.)
}
