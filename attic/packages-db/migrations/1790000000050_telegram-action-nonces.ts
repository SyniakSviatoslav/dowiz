import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * TG-NOTIF-3 · One-shot nonces for Telegram storefront-toggle confirmation (BR-15 / BR-19).
 *
 * The bot's confirm step (close/open) echoes the location and carries a single-use nonce.
 * Consumed via DELETE ... RETURNING (atomic one-shot) so a replayed/duplicate callback
 * cannot re-toggle. Short TTL (set by the writing handler). location_id is trusted from the
 * nonce row (not from callback_data) per the authority model.
 *
 * RLS mirrors order_ratings: FORCE + tenant_isolation on app.user_id. The webhook handler
 * sets app.user_id = resolved member before INSERT/DELETE, so the policy matches.
 * DML grants inherit the same default-privilege path that makes order_ratings writable.
 */
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE telegram_action_nonces (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      nonce text NOT NULL UNIQUE,
      location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      user_id uuid NOT NULL,
      action text NOT NULL CHECK (action IN ('store.open','store.close')),
      chat_id text NOT NULL,
      expires_at timestamptz NOT NULL,
      created_at timestamptz NOT NULL DEFAULT now()
    );
    CREATE INDEX telegram_action_nonces_expiry_idx ON telegram_action_nonces(expires_at);
    CREATE INDEX telegram_action_nonces_location_idx ON telegram_action_nonces(location_id);
  `);

  pgm.sql(`
    ALTER TABLE telegram_action_nonces ENABLE ROW LEVEL SECURITY;
    ALTER TABLE telegram_action_nonces FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON telegram_action_nonces
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS telegram_action_nonces;`);
}
