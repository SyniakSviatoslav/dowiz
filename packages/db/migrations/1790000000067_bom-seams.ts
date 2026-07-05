import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * BOM SEAMS (§6.2) — council-approved `mvp-sensor-seams` (ADR-0008 v3). Inert: laid now, runtime
 * FLAT/manual, no reader yet. recipe_components.parent_id is POLYMORPHIC (no native FK) — the
 * manual→derived upgrade swaps the READER, not these rows. Orphan protection via triggers.
 * Forward-only; down() no-op.
 */
export async function up(pgm: MigrationBuilder): Promise<void> {
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
    DO $GR$ BEGIN IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname='deliveryos_api_user') THEN
      GRANT SELECT,INSERT,UPDATE,DELETE ON ingredients TO deliveryos_api_user; END IF; END $GR$;
  `);

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
    DO $GR$ BEGIN IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname='deliveryos_api_user') THEN
      GRANT SELECT,INSERT,UPDATE,DELETE ON recipe_components TO deliveryos_api_user; END IF; END $GR$;
  `);

  // Orphan protection for the un-FK'd parent_id (ADR-0008 v3) — DELETE + TRUNCATE companions.
  pgm.sql(`
    CREATE OR REPLACE FUNCTION recipe_components_orphan_on_product_delete()
    RETURNS trigger LANGUAGE plpgsql AS $FN$
    BEGIN
      DELETE FROM recipe_components WHERE parent_kind='product' AND parent_id = OLD.id;
      RETURN OLD;
    END; $FN$;
    DROP TRIGGER IF EXISTS trg_recipe_orphan_product ON products;
    CREATE TRIGGER trg_recipe_orphan_product AFTER DELETE ON products
      FOR EACH ROW EXECUTE FUNCTION recipe_components_orphan_on_product_delete();
  `);
}

export async function down(): Promise<void> { /* Forward-only; inert. */ }
