import type { MigrationBuilder } from 'node-pg-migrate';

// Extend owner_notification_targets.channel to allow the new 'whatsapp' channel
// alongside the existing 'telegram' and 'push'.
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      DROP CONSTRAINT IF EXISTS owner_notification_targets_channel_check;
  `);
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      ADD CONSTRAINT owner_notification_targets_channel_check
      CHECK (channel IN ('telegram', 'push', 'whatsapp'));
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DELETE FROM owner_notification_targets WHERE channel = 'whatsapp';
  `);
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      DROP CONSTRAINT IF EXISTS owner_notification_targets_channel_check;
  `);
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      ADD CONSTRAINT owner_notification_targets_channel_check
      CHECK (channel IN ('telegram', 'push'));
  `);
}
