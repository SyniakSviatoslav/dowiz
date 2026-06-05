import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE POLICY anonymous_select ON orders FOR SELECT
      USING (app_current_user() IS NULL);
  `);
  pgm.sql(`
    CREATE POLICY anonymous_select ON order_items FOR SELECT
      USING (app_current_user() IS NULL);
  `);
}

export async function down(): Promise<void> {}
