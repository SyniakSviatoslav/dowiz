import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE courier_sessions (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      courier_id uuid NOT NULL REFERENCES couriers(id) ON DELETE CASCADE,
      family_id uuid NOT NULL,
      token_hash text NOT NULL,
      active_location_id uuid NOT NULL REFERENCES locations(id),
      issued_at timestamptz NOT NULL DEFAULT now(),
      expires_at timestamptz NOT NULL,
      revoked_at timestamptz,
      replaced_by uuid REFERENCES courier_sessions(id),
      last_used_at timestamptz,
      user_agent_hash text,
      ip_hash text
    );
    CREATE INDEX courier_sessions_courier_idx ON courier_sessions(courier_id);
    CREATE INDEX courier_sessions_family_idx ON courier_sessions(family_id);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE courier_sessions;
  `);
}
