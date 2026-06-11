import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      ADD COLUMN IF NOT EXISTS user_id uuid NOT NULL DEFAULT gen_random_uuid(),
      ADD CONSTRAINT fk_user
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE;
  `);

  // Backfill existing rows: set user_id to the owner_id from the membership for that location?
  // We cannot know for sure, so we will set it to null and then update via the linking process.
  // But we made it NOT NULL, so we need to set a default? We'll change to allow null temporarily.
  // Let's change the column to be nullable.
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      ALTER COLUMN user_id DROP NOT NULL;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      DROP CONSTRAINT IF EXISTS fk_user;
  `);
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      DROP COLUMN IF EXISTS user_id;
  `);
}