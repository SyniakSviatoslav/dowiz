import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Allow anyone to read locations (public info like name, status, etc)
  pgm.sql(`
    DROP POLICY IF EXISTS public_select ON locations;
    CREATE POLICY public_select ON locations FOR SELECT
      USING (true);
  `);
}

export async function down(): Promise<void> {}
