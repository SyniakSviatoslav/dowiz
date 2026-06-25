# Proposed DDL — mvp-sensor-seams (council-APPROVED, Option B)

> The migration FILES live in `packages/db/migrations/` — a protect-paths governance zone (human-authored/
> approved only). This artifact holds the exact, ready-to-paste DDL. **To apply:** create the two files
> below with `pnpm migrate:create <name>` (writes the stub into `packages/db/migrations/`), paste the body,
> then `pnpm migrate:up` against staging first. Both are forward-only, additive, inert-by-default; `down()`
> is a no-op. Money is integer minor units. RLS ENABLE+FORCE is in the same migration as each new tenant
> table, `tenant_isolation` mirrors `menu_schedules` (1790000000062).

## Migration 1 — `1790000000066_sensor-bus-now` (§6.1 NOW-runtime)

```ts
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // 1. STOCK SEAM (Option B) — inert column only. NULL = unlimited (current binary behaviour).
  pgm.sql(`ALTER TABLE products ADD COLUMN IF NOT EXISTS stock_remaining integer;`);

  // 2. PROMISE vs LIVE ETA (ADR-0009 §3, ESTOP-1 split). Frozen promised_window_* = promise-as-made
  //    (measurement, owner/analytics only). Mutable live_eta_* = customer truth channel. No point column.
  pgm.sql(`
    ALTER TABLE orders
      ADD COLUMN IF NOT EXISTS promised_window_lo_min integer,
      ADD COLUMN IF NOT EXISTS promised_window_hi_min integer,
      ADD COLUMN IF NOT EXISTS live_eta_lo_min        integer,
      ADD COLUMN IF NOT EXISTS live_eta_hi_min        integer,
      ADD COLUMN IF NOT EXISTS stock_committed        boolean NOT NULL DEFAULT false;
  `);

  // 3. set-once guard on promised_window_* (immutable once set). live_eta_* intentionally NOT guarded.
  pgm.sql(`
    CREATE OR REPLACE FUNCTION orders_promised_window_set_once()
    RETURNS trigger LANGUAGE plpgsql AS $$
    BEGIN
      IF OLD.promised_window_lo_min IS NOT NULL
         AND (NEW.promised_window_lo_min IS DISTINCT FROM OLD.promised_window_lo_min
           OR NEW.promised_window_hi_min IS DISTINCT FROM OLD.promised_window_hi_min) THEN
        RAISE EXCEPTION 'promised_window is immutable once set (order %)', OLD.id;
      END IF;
      RETURN NEW;
    END;
    $$;
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

  // 5. CONFIG KNOBS (§1.4/§2.1/§2.4). eta_cap = absolute window ceiling; min_window_width = the
  //    range-never-point FLOOR (no pseudo-precise "1–2 min").
  pgm.sql(`
    ALTER TABLE locations
      ADD COLUMN IF NOT EXISTS eta_cap_min          integer NOT NULL DEFAULT 90,
      ADD COLUMN IF NOT EXISTS dispatch_margin_min  integer NOT NULL DEFAULT 5,
      ADD COLUMN IF NOT EXISTS material_shift_min   integer NOT NULL DEFAULT 5,
      ADD COLUMN IF NOT EXISTS otp_target_pct       integer NOT NULL DEFAULT 90,
      ADD COLUMN IF NOT EXISTS min_window_width_min integer NOT NULL DEFAULT 10,
      ADD COLUMN IF NOT EXISTS geofence_radius_m    integer NOT NULL DEFAULT 150;
  `);

  // 6. SENSOR EVENTS (§1.1) — append-only, exactly-once per (order,event_type). Dual-context RLS:
  //    owners read via membership; courier ping context writes via app.current_tenant (one GUC set
  //    per context → the other disjunct is a dead NULL → no tenancy widening).
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
    DO $$ BEGIN
      IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'deliveryos_api_user') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON order_sensor_events TO deliveryos_api_user;
      END IF;
    END $$;
  `);

  // 7. FUNNEL (§1.3) — anonymous, session-scoped pre-order signal; zero PII beyond session_ref.
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
    DO $$ BEGIN
      IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'deliveryos_api_user') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON funnel_events TO deliveryos_api_user;
      END IF;
    END $$;
  `);

  // 8. GEOFENCE DETERMINISM (R3-H2) — one active assignment per courier for MVP → geofence binds
  //    unambiguously. (Replaced by per-assignment proximity binding when P3 courier_sequence activates.)
  pgm.sql(`
    CREATE UNIQUE INDEX IF NOT EXISTS courier_one_active_assignment
      ON courier_assignments (courier_id)
      WHERE status IN ('assigned', 'accepted', 'picked_up');
  `);
}

