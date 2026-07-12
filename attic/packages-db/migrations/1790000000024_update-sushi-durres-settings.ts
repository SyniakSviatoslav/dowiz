import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    UPDATE locations SET
      name = 'Dubin & Sushi',
      phone = '+355683085694',
      address = 'Rruga Sulejman Kadiu, Durrës',
      lat = 41.315347,
      lng = 19.4449964,
      status = 'active',
      hours_json = '{"monday":{"isOpen":true,"open":"10:00","close":"22:00"},"tuesday":{"isOpen":true,"open":"10:00","close":"22:00"},"wednesday":{"isOpen":true,"open":"10:00","close":"22:00"},"thursday":{"isOpen":true,"open":"10:00","close":"22:00"},"friday":{"isOpen":true,"open":"10:00","close":"22:00"},"saturday":{"isOpen":true,"open":"10:00","close":"22:00"},"sunday":{"isOpen":true,"open":"10:00","close":"22:00"}}'
    WHERE slug = 'sushi-durres';
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    UPDATE locations SET
      name = 'Pizza Roma',
      status = 'open'
    WHERE slug = 'sushi-durres';
  `);
}
