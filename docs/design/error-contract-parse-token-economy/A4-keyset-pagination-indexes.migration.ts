// ⚠️ APPROVAL-PENDING MIGRATION ARTIFACT (A4 / ADR-0010, B3/B13/B14)
// ---------------------------------------------------------------------------
// This file is a STAGED migration awaiting human approval. `packages/db/migrations/`
// is a protected red-line zone (protect-paths hook). To apply:
//   1. Review + move this file to:
//        packages/db/migrations/1790000000068_keyset-pagination-indexes.ts
//      (bump the timestamp if a higher-numbered migration has landed since).
//   2. Run on the STAGING DB FIRST (ship-discipline §2), via:
//        flyctl proxy 5433:5432 -a dowiz-staging-db
//        DATABASE_URL_MIGRATIONS=postgres://…@localhost:5433/… pnpm --filter @deliveryos/db migrate up
//   3. Then deploy + validate; prod only on merge to main.
//
// CORRECTNESS NOTE: the route fix (composite `(sort,id)` keyset in owner/dashboard.ts,
// owner/alerts.ts, owner/signals.ts) is ALREADY CORRECT without this index — the index is a
// PERFORMANCE optimization (lets the planner serve the tie-broken keyset with one index scan
// instead of leaning on the 2-column index + a residual id filter/sort). Safe to apply later.
// ---------------------------------------------------------------------------
import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * A4 (ADR-0010, B3/B13/B14) — composite keyset indexes backing the strict `(sort, id)`
 * cursor pagination fix in owner/dashboard.ts (orders), owner/alerts.ts (location_alerts)
 * and owner/signals.ts (customer_signals).
 *
 * The routes now page by `(created_at, id)` / `(raised_at, id)` so a tie on the sort column
 * (same-millisecond burst orders/alerts/signals) is broken by id and never silently dropped
 * between pages. These indexes let the planner satisfy
 *   WHERE location_id=$1 AND (sort,id) < ($a,$b) ORDER BY sort DESC, id DESC
 * with a single index scan (the pre-existing 2-column indexes stop short of `id`).
 *
 * SCOPE: read-path only. ZERO table-schema change, ZERO RLS-policy change, ZERO money/PII column.
 * The old 2-column indexes are LEFT IN PLACE (strictly additive; redundancy is harmless).
 *
 * B14: `CREATE INDEX CONCURRENTLY` cannot run inside node-pg-migrate's default transaction, so
 * this migration is `pgm.noTransaction()`. `IF NOT EXISTS` makes it re-runnable; an interrupted
 * CONCURRENTLY build can leave an INVALID index — recover with `DROP INDEX CONCURRENTLY <name>`
 * then re-run (or `REINDEX INDEX CONCURRENTLY <name>`).
 */

const INDEXES: { name: string; table: string; cols: string }[] = [
  { name: 'orders_location_created_id_idx', table: 'orders', cols: 'location_id, created_at DESC, id DESC' },
  { name: 'location_alerts_location_created_id_idx', table: 'location_alerts', cols: 'location_id, created_at DESC, id DESC' },
  { name: 'customer_signals_location_raised_id_idx', table: 'customer_signals', cols: 'location_id, raised_at DESC, id DESC' },
];

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.noTransaction();
  for (const ix of INDEXES) {
    pgm.sql(`CREATE INDEX CONCURRENTLY IF NOT EXISTS ${ix.name} ON ${ix.table}(${ix.cols});`);
  }
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.noTransaction();
  for (const ix of INDEXES) {
    pgm.sql(`DROP INDEX CONCURRENTLY IF EXISTS ${ix.name};`);
  }
}
