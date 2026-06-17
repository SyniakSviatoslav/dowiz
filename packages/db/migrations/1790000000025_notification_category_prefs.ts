import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Update default prefs to category-based model.
  // category_operations = true (on by default), category_analytics = false (off by default).
  // 'orders' category is implicit critical — no pref key needed; always sent.
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      ALTER COLUMN prefs SET DEFAULT '{"category_operations": true, "category_analytics": false}'::jsonb;
  `);

  // Migrate existing rows: merge category prefs into their existing event-level prefs.
  // We add the category keys only if not already present, preserving any existing event-level overrides.
  pgm.sql(`
    UPDATE owner_notification_targets
    SET prefs = prefs
      || '{"category_operations": true, "category_analytics": false}'::jsonb
    WHERE NOT (prefs ? 'category_operations');
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      ALTER COLUMN prefs SET DEFAULT '{"order.created": true, "order.pending_aging": true}'::jsonb;
  `);

  pgm.sql(`
    UPDATE owner_notification_targets
    SET prefs = prefs - 'category_operations' - 'category_analytics' - 'quiet_start' - 'quiet_end';
  `);
}
