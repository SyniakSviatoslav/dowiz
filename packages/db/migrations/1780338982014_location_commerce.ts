import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE locations
      ADD COLUMN currency_code text NOT NULL DEFAULT 'ALL',
      ADD COLUMN currency_minor_unit int NOT NULL DEFAULT 0,
      ADD COLUMN tax_rate numeric NOT NULL DEFAULT 0,
      ADD COLUMN price_includes_tax boolean NOT NULL DEFAULT true,
      ADD COLUMN min_order_value integer,
      ADD COLUMN free_delivery_threshold integer,
      ADD COLUMN delivery_fee_flat integer,
      ADD COLUMN delivery_polygon jsonb;

    CREATE TABLE delivery_tiers (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id),
      max_distance_km numeric NOT NULL,
      fee integer NOT NULL CHECK (fee >= 0),
      min_order integer
    );
  `);

  pgm.sql(`
    ALTER TABLE delivery_tiers ENABLE ROW LEVEL SECURITY;
    ALTER TABLE delivery_tiers FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON delivery_tiers
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);
}

export async function down(): Promise<void> {}
