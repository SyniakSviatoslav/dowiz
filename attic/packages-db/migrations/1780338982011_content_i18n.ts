import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE product_translations (
      product_id uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
      locale text NOT NULL,
      name text NOT NULL,
      description text,
      PRIMARY KEY (product_id, locale)
    );

    CREATE TABLE category_translations (
      category_id uuid NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
      locale text NOT NULL,
      name text NOT NULL,
      PRIMARY KEY (category_id, locale)
    );

    ALTER TABLE locations
      ADD COLUMN default_locale text NOT NULL DEFAULT 'sq',
      ADD COLUMN supported_locales text[] NOT NULL DEFAULT ARRAY['sq','en'];
  `);

  pgm.sql(`
    ALTER TABLE product_translations ENABLE ROW LEVEL SECURITY;
    ALTER TABLE product_translations FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON product_translations
      USING ( product_id IN (SELECT id FROM products WHERE location_id IN (SELECT app_member_location_ids())) );

    ALTER TABLE category_translations ENABLE ROW LEVEL SECURITY;
    ALTER TABLE category_translations FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON category_translations
      USING ( category_id IN (SELECT id FROM categories WHERE location_id IN (SELECT app_member_location_ids())) );
  `);
}

export async function down(): Promise<void> {}
