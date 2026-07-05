// Crypto payments PHASE A — money ledger schema (ADR-0017, resolution.md RESOLVE round). operator places into
// packages/db/migrations/. Additive + DARK (no runtime reads it until PAYMENTS_PREPAID_ENABLED/CRYPTO_ENABLED).
// All money = integer minor units. ENABLE+FORCE RLS + the dual (member OR app.current_tenant) policy (C3) +
// grant-mirror from orders. Idempotency by DB UNIQUE (insert-wins). DEFINER resolver for the webhook tenancy.
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    -- payment_method: cash-only by construction (mig 1780310044710). Add crypto (live rail) + card
    -- (schema-rich, adapter unbuilt). ADD VALUE must run outside a txn block in older PG; node-pg-migrate
    -- runs each migration in a txn → use the IF NOT EXISTS form which is txn-safe in PG12+.
    ALTER TYPE payment_method ADD VALUE IF NOT EXISTS 'crypto';
    ALTER TYPE payment_method ADD VALUE IF NOT EXISTS 'card';
  `);
  // The new enum values must be committed before they can be used; node-pg-migrate single-txn would error on
  // use-in-same-txn, but we only USE them at runtime (later), so this is fine.
  pgm.sql(`
    -- order.payment_status — the money state machine, decoupled from order_status (fulfillment). Additive,
    -- default 'unpaid' (COD stays unpaid for life; the cash-as-proof spine is unchanged).
    ALTER TABLE orders ADD COLUMN IF NOT EXISTS payment_status text NOT NULL DEFAULT 'unpaid'
      CHECK (payment_status IN ('unpaid','pending','authorized','paid','failed','refunded'));

    -- payments: one charge row per provider invoice. Money SoT (with payment_events). Integer minor units.
    CREATE TABLE IF NOT EXISTS payments (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id),
      order_id uuid NOT NULL REFERENCES orders(id),
      provider text NOT NULL,
      provider_payment_id text,
      status text NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending','authorized','paid','failed','refunded')),
      amount_minor integer NOT NULL CHECK (amount_minor >= 0),
      captured_amount_minor integer NOT NULL DEFAULT 0 CHECK (captured_amount_minor >= 0),
      refunded_amount_minor integer NOT NULL DEFAULT 0 CHECK (refunded_amount_minor >= 0),
      currency_code text NOT NULL,
      created_at timestamptz NOT NULL DEFAULT now(),
      updated_at timestamptz NOT NULL DEFAULT now(),
      -- residual-guard invariant (mirrors Stage-21): refunded <= captured <= amount. No float anywhere.
      CONSTRAINT payments_money_residual CHECK (refunded_amount_minor <= captured_amount_minor
                                                AND captured_amount_minor <= amount_minor),
      CONSTRAINT payments_provider_unique UNIQUE (provider, provider_payment_id)
    );
    CREATE INDEX IF NOT EXISTS payments_order_idx ON payments(order_id);
    CREATE INDEX IF NOT EXISTS payments_location_idx ON payments(location_id);

    -- payment_events: append-only money ledger (like courier_cash_ledger). Idempotent by UNIQUE
    -- (provider, provider_payment_id, type) — Plisio resends one txn_id across status changes, so the
    -- composite admits pending→completed progression while killing same-status replays.
    CREATE TABLE IF NOT EXISTS payment_events (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      payment_id uuid NOT NULL REFERENCES payments(id),
      location_id uuid NOT NULL REFERENCES locations(id),
      provider text NOT NULL,
      provider_payment_id text,
      type text NOT NULL,
      amount_minor integer CHECK (amount_minor >= 0),
      currency_code text,
      signature_verified boolean NOT NULL,
      payload jsonb,
      created_at timestamptz NOT NULL DEFAULT now(),
      CONSTRAINT payment_events_idem_unique UNIQUE (provider, provider_payment_id, type)
    );
    CREATE INDEX IF NOT EXISTS payment_events_payment_idx ON payment_events(payment_id);
    CREATE INDEX IF NOT EXISTS payment_events_provider_ref_idx ON payment_events(provider, provider_payment_id);

    -- ── RLS: ENABLE + FORCE, dual policy (members read their locations; the webhook writes the one tenant it
    -- set via app.current_tenant). GUC-ready now → load-bearing once B3 removes BYPASSRLS. ──
    ALTER TABLE payments ENABLE ROW LEVEL SECURITY;        ALTER TABLE payments FORCE ROW LEVEL SECURITY;
    ALTER TABLE payment_events ENABLE ROW LEVEL SECURITY;  ALTER TABLE payment_events FORCE ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS tenant_dual ON payments;
    CREATE POLICY tenant_dual ON payments FOR ALL
      USING (location_id IN (SELECT app_member_location_ids())
             OR location_id = nullif(current_setting('app.current_tenant', true), '')::uuid)
      WITH CHECK (location_id IN (SELECT app_member_location_ids())
             OR location_id = nullif(current_setting('app.current_tenant', true), '')::uuid);
    DROP POLICY IF EXISTS tenant_dual ON payment_events;
    CREATE POLICY tenant_dual ON payment_events FOR ALL
      USING (location_id IN (SELECT app_member_location_ids())
             OR location_id = nullif(current_setting('app.current_tenant', true), '')::uuid)
      WITH CHECK (location_id IN (SELECT app_member_location_ids())
             OR location_id = nullif(current_setting('app.current_tenant', true), '')::uuid);

    -- grant-mirror: clone orders' grantees onto the two new tables (mirrors mig 1790000000028).
    DO $$
    DECLARE r record;
    BEGIN
      FOR r IN SELECT grantee, privilege_type FROM information_schema.role_table_grants
               WHERE table_name='orders' AND table_schema='public'
                 AND privilege_type IN ('SELECT','INSERT','UPDATE','DELETE')
      LOOP
        EXECUTE format('GRANT %s ON payments TO %I', r.privilege_type, r.grantee);
        EXECUTE format('GRANT %s ON payment_events TO %I', r.privilege_type, r.grantee);
      END LOOP;
    END $$;

    -- C3 webhook tenancy resolver: DEFINER, pinned search_path, returns ONLY the location_id for a provider
    -- ref (the row isn't SELECT-visible to the unauthenticated webhook under RLS). GRANT to the op role.
    CREATE OR REPLACE FUNCTION payment_location_by_provider_ref(p_provider text, p_provider_payment_id text)
      RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
        SELECT location_id FROM payments WHERE provider = p_provider AND provider_payment_id = p_provider_payment_id LIMIT 1
      $fn$;
    REVOKE ALL ON FUNCTION payment_location_by_provider_ref(text, text) FROM PUBLIC;
    DO $$
    DECLARE r record;
    BEGIN
      FOR r IN SELECT DISTINCT grantee FROM information_schema.role_table_grants
               WHERE table_name='orders' AND table_schema='public' AND grantee <> 'PUBLIC'
      LOOP
        EXECUTE format('GRANT EXECUTE ON FUNCTION payment_location_by_provider_ref(text, text) TO %I', r.grantee);
      END LOOP;
    END $$;
  `);
}

export async function down(): Promise<void> {
  // Forward-only.
}
