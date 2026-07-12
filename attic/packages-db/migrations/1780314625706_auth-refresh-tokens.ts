import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE auth_refresh_tokens (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      family_id uuid NOT NULL,
      token_hash text NOT NULL,
      used boolean NOT NULL DEFAULT false,
      expires_at timestamptz NOT NULL,
      created_at timestamptz NOT NULL DEFAULT now()
    );
    CREATE INDEX auth_refresh_tokens_hash_idx ON auth_refresh_tokens(token_hash);
    CREATE INDEX auth_refresh_tokens_user_id_idx ON auth_refresh_tokens(user_id);
    CREATE INDEX auth_refresh_tokens_family_idx ON auth_refresh_tokens(family_id);

    -- Non-tenant, no RLS
    ALTER TABLE auth_refresh_tokens DISABLE ROW LEVEL SECURITY;
  `);
}

export async function down(): Promise<void> {}
