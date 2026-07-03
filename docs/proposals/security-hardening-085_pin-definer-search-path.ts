// ─────────────────────────────────────────────────────────────────────────────
// STAGED MIGRATION ARTIFACT — DO NOT EDIT packages/db/migrations/ DIRECTLY.
//
// OPERATOR APPLY STEPS:
//   1. Copy this file to packages/db/migrations/ with the NEXT free sequence number,
//      i.e. `1790000000085_pin-definer-search-path.ts` (current tail = ...084; the
//      migrations dir is a protected governance zone — this handoff mirrors 072/077).
//   2. Run on the STAGING DB first:
//        flyctl proxy 5433:5432 -a dowiz-staging-db   # then, against the proxied DB:
//        pnpm migrate:up                              # (DATABASE_URL_MIGRATIONS → localhost:5433)
//      then prove the pin (see PROOF GATE below).
//   3. PROD only on merge to `main` (CI `migrate:up` step runs it before the Fly deploy).
//
// WHAT / WHY (security-hardening batch 2026-07-02, finding #3 — resolution.md §"#3",
//   proposal.md §4/#3 and §5):
//   `app_member_location_ids()` is the KEYSTONE SECURITY DEFINER helper behind ~40 member RLS
//   policies (`… location_id IN (SELECT app_member_location_ids())`). It was created in
//   1780310071220_core-identity.ts WITHOUT a pinned search_path. A SECURITY DEFINER function
//   with a mutable search_path is an RLS-bypass / privilege-escalation vector: a caller who can
//   create an object in an earlier-resolved schema (notably an implicit `pg_temp`) can shadow a
//   relation/function the definer resolves — through the exact predicate meant to ENFORCE tenant
//   isolation. Pinning to `pg_catalog, public, pg_temp` (pg_temp last) closes the spoof.
//
//   This is a PURE METADATA change (`ALTER FUNCTION … SET search_path`): no body/logic change,
//   no table rewrite, behavior-neutral for every legitimate caller (they already resolve `public`).
//   Safe under BOTH pool cases (BYPASSRLS today, NOBYPASSRLS post-B3-flip) and independent of the
//   flip — ship in Tier 1, do NOT defer the keystone to Phase-3.
//
//   Also pins the M6 menu-read SECURITY DEFINER helpers if still unpinned (resolution.md §"#3":
//   "re-pinning the keystone AND the M6 menu-read definers if still unpinned"). Per
//   scripts/definer-baseline.json these are `public.read_public_menu(...)` and
//   `public.read_public_menu_all_locales(text)` — both HOT-PATH storefront readers. They are pinned
//   robustly by a catalog-driven loop (below) so exact signature/overload drift cannot break the
//   migration; only unpinned SECURITY DEFINER functions of those names are touched.
//
// PROOF GATE (prove on staging BEFORE prod — this is the "runtime pin" the CI static gate
//   cannot prove in a DB-less job; resolution.md §"#3"):
//   SELECT proname, proconfig FROM pg_proc
//    WHERE proname IN ('app_member_location_ids','read_public_menu','read_public_menu_all_locales')
//   → every row's proconfig must contain 'search_path=pg_catalog, public, pg_temp'.
//   The static regression net (scripts/guardrail-definer-search-path.mjs, already ci:true) keeps a
//   NEW unpinned definer from landing; this migration + the boot-guard/verify:rls probe prove the
//   LIVE pin. Do NOT claim CI proves the live pin.
//
// down(): real reversal — RESET search_path on the same functions, restoring the historical
//   (unpinned) state. Forward-only in INTENT; down provided for local/rollback parity.
// ─────────────────────────────────────────────────────────────────────────────
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // 1. KEYSTONE — the required pin (resolution.md / proposal.md §5). Stable zero-arg signature.
  pgm.sql(`
    ALTER FUNCTION public.app_member_location_ids() SET search_path = pg_catalog, public, pg_temp;
  `);

  // 2. M6 menu-read definers — pin every UNPINNED SECURITY DEFINER function of these names,
  //    regardless of exact signature/overload (robust against re-versioning across 018/032/033/
  //    055/063/064/065/072). No-op if a name is absent or already pinned.
  pgm.sql(`
    DO $$
    DECLARE
      r record;
    BEGIN
      FOR r IN
        SELECT p.oid::regprocedure AS sig
        FROM pg_proc p
        JOIN pg_namespace n ON n.oid = p.pronamespace
        WHERE n.nspname = 'public'
          AND p.proname IN ('read_public_menu', 'read_public_menu_all_locales')
          AND p.prosecdef = true                                   -- SECURITY DEFINER only
          AND NOT EXISTS (                                         -- skip already-pinned
            SELECT 1 FROM unnest(coalesce(p.proconfig, '{}'::text[])) c
            WHERE c LIKE 'search_path=%'
          )
      LOOP
        EXECUTE format(
          'ALTER FUNCTION %s SET search_path = pg_catalog, public, pg_temp', r.sig
        );
      END LOOP;
    END $$;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  // Reverse the pin — restore the historical unpinned state (RESET removes the per-function
  // proconfig search_path entry). Mirrors up() exactly.
  pgm.sql(`
    ALTER FUNCTION public.app_member_location_ids() RESET search_path;
  `);
  pgm.sql(`
    DO $$
    DECLARE
      r record;
    BEGIN
      FOR r IN
        SELECT p.oid::regprocedure AS sig
        FROM pg_proc p
        JOIN pg_namespace n ON n.oid = p.pronamespace
        WHERE n.nspname = 'public'
          AND p.proname IN ('read_public_menu', 'read_public_menu_all_locales')
          AND p.prosecdef = true
          AND EXISTS (
            SELECT 1 FROM unnest(coalesce(p.proconfig, '{}'::text[])) c
            WHERE c LIKE 'search_path=%'
          )
      LOOP
        EXECUTE format('ALTER FUNCTION %s RESET search_path', r.sig);
      END LOOP;
    END $$;
  `);
}
