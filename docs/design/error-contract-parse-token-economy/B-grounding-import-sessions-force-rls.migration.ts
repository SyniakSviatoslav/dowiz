// ⚠️ APPROVAL-PENDING MIGRATION ARTIFACT (B5/B12 / ADR-0011, DB owner)
// ---------------------------------------------------------------------------
// `packages/db/migrations/` is a protected red-line zone. To apply:
//   1. Move to packages/db/migrations/1790000000069_import-sessions-force-rls.ts
//      (bump the timestamp if a higher migration landed since).
//   2. Run on STAGING DB FIRST (ship-discipline §2):
//        flyctl proxy 5433:5432 -a dowiz-staging-db
//        DATABASE_URL_MIGRATIONS=postgres://…@localhost:5433/… pnpm --filter @deliveryos/db migrate up
//   3. Apply the staged verify:rls strengthening (B12-verify-rls-force-gate.md) and confirm
//      `pnpm verify:rls` is GREEN. Only then may MENU_GROUNDING_ENABLED be turned on (B2-grounding).
//
// This is the LAST gate on B2-grounding. import_sessions is RLS ENABLE-only
// (1780338982025:26) and absent from the FORCE migration (1780421100051). Without FORCE, the
// table-OWNER role bypasses its tenant policy — the grounding draft data (which lands in
// import_sessions) could be read cross-tenant by an owner-role query. FORCE closes that.
// Forward-only, no table/column/policy change — just the FORCE bit. Reversible via down().
// ---------------------------------------------------------------------------
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Not CONCURRENTLY / not noTransaction — a catalog flag flip, instantaneous, txn-safe.
  pgm.sql('ALTER TABLE import_sessions FORCE ROW LEVEL SECURITY;');
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql('ALTER TABLE import_sessions NO FORCE ROW LEVEL SECURITY;');
}
