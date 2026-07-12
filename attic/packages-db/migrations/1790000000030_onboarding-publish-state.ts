import type { MigrationBuilder } from 'node-pg-migrate';

// Zero-friction onboarding (menu-first) — draft→publish two-state foundation (O0).
//
//   published_at      NULL = DRAFT (preview exists, not order-able). Set on publish.
//   pickup_enabled    fulfillment path: pickup-only avoids needing a courier.
//   menu_confirmed_at human-review gate: set when the owner commits a reviewed menu.
//
// `status` (open/closed) stays the DAILY open/close switch — orthogonal to publish.
// BACKFILL: any existing location that already has a menu (products) or is currently
// open is a real, live business → mark it published + menu-confirmed so the new
// gate never flips a live storefront back to draft.

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE locations
      ADD COLUMN IF NOT EXISTS published_at      timestamptz,
      ADD COLUMN IF NOT EXISTS pickup_enabled    boolean NOT NULL DEFAULT false,
      ADD COLUMN IF NOT EXISTS menu_confirmed_at timestamptz;

    UPDATE locations l SET
      published_at      = COALESCE(l.published_at, l.created_at),
      menu_confirmed_at = COALESCE(l.menu_confirmed_at, l.created_at)
    WHERE l.published_at IS NULL
      AND ( l.status = 'open'
            OR EXISTS (SELECT 1 FROM products p WHERE p.location_id = l.id) );
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE locations
      DROP COLUMN IF EXISTS published_at,
      DROP COLUMN IF EXISTS pickup_enabled,
      DROP COLUMN IF EXISTS menu_confirmed_at;
  `);
}
