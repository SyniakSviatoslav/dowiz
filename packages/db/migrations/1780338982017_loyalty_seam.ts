import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE customers
      ADD COLUMN marketing_opt_in boolean NOT NULL DEFAULT false,
      ADD COLUMN loyalty_points integer NOT NULL DEFAULT 0 CHECK (loyalty_points >= 0);
  `);
}

export async function down(): Promise<void> {}
