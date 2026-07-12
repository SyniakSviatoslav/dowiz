import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE IF NOT EXISTS owner_notification_targets (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      channel text NOT NULL CHECK (channel IN ('telegram', 'push')),
      address text NOT NULL,
      status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'active', 'disabled', 'disconnected')),
      prefs jsonb NOT NULL DEFAULT '{"order.created": true, "order.pending_aging": true}'::jsonb,
      created_at timestamptz NOT NULL DEFAULT now(),
      last_error text,
      disabled_at timestamptz
    );
  `);

  pgm.sql(`CREATE UNIQUE INDEX IF NOT EXISTS owner_notification_targets_uniq ON owner_notification_targets(location_id, channel, address);`);

  pgm.sql(`ALTER TABLE owner_notification_targets ENABLE ROW LEVEL SECURITY;`);

  pgm.sql(`
    CREATE POLICY owner_notification_targets_owner_all ON owner_notification_targets 
    FOR ALL
    TO authenticated
    USING (
      location_id IN (
        SELECT location_id FROM memberships WHERE user_id = (current_setting('request.jwt.claim.sub', true))::uuid
      )
    );
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS owner_notification_targets CASCADE;`);
}
