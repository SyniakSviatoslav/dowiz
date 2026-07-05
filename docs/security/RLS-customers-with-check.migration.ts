// ⚠️ APPROVAL-PENDING SECURITY MIGRATION ARTIFACT (DB owner) — packages/db/ is protect-paths-blocked.
// Phase-0 ship-blocker (MVP plan). Fixes a RLS asymmetry flagged by the CLEAN seed catalog.
// ---------------------------------------------------------------------------
// The `anonymous_update` policy on `customers` (migration 1780338981782) has a USING clause but NO
// WITH CHECK:
//     CREATE POLICY anonymous_update ON customers FOR UPDATE USING (app_current_user() IS NULL);
// USING gates which rows an anonymous session may target; WITH CHECK gates what the row may BECOME.
// Without it, the anonymous customer self-upsert path (ON CONFLICT DO UPDATE) can write a post-image
// the policy would otherwise reject — the classic "RLS UPDATE without WITH CHECK" hole (2A-DB seed).
//
// Fix: re-create the policy with a symmetric WITH CHECK (same predicate as USING) so an anon writer
// can only produce rows that still satisfy the anon condition. Behavior-identical for the legitimate
// anon upsert; closes the asymmetry. Additive/idempotent (DROP IF EXISTS + CREATE).
//
// APPLY: place in packages/db/migrations/<next>_customers-anon-update-with-check.ts, run staging-first,
// then `pnpm verify:rls`. Add a guardrail: lint every `FOR UPDATE` policy to require WITH CHECK.
// ---------------------------------------------------------------------------
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP POLICY IF EXISTS anonymous_update ON customers;
    CREATE POLICY anonymous_update ON customers FOR UPDATE
      USING (app_current_user() IS NULL)
      WITH CHECK (app_current_user() IS NULL);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  // Reverting drops the WITH CHECK (re-opens the asymmetry) — restore the prior USING-only form.
  pgm.sql(`
    DROP POLICY IF EXISTS anonymous_update ON customers;
    CREATE POLICY anonymous_update ON customers FOR UPDATE
      USING (app_current_user() IS NULL);
  `);
}
