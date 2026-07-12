import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * TG-NOTIF-5 · Add category keys to owner_notification_targets.prefs (additive backfill).
 *
 * Categories: operational (default ON), quality (default OFF). Transactional events are
 * NON-MUTABLE and NOT stored in prefs — the dispatcher always sends them, regardless of
 * toggle/quiet-hours (per the recorded ETHICAL invariant: category = reversibility of
 * consequence, not loudness; order.pending_aging signed as transactional).
 *
 * ADDITIVE & forward-compatible: existing per-event keys (order.created, order.pending_aging)
 * are LEFT IN PLACE so the CURRENT dispatcher (reads prefs[event], no flag) keeps working
 * until TG_CATEGORY_GATING flips. The new Policy-Gateway dispatcher reads prefs[category].
 *
 * BR-9 (lossy backfill) — non-issue: today prefs only ever holds order.created /
 * order.pending_aging (both transactional, non-mutable); NO operational/quality event is
 * currently per-event toggled, so defaulting operational=ON / quality=OFF loses no real intent.
 */
export async function up(pgm: MigrationBuilder): Promise<void> {
  // New rows: keep legacy event keys for the old dispatcher + add category defaults.
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      ALTER COLUMN prefs SET DEFAULT
      '{"order.created": true, "order.pending_aging": true, "operational": true, "quality": false}'::jsonb;
  `);

  // Backfill existing rows additively (only where category keys are absent).
  pgm.sql(`
    UPDATE owner_notification_targets
       SET prefs = prefs || '{"operational": true, "quality": false}'::jsonb
     WHERE NOT (prefs ? 'operational');
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      ALTER COLUMN prefs SET DEFAULT
      '{"order.created": true, "order.pending_aging": true}'::jsonb;
  `);
  // Backfilled category keys are left in place (forward-only data; harmless to old dispatcher).
}
