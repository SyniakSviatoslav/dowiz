import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE modifier_groups (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id),
      name text NOT NULL,
      min_select int NOT NULL DEFAULT 0 CHECK (min_select >= 0),
      max_select int NOT NULL DEFAULT 1 CHECK (max_select >= min_select),
      required boolean NOT NULL DEFAULT false,
      created_at timestamptz NOT NULL DEFAULT now()
    );

    CREATE TABLE modifiers (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      group_id uuid NOT NULL REFERENCES modifier_groups(id) ON DELETE CASCADE,
      location_id uuid NOT NULL REFERENCES locations(id),
      name text NOT NULL,
      price_delta integer NOT NULL DEFAULT 0 CHECK (price_delta >= 0),
      available boolean NOT NULL DEFAULT true,
      sort_order int NOT NULL DEFAULT 0
    );

    CREATE TABLE product_modifier_groups (
      product_id uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
      group_id   uuid NOT NULL REFERENCES modifier_groups(id) ON DELETE CASCADE,
      sort_order int NOT NULL DEFAULT 0,
      PRIMARY KEY (product_id, group_id)
    );

    CREATE TABLE order_item_modifiers (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      order_item_id uuid NOT NULL REFERENCES order_items(id) ON DELETE CASCADE,
      modifier_id uuid REFERENCES modifiers(id),
      name_snapshot text NOT NULL,
      price_delta_snapshot integer NOT NULL CHECK (price_delta_snapshot >= 0)
    );
  `);

  // ENABLE AND FORCE RLS
  pgm.sql(`
    ALTER TABLE modifier_groups ENABLE ROW LEVEL SECURITY;
    ALTER TABLE modifier_groups FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON modifier_groups
      USING ( location_id IN (SELECT app_member_location_ids()) );

    ALTER TABLE modifiers ENABLE ROW LEVEL SECURITY;
    ALTER TABLE modifiers FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON modifiers
      USING ( location_id IN (SELECT app_member_location_ids()) );

    ALTER TABLE product_modifier_groups ENABLE ROW LEVEL SECURITY;
    ALTER TABLE product_modifier_groups FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON product_modifier_groups
      USING ( product_id IN (SELECT id FROM products WHERE location_id IN (SELECT app_member_location_ids())) );

    ALTER TABLE order_item_modifiers ENABLE ROW LEVEL SECURITY;
    ALTER TABLE order_item_modifiers FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON order_item_modifiers
      USING ( order_item_id IN (
        SELECT oi.id FROM order_items oi
        JOIN orders o ON o.id = oi.order_id
        WHERE o.location_id IN (SELECT app_member_location_ids())
      ));
  `);
}

export async function down(): Promise<void> {}
