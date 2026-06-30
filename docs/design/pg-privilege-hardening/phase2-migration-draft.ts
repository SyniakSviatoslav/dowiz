// B3 remediation PHASE 2 — cross-tenant system-sweep SECURITY DEFINER maintenance fns (operator places into
// packages/db/migrations/). 19 fns consolidated from the 3 worker-remediation lanes. Each: SECURITY DEFINER,
// pinned search_path (ITEM1 guardrail), REVOKE ALL FROM PUBLIC + GRANT EXECUTE TO dowiz_app. Each mirrors the
// worker's prior SQL EXACTLY — behavior-identical under today's bypass; correct post-flip. Member-keyed tables
// (location_alerts, order_status_history, owner_notification_targets, customer_signals, settlement_*) are
// reachable by these system actors (no member identity) ONLY via these DEFINER fns — NOT by widening tenant
// policies onto owner tables. Forward-only. No money/integer logic changed.
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    -- ===== Lane 1: order/dwell/lifecycle =====
    CREATE OR REPLACE FUNCTION app_sweep_timeout_orders() RETURNS TABLE(id uuid, location_id uuid)
      LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        WITH cancelled AS (
          UPDATE orders SET status='CANCELLED', timeout_at=NULL
            WHERE status='PENDING' AND timeout_at IS NOT NULL AND timeout_at < now()
            RETURNING id, location_id),
        hist AS (
          INSERT INTO order_status_history (order_id, location_id, from_status, to_status, actor)
            SELECT id, location_id, 'PENDING','CANCELLED','system:timeout' FROM cancelled)
        SELECT id, location_id FROM cancelled $fn$;
    REVOKE ALL ON FUNCTION app_sweep_timeout_orders() FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_sweep_timeout_orders() TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_dwell_due_orders(p_location_id uuid, p_status text, p_threshold_seconds text, p_kind text)
      RETURNS TABLE(id uuid, status text, status_updated_at timestamptz)
      LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        SELECT o.id, o.status::text, COALESCE(o.confirmed_at, o.created_at) AS status_updated_at
        FROM orders o
        WHERE o.location_id = p_location_id
          AND o.status = p_status::order_status
          AND COALESCE(o.confirmed_at, o.created_at) < now() - (p_threshold_seconds || ' seconds')::interval
          AND NOT EXISTS (SELECT 1 FROM location_alerts la WHERE la.order_id = o.id AND la.kind = p_kind AND la.resolved_at IS NULL)
        LIMIT 50 $fn$;
    REVOKE ALL ON FUNCTION app_dwell_due_orders(uuid, text, text, text) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_dwell_due_orders(uuid, text, text, text) TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_dwell_create_alert(p_location_id uuid, p_order_id uuid, p_kind text)
      RETURNS TABLE(id uuid)
      LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        INSERT INTO location_alerts (location_id, order_id, kind, status, escalation_level)
        VALUES (p_location_id, p_order_id, p_kind, 'active', 0)
        ON CONFLICT DO NOTHING RETURNING id $fn$;
    REVOKE ALL ON FUNCTION app_dwell_create_alert(uuid, uuid, text) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_dwell_create_alert(uuid, uuid, text) TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_active_notification_targets(p_location_id uuid, p_channel text)
      RETURNS TABLE(id uuid, address text)
      LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        SELECT id, address FROM owner_notification_targets
        WHERE location_id = p_location_id AND channel = p_channel AND status = 'active' $fn$;
    REVOKE ALL ON FUNCTION app_active_notification_targets(uuid, text) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_active_notification_targets(uuid, text) TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_alert_state(p_alert_id uuid)
      RETURNS TABLE(status text, acknowledged_at timestamptz, escalation_level int)
      LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        SELECT status, acknowledged_at, escalation_level FROM location_alerts WHERE id = p_alert_id $fn$;
    REVOKE ALL ON FUNCTION app_alert_state(uuid) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_alert_state(uuid) TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_bump_alert_escalation(p_alert_id uuid, p_tier int)
      RETURNS void LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        UPDATE location_alerts SET escalation_level = GREATEST(escalation_level, p_tier) WHERE id = p_alert_id $fn$;
    REVOKE ALL ON FUNCTION app_bump_alert_escalation(uuid, int) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_bump_alert_escalation(uuid, int) TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_count_active_dwell_alerts(p_location_id uuid)
      RETURNS int LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        SELECT COUNT(*)::int FROM location_alerts
        WHERE location_id = p_location_id AND kind LIKE 'dwell_%' AND status = 'active' AND resolved_at IS NULL $fn$;
    REVOKE ALL ON FUNCTION app_count_active_dwell_alerts(uuid) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_count_active_dwell_alerts(uuid) TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_alert_tier_reached(p_alert_id uuid, p_tier int)
      RETURNS boolean LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        SELECT EXISTS (SELECT 1 FROM location_alerts WHERE id = p_alert_id AND escalation_level >= p_tier) $fn$;
    REVOKE ALL ON FUNCTION app_alert_tier_reached(uuid, int) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_alert_tier_reached(uuid, int) TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_alert_log_attempt(p_alert_id uuid, p_entry jsonb, p_error text)
      RETURNS void LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        UPDATE location_alerts SET attempts = COALESCE(attempts, '[]'::jsonb) || p_entry, last_error = p_error
        WHERE id = p_alert_id $fn$;
    REVOKE ALL ON FUNCTION app_alert_log_attempt(uuid, jsonb, text) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_alert_log_attempt(uuid, jsonb, text) TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_resolve_order_alerts(p_reason text, p_order_id uuid, p_kind text)
      RETURNS TABLE(id uuid) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        UPDATE location_alerts SET status='resolved', resolved_at=now(), resolution_reason=p_reason
        WHERE order_id = p_order_id AND kind = p_kind AND resolved_at IS NULL RETURNING id $fn$;
    REVOKE ALL ON FUNCTION app_resolve_order_alerts(text, uuid, text) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_resolve_order_alerts(text, uuid, text) TO dowiz_app;

    -- ===== Lane 2: courier =====
    CREATE OR REPLACE FUNCTION app_sweep_gps_purge(p_retention interval)
      RETURNS void LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        DELETE FROM courier_positions WHERE recorded_at < now() - p_retention $fn$;
    REVOKE ALL ON FUNCTION app_sweep_gps_purge(interval) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_sweep_gps_purge(interval) TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_sweep_stale_couriers(p_stale interval)
      RETURNS TABLE (shift_id uuid, courier_id uuid, order_id uuid, location_id uuid)
      LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
      BEGIN
        RETURN QUERY
        WITH stale AS (
          SELECT cs.id AS shift_id, cs.courier_id, ca.order_id, ca.location_id
          FROM courier_shifts cs
          JOIN courier_assignments ca ON cs.id = ca.shift_id AND ca.status IN ('assigned','accepted','picked_up')
          WHERE cs.status = 'on_delivery' AND cs.last_heartbeat_at < now() - p_stale
          FOR UPDATE SKIP LOCKED
        ), ins AS (
          INSERT INTO location_alerts (location_id, order_id, kind, status, escalation_level)
          SELECT s.location_id, s.order_id, 'courier_offline', 'active', 0 FROM stale s
          ON CONFLICT DO NOTHING)
        SELECT s.shift_id, s.courier_id, s.order_id, s.location_id FROM stale s;
      END $fn$;
    REVOKE ALL ON FUNCTION app_sweep_stale_couriers(interval) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_sweep_stale_couriers(interval) TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_sweep_expired_offers()
      RETURNS TABLE (order_id uuid, location_id uuid)
      LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        UPDATE courier_assignments
           SET status='offered_expired', cancelled_at=now(), cancellation_reason='offer_timeout'
         WHERE status='offered' AND offered_expires_at < now()
        RETURNING order_id, location_id $fn$;
    REVOKE ALL ON FUNCTION app_sweep_expired_offers() FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_sweep_expired_offers() TO dowiz_app;

    -- ===== Lane 3: money/retention/signal =====
    CREATE OR REPLACE FUNCTION app_recon_delivered_cash_mismatch()
      RETURNS TABLE(id uuid, order_id uuid, cash_amount integer, total integer)
      LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        SELECT a.id, a.order_id, a.cash_amount, o.total
        FROM courier_assignments a JOIN orders o ON o.id = a.order_id
        WHERE a.cash_collected = true AND a.cash_amount IS NOT NULL AND a.cash_amount != o.total
        LIMIT 20 $fn$;
    REVOKE ALL ON FUNCTION app_recon_delivered_cash_mismatch() FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_recon_delivered_cash_mismatch() TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_recon_open_shifts()
      RETURNS TABLE(id uuid, courier_id uuid, location_id uuid, started_at timestamptz)
      LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        SELECT id, courier_id, location_id, started_at FROM courier_shifts
        WHERE status IN ('available','on_delivery') AND started_at < now() - interval '24 hours'
        ORDER BY started_at LIMIT 20 $fn$;
    REVOKE ALL ON FUNCTION app_recon_open_shifts() FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_recon_open_shifts() TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_recon_assignments_missing_courier()
      RETURNS TABLE(id uuid) LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        SELECT a.id FROM courier_assignments a LEFT JOIN couriers c ON c.id = a.courier_id
        WHERE c.id IS NULL LIMIT 10 $fn$;
    REVOKE ALL ON FUNCTION app_recon_assignments_missing_courier() FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_recon_assignments_missing_courier() TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_generate_settlements(p_period_start timestamptz, p_period_end timestamptz)
      RETURNS void LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
    DECLARE v_pair record; v_payout record; v_item record; v_added_items integer; v_added_total integer;
    BEGIN
      FOR v_pair IN
        SELECT DISTINCT courier_id, location_id FROM courier_assignments
        WHERE status='delivered' AND cash_collected=true AND delivered_at >= p_period_start AND delivered_at < p_period_end
      LOOP
        INSERT INTO courier_payouts (courier_id, location_id, period_start, period_end, status)
        VALUES (v_pair.courier_id, v_pair.location_id, p_period_start, p_period_end, 'pending')
        ON CONFLICT (courier_id, location_id, period_start, period_end) DO UPDATE SET status = courier_payouts.status
        RETURNING id, status INTO v_payout;
        v_added_items := 0; v_added_total := 0;
        FOR v_item IN
          SELECT ca.id, ca.cash_amount, loc.currency_code FROM courier_assignments ca
          JOIN locations loc ON loc.id = ca.location_id
          WHERE ca.courier_id=v_pair.courier_id AND ca.location_id=v_pair.location_id AND ca.status='delivered'
            AND ca.cash_collected=true AND ca.delivered_at >= p_period_start AND ca.delivered_at < p_period_end
            AND NOT EXISTS (SELECT 1 FROM settlement_items si WHERE si.assignment_id = ca.id)
          FOR UPDATE OF ca SKIP LOCKED
        LOOP
          INSERT INTO settlement_items (payout_id, assignment_id, location_id, amount, currency_code)
          VALUES (v_payout.id, v_item.id, v_pair.location_id, v_item.cash_amount, v_item.currency_code)
          ON CONFLICT (assignment_id) DO NOTHING;
          UPDATE courier_assignments SET settlement_item_id = (SELECT id FROM settlement_items WHERE assignment_id = v_item.id)
          WHERE id = v_item.id;
          v_added_items := v_added_items + 1; v_added_total := v_added_total + v_item.cash_amount;
        END LOOP;
        IF v_added_items > 0 THEN
          UPDATE courier_payouts SET deliveries_count = deliveries_count + v_added_items, total_earned = total_earned + v_added_total
          WHERE id = v_payout.id;
          INSERT INTO settlement_audit_log (payout_id, location_id, action, actor_kind, metadata)
          VALUES (v_payout.id, v_pair.location_id, 'generated', 'system', jsonb_build_object('added_items', v_added_items, 'added_total', v_added_total));
        END IF;
      END LOOP;
    END $fn$;
    REVOKE ALL ON FUNCTION app_generate_settlements(timestamptz, timestamptz) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_generate_settlements(timestamptz, timestamptz) TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_sweep_velocity_active_locations()
      RETURNS TABLE(location_id uuid, phone_hash text, client_ip_hash text, customer_id uuid)
      LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        SELECT DISTINCT v.location_id, v.phone_hash, v.client_ip_hash, ve.customer_id
        FROM velocity_events v
        LEFT JOIN LATERAL (SELECT customer_id FROM orders WHERE location_id = v.location_id AND created_at > now() - interval '24 hours' LIMIT 1) ve ON true
        WHERE v.window_started_at > now() - interval '24 hours' $fn$;
    REVOKE ALL ON FUNCTION app_sweep_velocity_active_locations() FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_sweep_velocity_active_locations() TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_raise_customer_signal(p_customer_id uuid, p_location_id uuid, p_kind text, p_severity text, p_evidence jsonb)
      RETURNS uuid LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
    DECLARE v_id uuid;
    BEGIN
      IF EXISTS (SELECT 1 FROM customer_signals WHERE customer_id = p_customer_id AND kind = p_kind AND raised_at > now() - interval '1 hour') THEN
        RETURN NULL;
      END IF;
      INSERT INTO customer_signals (customer_id, location_id, kind, severity, evidence)
      VALUES (p_customer_id, p_location_id, p_kind, p_severity, p_evidence) RETURNING id INTO v_id;
      RETURN v_id;
    END $fn$;
    REVOKE ALL ON FUNCTION app_raise_customer_signal(uuid, uuid, text, text, jsonb) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_raise_customer_signal(uuid, uuid, text, text, jsonb) TO dowiz_app;
  `);
}

export async function down(): Promise<void> {
  // Forward-only per migration discipline.
}
