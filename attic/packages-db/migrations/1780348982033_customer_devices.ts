import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE IF NOT EXISTS customer_devices (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      customer_id uuid NOT NULL,
      platform text NOT NULL CHECK (platform IN ('fcm', 'apns', 'webpush')),
      token_encrypted text NOT NULL,
      fingerprint text NOT NULL,
      created_at timestamptz NOT NULL DEFAULT now(),
      last_seen_at timestamptz NOT NULL DEFAULT now()
    );
  `);

  pgm.sql(`CREATE UNIQUE INDEX IF NOT EXISTS customer_devices_fingerprint_idx ON customer_devices(fingerprint);`);
  pgm.sql(`CREATE INDEX IF NOT EXISTS customer_devices_customer_idx ON customer_devices(customer_id);`);

  // Not enabling RLS yet as it's just a scaffold for Phase 4
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS customer_devices CASCADE;`);
}
