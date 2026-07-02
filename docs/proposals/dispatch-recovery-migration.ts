// ═══════════════════════════════════════════════════════════════════════════════════════════
// PROPOSED MIGRATION — NOT ACTIVE. packages/db/migrations/ is a red-line protected path, so
// this file holds the exact migration CONTENT for OPERATOR placement (ADR-dispatch-recovery §5,
// the one load-bearing migration of the design).
//
// Operator steps:
//   1. Copy this file to packages/db/migrations/<next-timestamp>_dispatch-exhausted-marker.ts
//      (next after 1790000000084_location-themes-fonts.ts, e.g. 1790000000085_dispatch-exhausted-marker.ts).
//   2. Run it on the STAGING DB FIRST (flyctl proxy → node-pg-migrate), then deploy —
//      ⚠️ ORDER MATTERS: the CourierDispatchWorker exhaustion path writes this column; deploying
//      the dispatch-recovery code before the column exists makes every exhaustion tick fail
//      (42703 → rollback → journal row survives → hourly churn until the column lands).
//   3. Prod on explicit approval, same order (migrate, then deploy).
//
// Design notes (docs/adr/ADR-dispatch-recovery.md, resolution.md §C):
//   - Additive, forward-only, nullable, NO default → metadata-only ALTER, no table rewrite.
//   - orders.dispatch_exhausted_at is the durable held / needs-attention marker set in the
//     dispatch-exhaustion transaction; order_status stays truthful (no enum ripple on the
//     red-line state machine). Read by the owner dashboard, Recon O1 triage, and the
//     flag-gated grace-window pass (DISPATCH_OWNER_GRACE_ENABLED, default false).
//   - orders is already tenant-scoped + FORCE RLS; an additive nullable column needs no
//     policy change. Integer-money untouched. No index at pilot (R-DEFER-1: add a covering
//     index only at ~10× volume).
//   - Rollback posture: the column is inert if unwritten — safe to leave on code revert.
// ═══════════════════════════════════════════════════════════════════════════════════════════
import type { MigrationBuilder } from 'node-pg-migrate';

// ADR-dispatch-recovery (B2 honest exhaustion tail, ETHICAL-STOP-1): the durable owner-visible
// trace of a dispatch-exhausted order. Set (now()) by CourierDispatchWorker when the journal
// escalates at COURIER_DISPATCH_MAX_ATTEMPTS, in the SAME transaction that deletes the journal
// row — the trace is committed before the ORDER_DISPATCH_FAILED event fires, so it can never
// be erased into the void.
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders
      ADD COLUMN IF NOT EXISTS dispatch_exhausted_at timestamptz;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE orders
      DROP COLUMN IF EXISTS dispatch_exhausted_at;
  `);
}
