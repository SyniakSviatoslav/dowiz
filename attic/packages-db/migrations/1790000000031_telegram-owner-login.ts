import type { MigrationBuilder } from 'node-pg-migrate';

// Telegram owner-login (TG). Owner authenticates via the bot: web mints a short-lived
// login token → deep-link /start login_<token> → the bot maps the Telegram identity to
// an owner user (creating one on first login) → web polls and receives the owner JWT.
//
//   users.telegram_user_id  maps a Telegram identity to exactly one owner (UNIQUE).
//   telegram_login_tokens   ephemeral, single-use, short-TTL handshake token. Pre-auth,
//                           accessed only by explicit token lookup on the operational
//                           pool — no RLS (like the other pre-auth token tables).

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE users ADD COLUMN IF NOT EXISTS telegram_user_id text UNIQUE;

    CREATE TABLE telegram_login_tokens (
      token uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'authenticated', 'consumed')),
      user_id uuid REFERENCES users(id) ON DELETE CASCADE,
      telegram_user_id text,
      expires_at timestamptz NOT NULL,
      created_at timestamptz NOT NULL DEFAULT now()
    );
    CREATE INDEX telegram_login_tokens_expires_idx ON telegram_login_tokens(expires_at);
  `);

  // DML grants mirror orders so the operational role (start inserts, bot updates,
  // poll reads/updates) can write the token table regardless of the live role name.
  pgm.sql(`
    DO $$
    DECLARE r record;
    BEGIN
      FOR r IN
        SELECT DISTINCT grantee, privilege_type FROM information_schema.role_table_grants
        WHERE table_schema='public' AND table_name='orders'
          AND privilege_type IN ('SELECT','INSERT','UPDATE','DELETE') AND grantee <> 'PUBLIC'
      LOOP
        EXECUTE format('GRANT %s ON public.telegram_login_tokens TO %I', r.privilege_type, r.grantee);
      END LOOP;
    END
    $$;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE IF EXISTS telegram_login_tokens CASCADE;
    ALTER TABLE users DROP COLUMN IF EXISTS telegram_user_id;
  `);
}
