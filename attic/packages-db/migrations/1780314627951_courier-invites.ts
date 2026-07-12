import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE courier_invites (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id),
      code_hash text NOT NULL,
      created_by uuid REFERENCES users(id),
      expires_at timestamptz NOT NULL,
      used_at timestamptz,
      created_at timestamptz NOT NULL DEFAULT now()
    );

    ALTER TABLE courier_invites ENABLE ROW LEVEL SECURITY;
    ALTER TABLE courier_invites FORCE ROW LEVEL SECURITY;
    
    CREATE POLICY tenant_isolation ON courier_invites
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);
}

export async function down(): Promise<void> {}
