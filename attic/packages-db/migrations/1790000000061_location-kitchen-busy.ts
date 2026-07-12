import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * MENU-AVAILABILITY · `locations.kitchen_busy_until` — surfaces the contract's
 * `status: 'busy'` (distinct from 'closed').
 *
 * The public contract already enumerates status open|closed|busy, but no column
 * fed 'busy', so the eater only ever saw open/closed. This adds a NULLABLE
 * timestamptz an owner sets to "kitchen busy / raised ETA until <t>". The /info
 * endpoint derives status = busy when the venue is OPEN and now() < this value;
 * NULL or past => the existing open/closed logic is untouched (purely additive).
 *
 * Forward-only: nullable, no default, no row rewrite. down() drops the column.
 */
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE locations
      ADD COLUMN IF NOT EXISTS kitchen_busy_until timestamptz;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`ALTER TABLE locations DROP COLUMN IF EXISTS kitchen_busy_until;`);
}
