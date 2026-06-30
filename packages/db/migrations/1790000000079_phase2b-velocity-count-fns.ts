// B3 remediation PHASE 2 (cont.) — velocity-count DEFINER fns for lib/signals/compute.ts.
// computeSignals() reads velocity_events (member-keyed SELECT) from GUC-less callers (signal-raiser system
// worker + anon checkout) → 0 rows post-flip. Single-location counts → DEFINER count fns (pinned search_path,
// GRANT dowiz_app). Mirror compute.ts:116-145 exactly. retention.ts needs no fn (acquisition_sources/
// provision_grants/claim_invites all have allow_ops_*_all USING(true)).
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE OR REPLACE FUNCTION app_velocity_phone_count(p_location uuid, p_phone_hash text, p_seconds text)
      RETURNS int LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        SELECT COUNT(*)::int FROM velocity_events
        WHERE location_id = p_location AND phone_hash = p_phone_hash AND kind = 'order_placed'
          AND window_started_at > now() - (p_seconds || ' seconds')::interval $fn$;
    REVOKE ALL ON FUNCTION app_velocity_phone_count(uuid, text, text) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_velocity_phone_count(uuid, text, text) TO dowiz_app;

    CREATE OR REPLACE FUNCTION app_velocity_ip_count(p_location uuid, p_ip_hash text, p_seconds text)
      RETURNS int LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        SELECT COUNT(*)::int FROM velocity_events
        WHERE location_id = p_location AND client_ip_hash = p_ip_hash AND kind = 'order_placed'
          AND window_started_at > now() - (p_seconds || ' seconds')::interval $fn$;
    REVOKE ALL ON FUNCTION app_velocity_ip_count(uuid, text, text) FROM PUBLIC;
    GRANT EXECUTE ON FUNCTION app_velocity_ip_count(uuid, text, text) TO dowiz_app;
  `);
}

export async function down(): Promise<void> {
  // Forward-only.
}
