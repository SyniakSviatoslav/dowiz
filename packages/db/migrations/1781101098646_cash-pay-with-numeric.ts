import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders DROP COLUMN IF EXISTS cash_pay_with;
    ALTER TABLE orders ADD COLUMN cash_pay_with integer;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders DROP COLUMN IF EXISTS cash_pay_with;
    ALTER TABLE orders ADD COLUMN cash_pay_with boolean NOT NULL DEFAULT false;
  `);
}
