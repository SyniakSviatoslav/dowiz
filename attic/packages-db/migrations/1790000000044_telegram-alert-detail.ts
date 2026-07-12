import type { MigrationBuilder } from 'node-pg-migrate';

// P0-4 (ADR-p0-privacy-hardening) — per-location Telegram alert detail level.
// Default 'area': the owner alert body carries order# + items/total + a coarse area
// (district/street, NO house number) + an authenticated deep-link; full address/phone
// live only behind the link. 'full' is an explicit owner opt-in (writes the home address
// into Telegram history — accepted risk, canary = full opt-in rate). 'minimal' = no
// address at all. Forward-only, additive; render defaults to 'area' even if unset.
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE locations
      ADD COLUMN IF NOT EXISTS telegram_alert_detail text NOT NULL DEFAULT 'area'
        CHECK (telegram_alert_detail IN ('full', 'area', 'minimal'));
  `);
}

export async function down(): Promise<void> {
  // Forward-only.
}
