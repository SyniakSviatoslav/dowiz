import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Drop the placeholder promotions table from the old planned-feature migration
  pgm.sql(`DROP TABLE IF EXISTS promotions CASCADE;`);

  // Create the updated_at trigger function if it doesn't exist
  pgm.sql(`
    CREATE OR REPLACE FUNCTION update_updated_at_column()
    RETURNS TRIGGER AS $$
    BEGIN
      NEW.updated_at = now();
      RETURN NEW;
    END;
    $$ LANGUAGE plpgsql;
  `);

  // Create the full promotions table
  pgm.sql(`
    CREATE TABLE promotions (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      code text NOT NULL,
      type text NOT NULL CHECK (type IN ('percentage', 'fixed', 'free_delivery')),
      discount_value integer NOT NULL CHECK (discount_value > 0),
      min_order_amount integer DEFAULT 0 CHECK (min_order_amount >= 0),
      max_uses integer DEFAULT NULL CHECK (max_uses IS NULL OR max_uses > 0),
      current_uses integer NOT NULL DEFAULT 0 CHECK (current_uses >= 0),
      max_uses_per_customer integer DEFAULT 1,
      valid_from timestamptz NOT NULL DEFAULT now(),
      valid_until timestamptz,
      is_active boolean NOT NULL DEFAULT true,
      applicable_product_ids uuid[] DEFAULT '{}',
      description text,
      created_at timestamptz NOT NULL DEFAULT now(),
      updated_at timestamptz NOT NULL DEFAULT now()
    );
  `);

  // Unique code per location
  pgm.sql(`CREATE UNIQUE INDEX idx_promotions_location_code ON promotions (location_id, code);`);

  // Index on location_id for lookups
  pgm.sql(`CREATE INDEX idx_promotions_location_id ON promotions (location_id);`);

  // RLS
  pgm.sql(`
    ALTER TABLE promotions ENABLE ROW LEVEL SECURITY;
    ALTER TABLE promotions FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON promotions
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);

  // Trigger for auto-updating updated_at
  pgm.sql(`
    CREATE TRIGGER set_promotions_updated_at
      BEFORE UPDATE ON promotions
      FOR EACH ROW
      EXECUTE FUNCTION update_updated_at_column();
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TRIGGER IF EXISTS set_promotions_updated_at ON promotions;`);
  pgm.sql(`DROP TABLE IF EXISTS promotions;`);
}
