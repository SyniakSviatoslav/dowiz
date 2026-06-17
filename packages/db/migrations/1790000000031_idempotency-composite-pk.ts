import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE idempotency_keys DROP CONSTRAINT idempotency_keys_pkey;
    ALTER TABLE idempotency_keys ADD PRIMARY KEY (location_id, key);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE idempotency_keys DROP CONSTRAINT idempotency_keys_pkey;
    ALTER TABLE idempotency_keys ADD PRIMARY KEY (key);
  `);
}
