import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders
      ADD CONSTRAINT cash_pay_with_non_negative CHECK (cash_pay_with >= 0);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders DROP CONSTRAINT IF EXISTS cash_pay_with_non_negative;
  `);
}
