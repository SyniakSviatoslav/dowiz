// STAGED MIGRATION ARTIFACT — operator places this at
//   packages/db/migrations/1790000000068_acquisition.ts
// (the migrations dir is a protected governance zone; this is the manual-approval handoff,
//  mirroring docs/security/SECURITY-DEFINER-search-path.migration.ts). Its SQL + the dedup
//  invariant are proven against a throwaway Postgres in P6-1 (see proof in the commit), but
//  `migrate:up` must run on the dev-Postgres to land it. P6-1 · next free number verified 068.
import type { MigrationBuilder } from 'node-pg-migrate';

// P6-1 — acquisition pipeline: provenance on products + the acquisition_sources
// state-machine table. One Google place_id → at most one shadow-tenant lifecycle row.
//
// `acquisition_sources` is NON-TENANT (one row per place_id, not per location) and
// PII-/competitor-bearing (place_raw). RLS posture mirrors access_requests (1790000000041):
// ENABLE+FORCE RLS + a single FOR ALL USING(true) ops policy — RLS here is a linter /
// anti-BYPASSRLS guard, NOT row isolation; the real boundary is the GRANT layer
// (anon/authenticated/service_role hard-revoked; ops role mirrors orders' DML). The
// internal route is ops-only; no tenant route reads this table.
//
// provenance on products (the claim + liability carrier): `source` records WHERE each
// row came from; `allergens_confirmed` stays false until an authenticated owner confirms
// post-claim (operator decision 2 — the pipeline NEVER asserts allergens as fact).
//
// Forward-only intent; down() drops the new objects (no PII rides a shadow row pre-claim;
// place_raw is minimized + hard-deletable via the service, not the migration).
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    -- provenance enum + columns on the existing products table (backward-compatible defaults)
    CREATE TYPE product_source AS ENUM ('owner', 'imported', 'ai_inferred', 'place');
    ALTER TABLE products
      ADD COLUMN source              product_source NOT NULL DEFAULT 'owner',
      ADD COLUMN allergens_confirmed boolean        NOT NULL DEFAULT false;

    -- acquisition lifecycle states (every non-terminal state has a forward edge AND an
    -- exit to MANUAL_REVIEW/terminal — enforced in the state-machine, not just here)
    CREATE TYPE acquisition_state AS ENUM (
      'SOURCED', 'PLACE_INGESTED', 'MENU_EXTRACTED', 'ENRICHED',
      'PROVISIONED', 'VERIFIED', 'CLAIM_OFFERED', 'CLAIMED',
      'MENU_NOT_FOUND', 'LOW_QUALITY', 'MANUAL_REVIEW', 'DISQUALIFIED', 'ABANDONED'
    );

    CREATE TABLE acquisition_sources (
      id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      place_id       text NOT NULL UNIQUE,              -- canonical identity → dedup anchor
      state          acquisition_state NOT NULL DEFAULT 'SOURCED',
      place_raw      jsonb,                             -- minimized Places payload (audit/replay); hard-deletable
      website_url    text,
      menu_kind      text,                              -- html | pdf_text | pdf_image | image | none
      menu_draft     jsonb,                             -- structured+enriched menu BEFORE tenant write (transient)
      confidence     int CHECK (confidence BETWEEN 0 AND 100),
      org_id         uuid REFERENCES organizations(id),
      location_id    uuid REFERENCES locations(id),
      failure_reason text,                              -- REQUIRED on any terminal/MANUAL_REVIEW (enforced in code)
      claimed_at     timestamptz,
      created_at     timestamptz NOT NULL DEFAULT now(),
      updated_at     timestamptz NOT NULL DEFAULT now()
    );
    CREATE INDEX idx_acquisition_sources_state ON acquisition_sources (state);

    -- Non-tenant, PII/competitor-bearing: ENABLE + FORCE RLS, single ops policy (Pattern A2,
    -- mirror access_requests). NOT row isolation — the GRANT layer below is the boundary.
    ALTER TABLE acquisition_sources ENABLE ROW LEVEL SECURITY;
    ALTER TABLE acquisition_sources FORCE  ROW LEVEL SECURITY;
    CREATE POLICY allow_ops_acquisition_sources_all ON acquisition_sources
      FOR ALL USING (true) WITH CHECK (true);

    -- Remove from the Supabase Data API perimeter (mirror the non-tenant identity tables).
    REVOKE ALL PRIVILEGES ON TABLE acquisition_sources FROM anon, authenticated, service_role;
  `);

  // Mirror the DML grants `orders` already holds, so the same operational role that writes
  // orders can write acquisition_sources — regardless of its deployed name (the 026/041 lesson).
  pgm.sql(`
    DO $$
    DECLARE r record;
    BEGIN
      FOR r IN
        SELECT DISTINCT grantee, privilege_type
        FROM information_schema.role_table_grants
        WHERE table_schema = 'public'
          AND table_name = 'orders'
          AND privilege_type IN ('SELECT', 'INSERT', 'UPDATE', 'DELETE')
          AND grantee <> 'PUBLIC'
      LOOP
        EXECUTE format(
          'GRANT %s ON public.acquisition_sources TO %I',
          r.privilege_type, r.grantee
        );
      END LOOP;
    END
    $$;
  `);

  // Forward-compat: the aspirational operational role (migration 015) gets explicit DML.
  pgm.sql(`
    DO $$
    BEGIN
      IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'deliveryos_operational_user') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON public.acquisition_sources
          TO deliveryos_operational_user;
      END IF;
    END
    $$;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE IF EXISTS acquisition_sources;
    DROP TYPE  IF EXISTS acquisition_state;
    ALTER TABLE products
      DROP COLUMN IF EXISTS allergens_confirmed,
      DROP COLUMN IF EXISTS source;
    DROP TYPE  IF EXISTS product_source;
  `);
}
