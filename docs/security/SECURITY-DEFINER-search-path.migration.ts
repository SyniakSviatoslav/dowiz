// ⚠️ APPROVAL-PENDING SECURITY MIGRATION ARTIFACT (DB owner) — packages/db/ is protect-paths-blocked.
// MIG-ITEM1 of the pg-privilege-hardening change. See docs/design/pg-privilege-hardening/proposal.md.
// ---------------------------------------------------------------------------
// Fixes: 4 distinct SECURITY DEFINER Postgres functions run WITHOUT a pinned `search_path`
// (13 historical CREATE occurrences — `read_public_menu` is re-CREATEd across many migrations).
// Flagged by the security audit + the `cleaning-loop` memo. A SECURITY DEFINER function with a
// mutable search_path is a privilege-escalation / RLS-bypass vector: a caller who can create an
// object in an earlier-resolved schema (notably an *implicit* `pg_temp`, which is searched FIRST
// for relation/type names when it is not explicitly listed) can shadow a table the definer touches.
// Affected (the 4 live fns): the RLS lynchpin `app_member_location_ids()`, `read_public_menu()`,
// `read_public_menu_all_locales()`, `upsert_menu_version()`, `bump_menu_version_trigger_fn()`.
//
// APPLY: move to packages/db/migrations/<next>_secdef-search-path.ts (number assigned at placement —
// current head is 1790000000073, so e.g. …074/075), run on STAGING DB first, then verify (query
// below), then prod via release_command.
//
// Idempotent + signature-free: a DO block ALTERs every SECURITY DEFINER function in `public`
// that has no `search_path` in proconfig, resolving each signature via regprocedure. Safe to
// re-run. The pinned value is `pg_catalog, public, pg_temp` — CORRECTED from the earlier
// `pg_catalog, public`: explicitly listing `pg_temp` LAST demotes it out of its default
// first-position so a temp object can never shadow `public.<relation>`, while `pg_catalog` first
// prevents built-in shadowing. No body changes, grants preserved across ALTER FUNCTION, no-op down.
// (Also re-pins the staged P6/deliver-v2 fns that use the weaker `= public` — see proposal §10 R5.)
//
// VERIFY (should return 0 rows after):
//   SELECT p.oid::regprocedure FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
//   WHERE n.nspname='public' AND p.prosecdef
//     AND NOT EXISTS (SELECT 1 FROM unnest(coalesce(p.proconfig,'{}')) c WHERE c LIKE 'search_path=%');
//
// FOLLOW-ON (in the operator-handoff): add that same query as an exit(1) check to verify:rls so a
// future SECURITY DEFINER function without search_path fails the gate (see B12 pattern). The static
// counterpart already ships: scripts/guardrail-definer-search-path.mjs (CI/pre-commit, no DB needed).
// ---------------------------------------------------------------------------
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DO $$
    DECLARE r record;
    BEGIN
      FOR r IN
        SELECT p.oid::regprocedure AS sig
        FROM pg_proc p
        JOIN pg_namespace n ON n.oid = p.pronamespace
        WHERE n.nspname = 'public'
          AND p.prosecdef  -- SECURITY DEFINER
          -- Re-pin a fn whose path is MISSING or is one of the two known-WEAK staged values
          -- ('public' / 'pg_catalog, public' -- see proposal §6/R5). Conservative on purpose: a fn
          -- already at the strong triple, or one that explicitly lists another schema (e.g.
          -- extensions), is left untouched so this never breaks a fn that needs a wider path.
          AND NOT EXISTS (
            SELECT 1 FROM unnest(coalesce(p.proconfig, '{}'::text[])) c
            WHERE c LIKE 'search_path=%'
              AND c NOT IN ('search_path=public', 'search_path=pg_catalog, public')
          )
      LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = pg_catalog, public, pg_temp', r.sig);
        RAISE NOTICE 'pinned search_path on %', r.sig;
      END LOOP;
    END $$;
  `);
}

export async function down(): Promise<void> {
  // No-op: removing the search_path pin would re-introduce the vulnerability. A SECURITY DEFINER
  // function should never run without a pinned search_path; intentionally not reversed.
}
