import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`ALTER TYPE payment_outcome ADD VALUE IF NOT EXISTS 'delivered_prepaid';`);
}

export async function down(): Promise<void> {
  // Never executed according to discipline, but keeping forward-only logic.
}
