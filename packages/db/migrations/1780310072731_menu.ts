import type { MigrationBuilder } from 'node-pg-migrate';



export async function up(pgm: MigrationBuilder): Promise<void> {
  // 1. Categories
  pgm.sql(`
    CREATE TABLE categories (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id),
      name text NOT NULL,
      sort_order int NOT NULL DEFAULT 0,
      created_at timestamptz NOT NULL DEFAULT now()
    );
  `);

  // 2. Products
  pgm.sql(`
    CREATE TABLE products (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id),
      category_id uuid REFERENCES categories(id),
      name text NOT NULL,
      description text,
      price integer NOT NULL CHECK (price >= 0),
      is_available boolean NOT NULL DEFAULT true,
      image_url text,
      sort_order int NOT NULL DEFAULT 0,
      created_at timestamptz NOT NULL DEFAULT now()
    );
  `);

  // 3. RLS
  pgm.sql(`
    ALTER TABLE categories ENABLE ROW LEVEL SECURITY;
    ALTER TABLE categories FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON categories
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);

  pgm.sql(`
    ALTER TABLE products ENABLE ROW LEVEL SECURITY;
    ALTER TABLE products FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON products
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);
}

export async function down(): Promise<void> {}
