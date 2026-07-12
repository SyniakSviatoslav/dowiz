import type { MigrationBuilder } from 'node-pg-migrate';

// access_requests — non-tenant, PII-bearing public "register interest" capture
// (ADR-soft-access-gate). One row per email; the unique(email) constraint is the
// idempotency anchor (stored trim+lower in the handler). Lawful basis = EXPLICIT
// CONSENT (STOP-2): consent_at + privacy_version are the per-row evidence; a row is
// only ever written on a structurally-validated `consent === true` (z.literal(true)).
//
// RLS posture (Decision-2 / B4): ENABLE+FORCE + a single FOR ALL USING(true) ops
// policy reproduces the ops_worker_heartbeat pattern (1780691408625). RLS here is a
// linter / anti-BYPASSRLS guard, NOT row isolation — the real boundary is the GRANT
// layer below (anon/authenticated/service_role hard-revoked & lack schema USAGE).
//
// GRANT pattern (the landmine, learned from 1790000000026_customer-track-grants):
// migration 015's SELECT-only lockdown for deliveryos_operational_user is aspirational
// and may not be the live role. So we MIRROR whatever DML `orders` already grants —
// guaranteeing the same operational role that writes orders can write this table —
// and ALSO grant the aspirational deliveryos_operational_user for forward-compat.
// DELETE is granted day one (STOP-2): a PII store must have an erasure exit on day one.
//
// Forward-only: down() is intentionally a no-op (PII must not ride a DROP; the day-one
// DELETE grant + scripts/erase-access-request.ts is the erasure path either way — B10).
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE access_requests (
      id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      email           text NOT NULL UNIQUE,          -- stored trim+lower (normalized in code)
      source          text,
      locale          text,                          -- app locales: 'sq' | 'en' (free text, not a CHECK enum)
      status          text NOT NULL DEFAULT 'new',   -- new | invited | declined (manual transitions)
      ip_hash         text,                          -- sha256(realClientIp).slice(0,16), NEVER raw IP
      consent_at      timestamptz NOT NULL,          -- STOP-2: when the user consented (set in handler)
      privacy_version text        NOT NULL,          -- STOP-2: which privacy notice was consented to
      notified_at     timestamptz,
      notify_attempts smallint    NOT NULL DEFAULT 0, -- R2-9: bounded reconcile re-feed guard
      created_at      timestamptz NOT NULL DEFAULT now()
    );

    CREATE INDEX idx_access_requests_status_created
      ON access_requests (status, created_at DESC);                       -- ops listing of status='new'
    CREATE INDEX idx_access_requests_unnotified
      ON access_requests (created_at) WHERE notified_at IS NULL;          -- reconcile sweep predicate
    CREATE INDEX idx_access_requests_created
      ON access_requests (created_at);                                    -- 12-month retention sweep predicate

    -- Non-tenant, PII-bearing: ENABLE + FORCE RLS, single ops policy (Pattern A2).
    ALTER TABLE access_requests ENABLE ROW LEVEL SECURITY;
    ALTER TABLE access_requests FORCE  ROW LEVEL SECURITY;
    CREATE POLICY allow_ops_access_requests_all ON access_requests
      FOR ALL USING (true) WITH CHECK (true);

    -- Remove from the Supabase Data API perimeter (mirror the non-tenant identity tables).
    REVOKE ALL PRIVILEGES ON TABLE access_requests FROM anon, authenticated, service_role;
  `);

  // Mirror the DML grants that `orders` already holds, so the same operational role
  // that writes orders can write access_requests — regardless of its deployed name.
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
          'GRANT %s ON public.access_requests TO %I',
          r.privilege_type, r.grantee
        );
      END LOOP;
    END
    $$;
  `);

  // Forward-compat: the aspirational operational role (migration 015) gets explicit DML
  // (incl. DELETE for day-one erasure) if ***REDACTED*** is ever flipped to it.
  pgm.sql(`
    DO $$
    BEGIN
      IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'deliveryos_operational_user') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON public.access_requests
          TO deliveryos_operational_user;
      END IF;
    END
    $$;
  `);
}

export async function down(): Promise<void> {
  // Forward-only per migration discipline. PII is erasable via the day-one DELETE
  // grant + scripts/erase-access-request.ts regardless of table presence (B10).
}
