import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE IF NOT EXISTS telegram_connect_tokens (
      token uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      owner_id uuid NOT NULL,
      expires_at timestamptz NOT NULL,
      used_at timestamptz,
      chat_id_pending text,
      created_at timestamptz NOT NULL DEFAULT now()
    );
  `);

  pgm.sql(`CREATE INDEX IF NOT EXISTS telegram_connect_tokens_loc_idx ON telegram_connect_tokens(location_id);`);

  pgm.sql(`ALTER TABLE telegram_connect_tokens ENABLE ROW LEVEL SECURITY;`);

  pgm.sql(`
    CREATE POLICY telegram_connect_tokens_owner_all ON telegram_connect_tokens 
    FOR ALL
    TO authenticated
    USING (owner_id = (current_setting('request.jwt.claim.sub', true))::uuid)
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS telegram_connect_tokens CASCADE;`);
}
