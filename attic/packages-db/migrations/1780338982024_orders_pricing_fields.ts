import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Add required fields
  pgm.sql(`
ALTER TABLE orders
  DROP COLUMN IF EXISTS cash_pay_with,
  ADD COLUMN cash_pay_with integer,
  ADD COLUMN currency_code text NOT NULL DEFAULT 'ALL',
      ADD COLUMN menu_version bigint NOT NULL DEFAULT 0,
      ADD COLUMN client_menu_version bigint,
      ADD COLUMN request_hash text NOT NULL DEFAULT 'legacy';
  `);

  // Drop default constraint on request_hash to force explicit values going forward
  pgm.sql(`
    ALTER TABLE orders ALTER COLUMN request_hash DROP DEFAULT;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders
      DROP COLUMN IF EXISTS cash_pay_with,
      DROP COLUMN IF EXISTS currency_code,
      DROP COLUMN IF EXISTS menu_version,
      DROP COLUMN IF EXISTS client_menu_version,
      DROP COLUMN IF EXISTS request_hash;
      
    ALTER TABLE orders ADD COLUMN cash_pay_with integer;
  `);
}
