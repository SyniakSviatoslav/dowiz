import type { MigrationBuilder } from 'node-pg-migrate';

// UX-4 tips. A single optional courier tip amount (integer minor units, ALL),
// replacing fixed %-badges. Cash tips are informative — the courier collects
// them and they show in stats; the cash-reconciliation/payout math is unchanged
// here. Card-tip charging is deferred. Inherits orders tenant_isolation RLS.
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`ALTER TABLE orders ADD COLUMN IF NOT EXISTS tip_amount integer NOT NULL DEFAULT 0 CHECK (tip_amount >= 0);`);
}

export async function down(): Promise<void> {
  // Forward-only per migration discipline.
}
