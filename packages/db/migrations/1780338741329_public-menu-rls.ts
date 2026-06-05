import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Allow anyone to read categories and products (public menu)
  pgm.sql(`
    CREATE POLICY public_select ON categories FOR SELECT
      USING (true);
  `);
  pgm.sql(`
    CREATE POLICY public_select ON products FOR SELECT
      USING (true);
  `);
  pgm.sql(`
    CREATE POLICY public_select ON locations FOR SELECT
      USING (true);
  `);
}

export async function down(): Promise<void> {}
