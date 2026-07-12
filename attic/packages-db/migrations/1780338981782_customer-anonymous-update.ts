import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Allow anonymous UPDATE on customers for ON CONFLICT DO UPDATE
  pgm.sql(`
    CREATE POLICY anonymous_update ON customers FOR UPDATE
      USING (app_current_user() IS NULL);
  `);
  pgm.sql(`
    CREATE POLICY anonymous_select ON customers FOR SELECT
      USING (app_current_user() IS NULL);
  `);
}

export async function down(): Promise<void> {}
