import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Rename owner_id to user_id
  pgm.sql(`
    ALTER TABLE telegram_connect_tokens
      RENAME COLUMN owner_id TO user_id;
  `);

  // Update the policy
  pgm.sql(`
    DROP POLICY IF EXISTS telegram_connect_tokens_owner_all ON telegram_connect_tokens;
  `);
  pgm.sql(`
    CREATE POLICY telegram_connect_tokens_user_all ON telegram_connect_tokens 
    FOR ALL
    TO authenticated
    USING (user_id = (current_setting('request.jwt.claim.sub', true))::uuid);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  // Revert the policy
  pgm.sql(`
    DROP POLICY IF EXISTS telegram_connect_tokens_user_all ON telegram_connect_tokens;
  `);
  pgm.sql(`
    CREATE POLICY telegram_connect_tokens_owner_all ON telegram_connect_tokens 
    FOR ALL
    TO authenticated
    USING (owner_id = (current_setting('request.jwt.claim.sub', true))::uuid);
  `);

  // Rename user_id back to owner_id
  pgm.sql(`
    ALTER TABLE telegram_connect_tokens
      RENAME COLUMN user_id TO owner_id;
  `);
}