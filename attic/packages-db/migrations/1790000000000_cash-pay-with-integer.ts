import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders ALTER COLUMN cash_pay_with TYPE integer USING
      CASE WHEN cash_pay_with::text = 'true' THEN 1
           WHEN cash_pay_with::text = 'false' THEN 0
           ELSE cash_pay_with::integer END;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders ALTER COLUMN cash_pay_with TYPE boolean USING
      CASE WHEN cash_pay_with = 1 THEN true
           WHEN cash_pay_with = 0 THEN false
           ELSE NULL END;
  `);
}
