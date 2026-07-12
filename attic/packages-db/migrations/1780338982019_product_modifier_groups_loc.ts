import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Add column allowing NULL first
  pgm.sql(`
    ALTER TABLE product_modifier_groups
      ADD COLUMN location_id uuid REFERENCES locations(id);
  `);

  // Populate location_id by joining products
  pgm.sql(`
    UPDATE product_modifier_groups pmg
    SET location_id = p.location_id
    FROM products p
    WHERE p.id = pmg.product_id;
  `);

  // Make it NOT NULL
  pgm.sql(`
    ALTER TABLE product_modifier_groups
      ALTER COLUMN location_id SET NOT NULL;
  `);
}

export async function down(): Promise<void> {}
