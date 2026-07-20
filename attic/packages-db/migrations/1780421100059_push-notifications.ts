import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Extend customer_devices for Web Push subscriptions (P28)
  pgm.sql(`
    ALTER TABLE customer_devices
      ADD COLUMN IF NOT EXISTS subject_type text NOT NULL DEFAULT 'customer'
        CHECK (subject_type IN ('customer')),
      ADD COLUMN IF NOT EXISTS opted_in boolean NOT NULL DEFAULT false,
      ADD COLUMN IF NOT EXISTS push_subscription jsonb,
      ADD COLUMN IF NOT EXISTS vapid_endpoint text,
      ADD COLUMN IF NOT EXISTS keys_p256dh text,
      ADD COLUMN IF NOT EXISTS keys_auth text;
  `);

  // RLS customer_devices
  pgm.sql(`
    ALTER TABLE customer_devices ENABLE ROW LEVEL SECURITY;
    ALTER TABLE customer_devices FORCE ROW LEVEL SECURITY;
  `);
  pgm.sql(`
    DROP POLICY IF EXISTS customer_devices_owner_all ON customer_devices;
  `);
  pgm.sql(`
    CREATE POLICY customer_devices_owner_all ON customer_devices
      FOR ALL TO authenticated
      USING (customer_id IN (SELECT app_current_user()));
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE customer_devices
      DROP COLUMN IF EXISTS subject_type,
      DROP COLUMN IF EXISTS opted_in,
      DROP COLUMN IF EXISTS push_subscription,
      DROP COLUMN IF EXISTS vapid_endpoint,
      DROP COLUMN IF EXISTS keys_p256dh,
      DROP COLUMN IF EXISTS keys_auth;
  `);
  pgm.sql(`DROP POLICY IF EXISTS customer_devices_owner_all ON customer_devices;`);
  pgm.sql(`ALTER TABLE customer_devices DISABLE ROW LEVEL SECURITY;`);
}
