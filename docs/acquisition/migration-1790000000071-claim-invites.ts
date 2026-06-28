// STAGED MIGRATION ARTIFACT — operator places this at
//   packages/db/migrations/1790000000071_claim-invites.ts
// (migrations dir is a protected governance zone; manual-approval handoff, mirroring 068-070).
// **REQUIRES 068 + 069 + 070 applied first.** P6 claim phase · next free number after 070 = 071.
import type { MigrationBuilder } from 'node-pg-migrate';

// P6 CLAIM PHASE — the ownership-transfer authority. A single-use, short-TTL, opaque-256-bit claim
// token (sha256-hashed, plaintext never stored) lets an authenticated owner transfer a SHADOW org
// (owner_id NULL) to themselves. Council verdict docs/design/p6-claim-council-verdict.md.
//
// DESIGN NOTE (deviation from the inline claim_accept RLS policy in the proposal, with rationale):
// an ownership transfer is an UPDATE, and PostgreSQL requires a row to be SELECT-visible to be
// UPDATE-targeted — a shadow org has no SELECT policy for the (NOBYPASSRLS) claim role, so an inline
// FOR-UPDATE/FOR-ALL policy cannot make the transfer work without ALSO widening shadow-org reads. The
// architect's C2 anticipated this fork: a NARROW, single-purpose SECURITY DEFINER function
// `claim_transfer(token, user)` that validates the token INSIDE and performs the whole transfer
// atomically is the clean, auditable carve-out (the token remains the SOLE authority; the fn only
// ever touches the token's target shadow). This is consistent with decision 1b's "explicit, auditable,
// single-use carve-out" — it is NOT a blanket bypass. Built-in sha256 → zero search_path dependency.

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE claim_invites (
      id                    uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      acquisition_source_id uuid NOT NULL REFERENCES acquisition_sources(id) ON DELETE CASCADE,
      token_hash            text NOT NULL UNIQUE,        -- encode(sha256(token::bytea),'hex'); plaintext NEVER stored
      invited_contact_hash  text,                        -- sha256 of the official contact the Art-14 notice went to (audit + future email-match)
      expires_at            timestamptz NOT NULL,        -- short TTL (hours — human round-trip)
      used_at               timestamptz,                 -- single-use
      used_by_user_id       uuid REFERENCES users(id),
      revoked_at            timestamptz,
      created_at            timestamptz NOT NULL DEFAULT now()
    );
    CREATE UNIQUE INDEX claim_invites_one_active_per_source
      ON claim_invites (acquisition_source_id) WHERE used_at IS NULL AND revoked_at IS NULL;
    CREATE INDEX claim_invites_expires_idx
      ON claim_invites (expires_at) WHERE used_at IS NULL AND revoked_at IS NULL;

    -- Non-tenant, secret-bearing: ENABLE + FORCE RLS + single ops policy (access_requests Pattern A2).
    ALTER TABLE claim_invites ENABLE ROW LEVEL SECURITY;
    ALTER TABLE claim_invites FORCE  ROW LEVEL SECURITY;
    CREATE POLICY allow_ops_claim_invites_all ON claim_invites FOR ALL USING (true) WITH CHECK (true);
    REVOKE ALL PRIVILEGES ON TABLE claim_invites FROM anon, authenticated, service_role;
  `);

  // The token-gated ownership-transfer carve-out. SECURITY DEFINER (runs as owner) so it can write the
  // shadow org/membership, but it validates the token INSIDE and touches ONLY the token's target source.
  // Errors are 'CLAIMERR:<code>' so the service can map them to typed ClaimError codes.
  pgm.sql(`
    CREATE OR REPLACE FUNCTION claim_transfer(p_token text, p_user_id uuid)
    RETURNS TABLE(org_id uuid, location_id uuid)
    LANGUAGE plpgsql SECURITY DEFINER SET search_path = public AS $$
    DECLARE v_source uuid; v_org uuid; v_loc uuid; v_state acquisition_state;
    BEGIN
      -- 1. token = sole authority. Lock the active invite.
      SELECT acquisition_source_id INTO v_source FROM claim_invites
        WHERE token_hash = encode(sha256(p_token::bytea), 'hex')
          AND used_at IS NULL AND revoked_at IS NULL AND expires_at > now()
        FOR UPDATE;
      IF v_source IS NULL THEN RAISE EXCEPTION 'CLAIMERR:INVALID_OR_EXPIRED_TOKEN'; END IF;

      -- 2. resolve the spine from acquisition_sources; require a claimable state.
      SELECT s.org_id, s.location_id, s.state INTO v_org, v_loc, v_state
        FROM acquisition_sources s WHERE s.id = v_source;
      IF v_state <> 'CLAIM_OFFERED' OR v_org IS NULL OR v_loc IS NULL THEN
        RAISE EXCEPTION 'CLAIMERR:NOT_CLAIMABLE'; END IF;

      -- 3. transfer ownership, race-guarded (NULL → user).
      UPDATE organizations SET owner_id = p_user_id WHERE id = v_org AND owner_id IS NULL;
      IF NOT FOUND THEN RAISE EXCEPTION 'CLAIMERR:ALREADY_CLAIMED'; END IF;

      -- 4. membership (status MUST be explicit 'active' — council C7).
      INSERT INTO memberships (user_id, location_id, role, status)
        VALUES (p_user_id, v_loc, 'owner', 'active');

      -- 5. burn the invite (single-use).
      UPDATE claim_invites SET used_at = now(), used_by_user_id = p_user_id
        WHERE acquisition_source_id = v_source AND used_at IS NULL;

      -- 6. state-pinned CLAIM_OFFERED→CLAIMED + erase the raw scraped blob (H-erase) + void grants (H-void).
      UPDATE acquisition_sources
        SET state = 'CLAIMED', claimed_at = now(), place_raw = NULL, menu_draft = NULL
        WHERE id = v_source AND state = 'CLAIM_OFFERED';
      IF NOT FOUND THEN RAISE EXCEPTION 'CLAIMERR:RACED'; END IF;
      DELETE FROM provision_grants WHERE acquisition_source_id = v_source;

      RETURN QUERY SELECT v_org, v_loc;
    END $$;

    REVOKE ALL ON FUNCTION claim_transfer(text, uuid) FROM PUBLIC;
  `);

  // Grant EXECUTE to whatever role can already write orders (the operational role).
  pgm.sql(`
    DO $$
    DECLARE r record;
    BEGIN
      FOR r IN
        SELECT DISTINCT grantee, privilege_type FROM information_schema.role_table_grants
        WHERE table_schema = 'public' AND table_name = 'orders'
          AND privilege_type IN ('SELECT', 'INSERT', 'UPDATE', 'DELETE') AND grantee <> 'PUBLIC'
      LOOP
        EXECUTE format('GRANT %s ON public.claim_invites TO %I', r.privilege_type, r.grantee);
        EXECUTE format('GRANT EXECUTE ON FUNCTION claim_transfer(text, uuid) TO %I', r.grantee);
      END LOOP;
    END
    $$;
  `);
  pgm.sql(`
    DO $$
    BEGIN
      IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'deliveryos_operational_user') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON public.claim_invites TO deliveryos_operational_user;
        GRANT EXECUTE ON FUNCTION claim_transfer(text, uuid) TO deliveryos_operational_user;
      END IF;
    END
    $$;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP FUNCTION IF EXISTS claim_transfer(text, uuid);
    DROP TABLE IF EXISTS claim_invites CASCADE;
  `);
}