export async function down(): Promise<void> { /* Forward-only; inert. */ }
```

## Migration 2 — `1790000000067_bom-seams` (§6.2 SEAM — inert, runtime FLAT/manual)

```ts
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // INGREDIENTS — self-referential via kind (leaf raw / intermediate batch node). Inert: no reader yet.
  pgm.sql(`
    CREATE TABLE IF NOT EXISTS ingredients (
      id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id   uuid NOT NULL REFERENCES locations(id),
      name          text NOT NULL,
      kind          text NOT NULL DEFAULT 'raw' CHECK (kind IN ('raw','intermediate')),
      is_batch_made boolean NOT NULL DEFAULT false,
      unit          text,
      current_stock numeric,
      tracking_mode text NOT NULL DEFAULT 'untracked' CHECK (tracking_mode IN ('quantity','boolean','untracked')),
      waste_pct     numeric NOT NULL DEFAULT 0,
      reset_cadence text CHECK (reset_cadence IS NULL OR reset_cadence IN ('daily','weekly','monthly')),
      last_set_at   timestamptz,
      created_at    timestamptz NOT NULL DEFAULT now()
    );
    CREATE INDEX IF NOT EXISTS ingredients_loc_idx ON ingredients (location_id);
  `);
  pgm.sql(`
    ALTER TABLE ingredients ENABLE ROW LEVEL SECURITY;
    ALTER TABLE ingredients FORCE  ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS tenant_isolation ON ingredients;
    CREATE POLICY tenant_isolation ON ingredients
      USING      ( location_id IN (SELECT app_member_location_ids()) )
      WITH CHECK ( location_id IN (SELECT app_member_location_ids()) );
    REVOKE ALL ON ingredients FROM anon, authenticated, service_role;
    DO $$ BEGIN IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname='deliveryos_api_user') THEN
      GRANT SELECT,INSERT,UPDATE,DELETE ON ingredients TO deliveryos_api_user; END IF; END $$;
  `);

  // RECIPE_COMPONENTS — BOM (M:N). parent = product OR intermediate ingredient (POLYMORPHIC parent_id,
  //    no native FK — ADR-0008). The manual→derived upgrade swaps the READER, not these rows.
  pgm.sql(`
    CREATE TABLE IF NOT EXISTS recipe_components (
      id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id    uuid NOT NULL REFERENCES locations(id),
      parent_kind    text NOT NULL CHECK (parent_kind IN ('product','ingredient')),
      parent_id      uuid NOT NULL,
      ingredient_id  uuid NOT NULL REFERENCES ingredients(id),
      qty_per_parent numeric NOT NULL,
      unit           text NOT NULL,
      created_at     timestamptz NOT NULL DEFAULT now()
    );
    CREATE INDEX IF NOT EXISTS recipe_components_parent_idx ON recipe_components (location_id, parent_kind, parent_id);
  `);
  pgm.sql(`
    ALTER TABLE recipe_components ENABLE ROW LEVEL SECURITY;
    ALTER TABLE recipe_components FORCE  ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS tenant_isolation ON recipe_components;
    CREATE POLICY tenant_isolation ON recipe_components
      USING      ( location_id IN (SELECT app_member_location_ids()) )
      WITH CHECK ( location_id IN (SELECT app_member_location_ids()) );
    REVOKE ALL ON recipe_components FROM anon, authenticated, service_role;
    DO $$ BEGIN IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname='deliveryos_api_user') THEN
      GRANT SELECT,INSERT,UPDATE,DELETE ON recipe_components TO deliveryos_api_user; END IF; END $$;
  `);

  // Orphan protection for the un-FK'd parent_id (ADR-0008 v3) — DELETE + TRUNCATE companions.
  pgm.sql(`
    CREATE OR REPLACE FUNCTION recipe_components_orphan_on_product_delete()
    RETURNS trigger LANGUAGE plpgsql AS $$
    BEGIN
      DELETE FROM recipe_components WHERE parent_kind='product' AND parent_id = OLD.id;
      RETURN OLD;
    END; $$;
    DROP TRIGGER IF EXISTS trg_recipe_orphan_product ON products;
    CREATE TRIGGER trg_recipe_orphan_product AFTER DELETE ON products
      FOR EACH ROW EXECUTE FUNCTION recipe_components_orphan_on_product_delete();
  `);
}

export async function down(): Promise<void> { /* Forward-only; inert. */ }
```

## Apply guide
1. `pnpm migrate:create sensor-bus-now` → paste Migration 1 body into the new stub.
2. `pnpm migrate:create bom-seams` → paste Migration 2 body.
3. Staging DB first: `flyctl proxy 5433:5432 -a dowiz-staging-db` then `DATABASE_URL_MIGRATIONS=… pnpm migrate:up` (or the staging CI release_command). Verify with `\d orders`, `\d order_sensor_events`, RLS `\dp`.
4. Prod runs via `release_command` on merge to main (no new prod surface — inert until the runtime lands).

## Notes
- **Stock runtime (decrement/restock) is NOT here** (Option B). The deferred follow-up adds the
  SECURITY-DEFINER restock fn + decrement-at-confirm + the anti-cheat-green DoD (ADR-0007 v4).
- `order_status_history` per-transition timestamps already exist (orderStatusService) — no migration.
- After these land, the runtime (geofence detection, promised_window/live_eta writers, funnel ingest,
  manual bridges) is plain app code in `apps/api`/`apps/web` (not protected) — buildable then.
