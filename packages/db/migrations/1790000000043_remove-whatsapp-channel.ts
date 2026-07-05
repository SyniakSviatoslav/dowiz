import type { MigrationBuilder } from 'node-pg-migrate';

// P0-2 (ADR-p0-privacy-hardening) — retire the WhatsApp/Baileys owner-alert channel.
// Baileys streamed customer PII (name/phone/address) to Meta via an unofficial,
// ToS-violating client. The adapter + dep + env are removed in code; this migration
// handles the data side, forward-only, with NO silent notification loss:
//
//  1. Disable existing whatsapp targets (status='disabled') + stamp a reconfigure
//     reason so the owner UI can surface "re-add a channel" (F-2). We DO NOT delete
//     the rows (anonymize/disable-not-delete; the owner may still see history).
//  2. Add a NOT VALID anti-whatsapp CHECK so NEW inserts/updates can't use the
//     channel again, while the disabled legacy rows are tolerated (NOT VALID skips
//     validation of existing rows). We deliberately do NOT narrow the existing
//     owner_notification_targets_channel_check (DEV-1/R12: narrowing is not
//     forward-only and would fight the legacy rows). Never VALIDATE this constraint.
export async function up(pgm: MigrationBuilder): Promise<void> {
  // 1. Disable any active whatsapp targets, recording why (idempotent).
  pgm.sql(`
    UPDATE owner_notification_targets
       SET status = 'disabled',
           prefs = COALESCE(prefs, '{}'::jsonb) || '{"disabled_reason":"whatsapp_channel_removed_reconfigure"}'::jsonb
     WHERE channel = 'whatsapp' AND status = 'active';
  `);

  // 2. Reject NEW whatsapp rows without forcing a rewrite of legacy rows.
  pgm.sql(`
    ALTER TABLE owner_notification_targets
      DROP CONSTRAINT IF EXISTS owner_notification_targets_not_whatsapp;
    ALTER TABLE owner_notification_targets
      ADD CONSTRAINT owner_notification_targets_not_whatsapp
      CHECK (channel <> 'whatsapp') NOT VALID;
  `);
}

export async function down(): Promise<void> {
  // Forward-only. (The constraint can be dropped manually if ever reversed.)
}
