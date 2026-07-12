import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    UPDATE locations SET slug = 'sushi-durres' WHERE slug = 'pizza-roma';
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    UPDATE locations SET slug = 'pizza-roma' WHERE slug = 'sushi-durres';
  `);
}
