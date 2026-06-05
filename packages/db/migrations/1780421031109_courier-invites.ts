import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE IF EXISTS courier_invites CASCADE;
    CREATE TABLE courier_invites (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id),
      created_by_owner_id uuid NOT NULL REFERENCES users(id),
      role text NOT NULL DEFAULT 'courier' CHECK (role IN ('courier', 'dispatcher')),
      invited_email_hash text NOT NULL,
      code_hash text NOT NULL,
      expires_at timestamptz NOT NULL,
      used_at timestamptz,
      used_by_courier_id uuid REFERENCES couriers(id),
      revoked_at timestamptz,
      created_at timestamptz NOT NULL DEFAULT now()
    );
    CREATE INDEX courier_invites_loc_active_idx ON courier_invites(location_id) WHERE used_at IS NULL AND revoked_at IS NULL;
    
    ALTER TABLE courier_invites ENABLE ROW LEVEL SECURITY;
    CREATE POLICY isolate_courier_invites ON courier_invites
      USING (location_id = current_setting('app.current_tenant')::uuid);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE courier_invites;
  `);
}
