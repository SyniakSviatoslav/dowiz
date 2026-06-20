import type { MigrationBuilder } from 'node-pg-migrate';

// idempotency_keys: single-column PK on `key` → composite (location_id, key).
// Lets two tenants reuse the same client-generated key string independently; the
// route already queries/inserts with (key, location_id). Safe: `key` was globally
// unique, so no (location_id, key) duplicates exist.

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE idempotency_keys DROP CONSTRAINT IF EXISTS idempotency_keys_pkey;
    ALTER TABLE idempotency_keys ADD PRIMARY KEY (location_id, key);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE idempotency_keys DROP CONSTRAINT IF EXISTS idempotency_keys_pkey;
    ALTER TABLE idempotency_keys ADD PRIMARY KEY (key);
  `);
}
