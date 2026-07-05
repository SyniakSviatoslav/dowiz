// ⚠️ APPROVAL-PENDING SECURITY MIGRATION ARTIFACT (DB owner) — packages/db/ is protect-paths-blocked.
// MIG-ITEM2 of the pg-privilege-hardening change. See docs/design/pg-privilege-hardening/proposal.md.
// ---------------------------------------------------------------------------
// Fixes: the operational hot-path role `deliveryos_api_user` carries BYPASSRLS (granted in
// `1780691681296_ops-location-alerts-policy.ts`). BYPASSRLS makes `FORCE ROW LEVEL SECURITY` a no-op
// on the pool that serves every API request — tenant isolation then rests SOLELY on app-level WHERE
// clauses, so a single missing `SET app.user_id` leaks cross-tenant rows. This is the exact reason
// `verify:rls` cannot pass today (the cleaning-loop "verify:rls fails" finding was read as a BYPASSRLS
// env artifact — it is the real bug).
//
// FIX (Option 2A — reuse the already-granted role, smallest blast radius): strip the attribute with
// `ALTER ROLE deliveryos_api_user NOBYPASSRLS`. NO grant changes — the role keeps the DML grants it has
// accumulated across migrations, so no write path regresses by construction. The exact reverse of the
// statement in `1780691681296`. Enforcement flips ON the instant this runs: any flow that forgot to
// set the tenant GUC, or that relied on seeing cross-tenant rows, will start failing — that is the
// intended exposure of latent isolation bugs, which is why it is STAGING-FIRST behind `verify:rls`.
//
// APPLY ORDER: this (MIG-ITEM2) runs BEFORE MIG-ITEM1 — until the hot-path role enforces RLS,
// `verify:rls` cannot pass and ITEM 1's gate is unobservable (proposal §4).
// APPLY: move to packages/db/migrations/<next>_operational-role-nobypassrls.ts (number assigned at
// placement — before the secdef-search-path file), run on STAGING DB first, then `pnpm verify:rls`
// + the full lifecycle E2E under the NOBYPASSRLS role. Any newly-failing query is a real isolation
// bug to FIX before prod — never re-grant BYPASSRLS as a "fix".
//
// VERIFY (must return 0 rows after):
//   SELECT rolname FROM pg_roles WHERE rolname='deliveryos_api_user' AND rolbypassrls;
//
// Idempotent: NOBYPASSRLS of an already-NOBYPASSRLS role is a no-op; the exception-swallowing block
// (mirrors `1780691681296`) makes it a no-op where the role is absent (e.g. a fresh local DB).
// ---------------------------------------------------------------------------
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // RETARGETED 2026-06-30: the LIVE operational role is `dowiz_app` (staging/prod DATABASE_URL_OPERATIONAL);
  // `deliveryos_api_user` is legacy/nologin. Flip BOTH (idempotent, exception-swallowed per role) so the
  // migration is correct on any environment. The GUC-coverage remediation (Phase 1 mig 077 policies + Phase 2
  // migs 078/079 worker DEFINER fns) MUST already be applied + the code deployed-dark before this runs.
  pgm.sql(`
    DO
    $$
    BEGIN
      BEGIN EXECUTE 'ALTER ROLE dowiz_app NOBYPASSRLS'; EXCEPTION WHEN OTHERS THEN END;
      BEGIN EXECUTE 'ALTER ROLE deliveryos_api_user NOBYPASSRLS'; EXCEPTION WHEN OTHERS THEN END;
    END
    $$;
  `);
}

export async function down(): Promise<void> {
  // No-op: re-granting BYPASSRLS re-introduces the RLS-bypass on the hot path. The operational role
  // must never bypass RLS; intentionally not reversed. If an emergency reversal is ever forced it is a
  // manual, reviewed `ALTER ROLE deliveryos_api_user BYPASSRLS`, never an automatic migration down().
}
