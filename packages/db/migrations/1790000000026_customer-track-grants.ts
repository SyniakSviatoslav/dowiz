import type { MigrationBuilder } from 'node-pg-migrate';

// customer_track_grants — single-use opaque-code grants backing tracking links
// embedded in order-confirmation URLs (https://{slug}.dowiz.org/s/{slug}/order/{id}?t={code}).
//
// Flow mirrors the existing courier_invites / customer_otp_sessions handoff:
//   1. Order mint INSERTs a grant (token_hash = sha256(code)) inside the order txn.
//   2. The order page, lacking a valid customer JWT, POSTs the raw code to
//      /api/customer/track/exchange, which JOINs grant -> orders -> customers and
//      reissues the standard 7-day customer JWT (issueCustomerToken).
//
// GRANTS — the landmine. The operational pool (***REDACTED***) performs
// both the mint INSERT and the exchange use_count++ UPDATE. Migration 015's
// SELECT-only lockdown for deliveryos_operational_user is aspirational and not the
// live role (order INSERTs succeed today, so the deployed role holds DML). Rather
// than hard-code a role name that may not match the deployed secret, we mirror
// whatever DML `orders` already grants — guaranteeing the same role that writes
// orders can write this table — and additionally grant the aspirational
// deliveryos_operational_user so the table is correct whichever role is live.

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE customer_track_grants (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      order_id uuid NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
      location_id uuid NOT NULL REFERENCES locations(id),
      token_hash text NOT NULL UNIQUE,
      expires_at timestamptz NOT NULL,
      use_count integer NOT NULL DEFAULT 0,
      created_at timestamptz NOT NULL DEFAULT now()
    );

    -- Cleanup job scans by expiry; exchange looks up by token_hash (UNIQUE already indexes it).
    CREATE INDEX customer_track_grants_expires_idx ON customer_track_grants(expires_at);

    -- RLS: mirror courier_invites exactly (tenant isolation via app.current_tenant).
    -- The pre-auth exchange runs on the operational pool with no tenant context and
    -- an explicit WHERE token_hash = $1; that path relies on the operational role
    -- bypassing RLS (ADR-006), exactly as order creation does today.
    ALTER TABLE customer_track_grants ENABLE ROW LEVEL SECURITY;
    ALTER TABLE customer_track_grants FORCE  ROW LEVEL SECURITY;
    CREATE POLICY isolate_customer_track_grants ON customer_track_grants
      USING (location_id = current_setting('app.current_tenant')::uuid);
  `);

  // Mirror the DML grants that `orders` already holds, so the same operational
  // role that writes orders can mint/update grants — regardless of its name.
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
          'GRANT %s ON public.customer_track_grants TO %I',
          r.privilege_type, r.grantee
        );
      END LOOP;
    END
    $$;
  `);

  // Forward-compat: the aspirational operational role (migration 015) needs DML
  // here too if ***REDACTED*** is ever flipped to it.
  pgm.sql(`
    DO $$
    BEGIN
      IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'deliveryos_operational_user') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON public.customer_track_grants
          TO deliveryos_operational_user;
      END IF;
    END
    $$;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS customer_track_grants CASCADE;`);
}
