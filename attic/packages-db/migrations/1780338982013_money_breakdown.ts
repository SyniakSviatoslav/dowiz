import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders
      ADD COLUMN delivery_fee   integer NOT NULL DEFAULT 0 CHECK (delivery_fee   >= 0),
      ADD COLUMN discount_total integer NOT NULL DEFAULT 0 CHECK (discount_total >= 0),
      ADD COLUMN tax_total      integer NOT NULL DEFAULT 0 CHECK (tax_total      >= 0);
  `);
}

export async function down(): Promise<void> {}
