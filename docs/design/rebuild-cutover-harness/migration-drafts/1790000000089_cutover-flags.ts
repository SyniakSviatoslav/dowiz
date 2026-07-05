import type { MigrationBuilder } from 'node-pg-migrate';

// ============================================================================
// OPERATOR-GATED DRAFT — NOT APPLIED. Do NOT place into packages/db/migrations/
// without the standing red-line review (packages/db/migrations/** is a 🔴 protect-path).
// ============================================================================
//
// ADR-0022 §2 — the `cutover_flags` runtime flip store for the reversible per-surface
// cutover harness (council RESOLVE: docs/design/rebuild-cutover-harness/resolution.md).
// One row per strangler surface; a flip is one UPDATE; rollback is the inverse UPDATE —
// instant, no redeploy. Consumed by apps/api/src/lib/cutover/flags.ts via bounded-TTL
// poll (REV-C3 fallback — LISTEN/NOTIFY is blocked on the transaction pooler).
//
// AUTHORITY MODEL (matches the platform_admins grants-alone + DEFINER-ingress house
// pattern, migrations 080/088):
//   - dowiz_app: SELECT only. NO direct INSERT/UPDATE/DELETE — an app-side compromise
//     (SQLi, deserialization) must never be able to flip a money surface to an
//     unproven stack.
//   - Flips (node→rust, readiness_ok) are OPERATOR acts on a privileged connection
//     (DATABASE_URL_MIGRATIONS role / table owner). Each flip UPDATE must set
//     updated_by to the operator's sign-off token (ADR-0022 §3).
//   - The ONE constrained app-side write path is cutover_auto_degrade(): SECURITY
//     DEFINER, pinned search_path (ledger #33 discipline), can ONLY move target toward
//     'node' (the incumbent stack — the direction that is always safe), and REFUSES the
//     money/irreversible surfaces S5/S7/S9 (REV-C5: human go/no-go only).
//   - RLS: ENABLE (not FORCE) + a read-only policy. Not FORCE because the table owner
//     executes both the operator flips and the DEFINER degrade; FORCE + zero write
//     policies would brick both. Writes for non-owners are impossible anyway: zero
//     write grants + zero write policies (belt and braces).
//
// SEED: all 10 surfaces at target='node', readiness_ok=false — the harness deploys
// dark and every flip starts refused until a surface's cutover DoD is recorded green.
//
// ROLLBACK (staging rehearsal): DROP FUNCTION cutover_auto_degrade(text, text);
// DROP TABLE cutover_flags; — no other object references them.

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE IF NOT EXISTS cutover_flags (
      surface      text PRIMARY KEY
                   CHECK (surface IN ('S1','S2','S3','S4','S5','S6','S7','S8','S9','S10')),
      target       text NOT NULL DEFAULT 'node' CHECK (target IN ('node','rust')),
      readiness_ok boolean NOT NULL DEFAULT false,
      updated_at   timestamptz NOT NULL DEFAULT now(),
      updated_by   text NOT NULL DEFAULT 'migration-seed'
    );

    INSERT INTO cutover_flags (surface)
    VALUES ('S1'),('S2'),('S3'),('S4'),('S5'),('S6'),('S7'),('S8'),('S9'),('S10')
    ON CONFLICT (surface) DO NOTHING;

    ALTER TABLE cutover_flags ENABLE ROW LEVEL SECURITY;

    DROP POLICY IF EXISTS cutover_flags_read ON cutover_flags;
    CREATE POLICY cutover_flags_read ON cutover_flags FOR SELECT USING (true);

    -- Grants-alone write control (platform_admins pattern, mig 080): the app role can
    -- read the flags and execute the constrained degrade fn — nothing else.
    DO $do$ BEGIN
      EXECUTE 'REVOKE INSERT, UPDATE, DELETE, TRUNCATE ON cutover_flags FROM dowiz_app';
      EXECUTE 'GRANT SELECT ON cutover_flags TO dowiz_app';
    EXCEPTION WHEN undefined_object THEN
      -- role absent on local/dev DBs — grants are a no-op there
    END $do$;

    -- REV-C5: the machine's only write path, direction-constrained toward Node and
    -- closed for money/irreversible surfaces. Runs as the table owner (DEFINER).
    CREATE OR REPLACE FUNCTION cutover_auto_degrade(p_surface text, p_reason text)
      RETURNS boolean
      LANGUAGE plpgsql VOLATILE SECURITY DEFINER
      SET search_path = pg_catalog, public, pg_temp AS $fn$
    BEGIN
      IF p_surface IN ('S5','S7','S9') THEN
        RAISE EXCEPTION 'cutover_auto_degrade refused for money/irreversible surface % — human go/no-go only (REV-C5)', p_surface;
      END IF;
      UPDATE cutover_flags
         SET target = 'node',
             updated_at = now(),
             updated_by = 'auto-degrade: ' || left(coalesce(p_reason, 'unspecified'), 200)
       WHERE surface = p_surface AND target = 'rust';
      RETURN FOUND;
    END;
    $fn$;

    REVOKE ALL ON FUNCTION cutover_auto_degrade(text, text) FROM PUBLIC;
    DO $do$ BEGIN
      EXECUTE 'GRANT EXECUTE ON FUNCTION cutover_auto_degrade(text, text) TO dowiz_app';
    EXCEPTION WHEN undefined_object THEN
    END $do$;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP FUNCTION IF EXISTS cutover_auto_degrade(text, text);
    DROP TABLE IF EXISTS cutover_flags;
  `);
}
