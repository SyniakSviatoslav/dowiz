import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders ADD COLUMN IF NOT EXISTS preflight jsonb;

    COMMENT ON COLUMN orders.preflight IS 'Preflight assessment: { outcome, reasons, confirmedReasons, serverIpHash, computedAt }. Populated on clean or soft-confirmed order creation. NULL if no preflight ran.';
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders DROP COLUMN IF EXISTS preflight;
  `);
}
