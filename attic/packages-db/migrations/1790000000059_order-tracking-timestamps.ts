import type { MigrationBuilder } from 'node-pg-migrate';

// ORDER-TRACKING vertical: per-transition timestamp instrumentation.
// Forward-only, additive, non-breaking. Adds the nullable *_at columns the
// 10-state machine transitions into but never stamped (preparing_at,
// in_delivery_at, picked_up_at) — ready_at/delivered_at/confirmed_at already
// exist (1780695000000_order_timelines + 1780310074262_orders). Also adds a
// nullable comment column to order_status_history for transition reasons
// (rejection/cancellation). NO notify column (excluded — Telegram coupling).
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders ADD COLUMN IF NOT EXISTS ready_at        timestamptz;
    ALTER TABLE orders ADD COLUMN IF NOT EXISTS preparing_at    timestamptz;
    ALTER TABLE orders ADD COLUMN IF NOT EXISTS in_delivery_at  timestamptz;
    ALTER TABLE orders ADD COLUMN IF NOT EXISTS picked_up_at    timestamptz;
  `);

  pgm.sql(`
    ALTER TABLE order_status_history ADD COLUMN IF NOT EXISTS comment text;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders DROP COLUMN IF EXISTS preparing_at;
    ALTER TABLE orders DROP COLUMN IF EXISTS in_delivery_at;
    ALTER TABLE orders DROP COLUMN IF EXISTS picked_up_at;
    ALTER TABLE order_status_history DROP COLUMN IF EXISTS comment;
  `);
}
