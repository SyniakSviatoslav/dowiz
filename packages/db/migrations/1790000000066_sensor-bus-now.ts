import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * MVP SENSOR-BUS (NOW-runtime) — council-approved batch `mvp-sensor-seams` (ADR-0007/0009 v4).
 * Forward-only, additive, inert-by-default. NO autopilot; NO stock-decrement runtime (Option B —
 * stock_remaining is an inert seam). Money is integer minor units. RLS ENABLE+FORCE in this migration.
 */
export async function up(pgm: MigrationBuilder): Promise<void> {
  // 1. STOCK SEAM (Option B) — inert column. NULL = unlimited (current binary behaviour).
  pgm.sql(`ALTER TABLE products ADD COLUMN IF NOT EXISTS stock_remaining integer;`);

  // 2. PROMISE vs LIVE ETA (ESTOP-1 split). Frozen promised_window_* = measurement; mutable live_eta_*
  //    = customer truth channel. range-never-point structural: no single-number column, only lo/hi.
  pgm.sql(`
    ALTER TABLE orders
      ADD COLUMN IF NOT EXISTS promised_window_lo_min integer,
      ADD COLUMN IF NOT EXISTS promised_window_hi_min integer,
      ADD COLUMN IF NOT EXISTS live_eta_lo_min        integer,
      ADD COLUMN IF NOT EXISTS live_eta_hi_min        integer,
      ADD COLUMN IF NOT EXISTS stock_committed        boolean NOT NULL DEFAULT false;
  `);

  // 3. set-once guard on promised_window_* (immutable once set). live_eta_* NOT guarded.
  pgm.sql(`
    CREATE OR REPLACE FUNCTION orders_promised_window_set_once()
    RETURNS trigger LANGUAGE plpgsql AS $FN$
    BEGIN
      IF OLD.promised_window_lo_min IS NOT NULL
         AND (NEW.promised_window_lo_min IS DISTINCT FROM OLD.promised_window_lo_min
           OR NEW.promised_window_hi_min IS DISTINCT FROM OLD.promised_window_hi_min) THEN
        RAISE EXCEPTION 'promised_window is immutable once set (order %)', OLD.id;
      END IF;
      RETURN NEW;
    END;
    $FN$;
    DROP TRIGGER IF EXISTS trg_orders_promised_window_set_once ON orders;
    CREATE TRIGGER trg_orders_promised_window_set_once
      BEFORE UPDATE OF promised_window_lo_min, promised_window_hi_min ON orders
      FOR EACH ROW EXECUTE FUNCTION orders_promised_window_set_once();
  `);

  // 4. NORMALISED DELIVERY BASELINE (§1.2) — observed road-distance baseline, no router.
  pgm.sql(`
    ALTER TABLE delivery_trace
      ADD COLUMN IF NOT EXISTS route_distance_m      integer,
      ADD COLUMN IF NOT EXISTS expected_delivery_min integer;
  `);

  // 5. CONFIG KNOBS — eta_cap absolute ceiling; min_window_width = range-never-point floor.
  pgm.sql(`
    ALTER TABLE locations
      ADD COLUMN IF NOT EXISTS eta_cap_min          integer NOT NULL DEFAULT 90,
      ADD COLUMN IF NOT EXISTS dispatch_margin_min  integer NOT NULL DEFAULT 5,
      ADD COLUMN IF NOT EXISTS material_shift_min   integer NOT NULL DEFAULT 5,
      ADD COLUMN IF NOT EXISTS otp_target_pct       integer NOT NULL DEFAULT 90,
      ADD COLUMN IF NOT EXISTS min_window_width_min integer NOT NULL DEFAULT 10,
      ADD COLUMN IF NOT EXISTS geofence_radius_m    integer NOT NULL DEFAULT 150;
  `);

  // 6. SENSOR EVENTS (§1.1) — append-only, exactly-once per (order,event_type). Dual-context RLS.
  pgm.sql(`
    CREATE TABLE IF NOT EXISTS order_sensor_events (
      id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id  uuid NOT NULL REFERENCES locations(id),
      order_id     uuid NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
      event_type   text NOT NULL,
      payload      jsonb,
      created_at   timestamptz NOT NULL DEFAULT now(),
      CONSTRAINT order_sensor_events_once UNIQUE (order_id, event_type)
    );
    CREATE INDEX IF NOT EXISTS order_sensor_events_order_idx ON order_sensor_events (order_id);
    CREATE INDEX IF NOT EXISTS order_sensor_events_loc_idx   ON order_sensor_events (location_id, created_at DESC);
  `);
  pgm.sql(`
    ALTER TABLE order_sensor_events ENABLE ROW LEVEL SECURITY;
    ALTER TABLE order_sensor_events FORCE  ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS tenant_dual ON order_sensor_events;
    CREATE POLICY tenant_dual ON order_sensor_events
      USING (
        location_id IN (SELECT app_member_location_ids())
        OR location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid )
      WITH CHECK (
        location_id IN (SELECT app_member_location_ids())
        OR location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid );
  `);
  pgm.sql(`
    REVOKE ALL ON order_sensor_events FROM anon, authenticated, service_role;
    DO $GR$ BEGIN
      IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'deliveryos_api_user') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON order_sensor_events TO deliveryos_api_user;
      END IF;
    END $GR$;
  `);

  // 7. FUNNEL (§1.3) — anonymous, session-scoped; zero PII beyond session_ref.
  pgm.sql(`
    CREATE TABLE IF NOT EXISTS funnel_events (
      id               uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id      uuid NOT NULL REFERENCES locations(id),
      session_ref      text NOT NULL,
      event_type       text NOT NULL,
      shown_eta_lo_min integer,
      shown_eta_hi_min integer,
      created_at       timestamptz NOT NULL DEFAULT now()
    );
    CREATE INDEX IF NOT EXISTS funnel_events_loc_idx ON funnel_events (location_id, created_at DESC);
  `);
  pgm.sql(`
    ALTER TABLE funnel_events ENABLE ROW LEVEL SECURITY;
    ALTER TABLE funnel_events FORCE  ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS tenant_isolation ON funnel_events;
    CREATE POLICY tenant_isolation ON funnel_events
      USING      ( location_id IN (SELECT app_member_location_ids()) )
      WITH CHECK ( location_id IN (SELECT app_member_location_ids())
                   OR location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid );
  `);
  pgm.sql(`
    REVOKE ALL ON funnel_events FROM anon, authenticated, service_role;
    DO $GR$ BEGIN
      IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'deliveryos_api_user') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON funnel_events TO deliveryos_api_user;
      END IF;
    END $GR$;
  `);

  // 8. GEOFENCE DETERMINISM (R3-H2) — one active assignment per courier for MVP.
  pgm.sql(`
    CREATE UNIQUE INDEX IF NOT EXISTS courier_one_active_assignment
      ON courier_assignments (courier_id)
      WHERE status IN ('assigned', 'accepted', 'picked_up');
  `);
}

export async function down(): Promise<void> { /* Forward-only; inert. */ }
