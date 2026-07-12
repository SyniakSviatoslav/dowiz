import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE locations
      ADD COLUMN IF NOT EXISTS fallback_config jsonb NOT NULL DEFAULT '{}'::jsonb
        CHECK (jsonb_typeof(fallback_config) = 'object');

    COMMENT ON COLUMN locations.fallback_config IS
      'P5-3: Fallback phone & degradation config per location. '
      'Keys: phone (text), show_phone_on_error (bool), show_phone_on_offline (bool), '
      'ws_retry_max (int), ws_retry_base_ms (int).';

    -- Customer contact reveals audit table
    CREATE TABLE IF NOT EXISTS customer_contact_reveals (
      id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      order_id   uuid NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
      customer_id uuid NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
      location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      revealed_by_owner_id uuid REFERENCES users(id),
      reason     text,
      created_at timestamptz NOT NULL DEFAULT now()
    );

    CREATE INDEX IF NOT EXISTS idx_contact_reveals_location ON customer_contact_reveals(location_id, created_at DESC);
    CREATE INDEX IF NOT EXISTS idx_contact_reveals_order ON customer_contact_reveals(order_id);

    COMMENT ON TABLE customer_contact_reveals IS 'P5-3: Audit trail for owner-initiated customer contact reveals. 0 PII.';
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE locations DROP COLUMN IF EXISTS fallback_config;
    DROP TABLE IF EXISTS customer_contact_reveals CASCADE;
  `);
}
