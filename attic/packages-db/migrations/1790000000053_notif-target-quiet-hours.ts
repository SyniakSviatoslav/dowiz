import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * TG-NOTIF-6 · Per-target configurable quiet-hours window for the dispatcher.
 *
 * Shape: { "from": <hour 0-23>, "to": <hour 0-23> } interpreted in the location's
 * timezone (locations.timezone, migration 1790000000049). NULL window = no quiet hours.
 * `from > to` is a normal overnight window (e.g. 22 → 8); `from === to` = disabled.
 *
 * Default + backfill to {22,8} preserves the protection of the current hardcoded
 * UTC 22:00–08:00 (now timezone-aware once TG_CATEGORY_GATING is on). The new dispatcher
 * HOLDS non-transactional events to the window's end (audit 'held') instead of dropping
 * them; transactional events always punch through (нуль тихих дропів).
 */
const DEFAULT_WINDOW = `'{"from": 22, "to": 8}'::jsonb`;

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      ADD COLUMN IF NOT EXISTS quiet_hours jsonb DEFAULT ${DEFAULT_WINDOW};
  `);

  // Shape guard: NULL, or both from/to are JSON numbers in [0,23].
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      DROP CONSTRAINT IF EXISTS owner_notification_targets_quiet_hours_check;
    ALTER TABLE owner_notification_targets
      ADD CONSTRAINT owner_notification_targets_quiet_hours_check CHECK (
        quiet_hours IS NULL OR (
          jsonb_typeof(quiet_hours->'from') = 'number'
          AND jsonb_typeof(quiet_hours->'to') = 'number'
          AND (quiet_hours->>'from')::int BETWEEN 0 AND 23
          AND (quiet_hours->>'to')::int   BETWEEN 0 AND 23
        )
      );
  `);

  // Backfill existing rows to the sensible default window.
  pgm.sql(`
    UPDATE owner_notification_targets
       SET quiet_hours = ${DEFAULT_WINDOW}
     WHERE quiet_hours IS NULL;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      DROP CONSTRAINT IF EXISTS owner_notification_targets_quiet_hours_check;
  `);
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      DROP COLUMN IF EXISTS quiet_hours;
  `);
}
