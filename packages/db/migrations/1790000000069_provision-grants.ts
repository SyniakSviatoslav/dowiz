// STAGED MIGRATION ARTIFACT — operator places this at
//   packages/db/migrations/1790000000069_provision-grants.ts
// (the migrations dir is a protected governance zone; this is the manual-approval handoff,
//  mirroring docs/acquisition/migration-1790000000068-acquisition.ts). Its SQL + the
//  provisioning-policy invariants are proven against a throwaway Postgres in P6-2 (see the
//  provision-rls.test.ts proof in the commit). `migrate:up` must run on the dev-Postgres to
//  land it. REQUIRES 068 (acquisition) applied first. P6-2 · next free number verified 069.
import type { MigrationBuilder } from 'node-pg-migrate';

// P6-2 — shadow-spine write authority via a SINGLE-USE provisioning token (operator decision 1b).
// The shadow spine (organizations owner_id NULL + locations status='closed' + menu_versions v1) is
// written THROUGH RLS, not around it: a narrow ADDITIVE provisioning policy admits the write ONLY
// while a valid, unconsumed, unexpired one-time token is set as a txn-local GUC.
//
// 🔴 HONEST CRUX (do not let this overclaim): the operational role BYPASSES RLS today (verify:rls
// fails; mig-015's restricted role is aspirational — see 1790000000041 comment; db/index.ts only
// blocks the literal 'postgres' superuser). So these provision_shadow policies enforce NOTHING
// today — they are defense-in-depth that BECOMES the live boundary the moment the role is locked
// to NOBYPASSRLS. Today's live boundary is (i) the app code (always owner_id NULL / status closed)
// and (ii) the GRANT layer. The provision-rls.test.ts proof runs under an explicit NOBYPASSRLS
// test role precisely because the live role would mask the policy.
//
// Token hashing uses the BUILT-IN pg_catalog sha256(bytea) (NOT pgcrypto digest) so there is zero
// search_path dependency in a hot-path WITH CHECK (architect condition C-arch-3). encode()/sha256()
// are both pg_catalog built-ins. The token is hex(32 random bytes) → ASCII, so Node's
// crypto.createHash('sha256').update(token).digest('hex') and pg sha256(token::bytea) agree byte-for-byte.
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE provision_grants (
      id                    uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      acquisition_source_id uuid NOT NULL REFERENCES acquisition_sources(id) ON DELETE CASCADE,
      token_hash            text NOT NULL UNIQUE,          -- encode(sha256(token::bytea),'hex'); plaintext NEVER stored
      expires_at            timestamptz NOT NULL,          -- short TTL (~5 min, set in the minting service)
      consumed_at           timestamptz,                   -- single-use: set once, in the SAME tx as the spine write
      created_at            timestamptz NOT NULL DEFAULT now()
    );

    -- Breaker H1 (multi-grant → double-spine) defense-in-depth at MINT: at most one ACTIVE
    -- (unconsumed) grant per acquisition source. The load-bearing dedup anchor is still the
    -- in-tx guarded state-transition (advance ENRICHED→PROVISIONED WHERE state='ENRICHED'); this
    -- partial-unique stops a second token from ever being minted while one is outstanding.
    CREATE UNIQUE INDEX provision_grants_one_active_per_source
      ON provision_grants (acquisition_source_id) WHERE consumed_at IS NULL;

    -- Reaper predicate (Q6 / breaker L1): a concrete sweep deletes expired-unconsumed grants.
    CREATE INDEX provision_grants_expires_idx
      ON provision_grants (expires_at) WHERE consumed_at IS NULL;

    -- Non-tenant, secret-bearing: ENABLE + FORCE RLS + single ops policy (access_requests Pattern A2).
    -- The provision_shadow policy subquery reads this table; the ops USING(true) + the grant-mirror
    -- below give the operational/provisioning role SELECT, with no recursion (it never references
    -- organizations/locations).
    ALTER TABLE provision_grants ENABLE ROW LEVEL SECURITY;
    ALTER TABLE provision_grants FORCE  ROW LEVEL SECURITY;
    CREATE POLICY allow_ops_provision_grants_all ON provision_grants
      FOR ALL USING (true) WITH CHECK (true);
    REVOKE ALL PRIVILEGES ON TABLE provision_grants FROM anon, authenticated, service_role;
  `);

  // Additive PERMISSIVE provisioning policies. PG ORs permissive policies: a normal tenant write
  // never sets app.provision_token (current_setting → NULL → sha256(NULL) → NULL → no grant match)
  // and never sets owner_id NULL, so this policy contributes nothing to the tenant OR-set (it cannot
  // WIDEN tenant writes — breaker Q2 refuted). FOR INSERT only → no read surface added (organizations
  // has no SELECT policy, so the service pre-generates UUIDs and never RETURNINGs — architect C-arch-1).
  const tokenValid = `EXISTS (
    SELECT 1 FROM provision_grants g
    WHERE g.token_hash = encode(sha256(current_setting('app.provision_token', true)::bytea), 'hex')
      AND g.consumed_at IS NULL
      AND g.expires_at > now()
  )`;
  pgm.sql(`
    CREATE POLICY provision_shadow ON organizations FOR INSERT
      WITH CHECK ( owner_id IS NULL AND ${tokenValid} );

    CREATE POLICY provision_shadow ON locations FOR INSERT
      WITH CHECK ( status = 'closed' AND ${tokenValid} );

    -- menu_versions has no owner_id/status discriminator → token-only (architect C-arch-2).
    CREATE POLICY provision_shadow ON menu_versions FOR INSERT
      WITH CHECK ( ${tokenValid} );
  `);

  // Mirror the DML grants `orders` already holds, so the same operational role that writes orders
  // can mint/consume grants — regardless of its deployed name (the 026/041 lesson).
  pgm.sql(`
    DO $$
    DECLARE r record;
    BEGIN
      FOR r IN
        SELECT DISTINCT grantee, privilege_type
        FROM information_schema.role_table_grants
        WHERE table_schema = 'public' AND table_name = 'orders'
          AND privilege_type IN ('SELECT', 'INSERT', 'UPDATE', 'DELETE')
          AND grantee <> 'PUBLIC'
      LOOP
        EXECUTE format('GRANT %s ON public.provision_grants TO %I', r.privilege_type, r.grantee);
      END LOOP;
    END
    $$;
  `);

  // Forward-compat: the aspirational operational role (migration 015) gets explicit DML.
  pgm.sql(`
    DO $$
    BEGIN
      IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'deliveryos_operational_user') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON public.provision_grants TO deliveryos_operational_user;
      END IF;
    END
    $$;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP POLICY IF EXISTS provision_shadow ON menu_versions;
    DROP POLICY IF EXISTS provision_shadow ON locations;
    DROP POLICY IF EXISTS provision_shadow ON organizations;
    DROP TABLE IF EXISTS provision_grants CASCADE;
  `);
}
