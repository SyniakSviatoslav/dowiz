// Shared fixture for the ADR-audit-fix-money DB-level proofs (settlements-catchup.test.ts +
// refund-due-spine.test.ts). Builds a FAITHFUL SUBSET of the prod schema — same table/column/
// constraint names, same uniques (settlement_items_assignment_uniq, payment_events_idem_unique,
// courier_payouts pair-period unique), the real prevent_payout_mutation trigger (1780421100052),
// the real payments/payment_events DDL + FORCE RLS dual policy (1790000000083), and the VERBATIM
// pre-fix app_sweep_timeout_orders() (1790000000078) — then applies the M-1/M-2/M-3 migration
// drafts by executing their up() bodies.
//
// Skips without MONEY_TEST_DATABASE_URL (claim-rls.test.ts pattern). Point it at a THROWAWAY
// database — the fixture DROPS AND RECREATES `public`. e.g.:
//   docker run -d -e POSTGRES_PASSWORD=test -p 127.0.0.1:55432:5432 postgres:16
//   MONEY_TEST_DATABASE_URL=postgres://postgres:test@127.0.0.1:55432/postgres \
//     node --test --import tsx tests/refund-due-spine.test.ts
//
// SCOPE NOTE: these tests prove FN/TRIGGER SEMANTICS on the subset — they are NOT proof the
// migrations apply cleanly on prod's actual schema/roles; that is the operator's preflight
// (scripts/ci-migration-preflight.mjs, per docs/lessons/2026-07-03-prod-staging-schema-drift.md).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import type { Pool, PoolClient } from 'pg';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, '../../..');

export const WATERMARK = '2026-07-10 00:00:00+00';

const SCHEMA_SQL = `
  DO $$ BEGIN CREATE ROLE dowiz_app NOLOGIN; EXCEPTION WHEN duplicate_object THEN NULL; END $$;

  CREATE TABLE locations (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    currency_code text NOT NULL DEFAULT 'ALL'
  );
  CREATE TABLE couriers (id uuid PRIMARY KEY DEFAULT gen_random_uuid());
  CREATE TABLE courier_shifts (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    status text NOT NULL DEFAULT 'available'
  );
  CREATE TABLE orders (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    location_id uuid NOT NULL REFERENCES locations(id),
    status text NOT NULL DEFAULT 'PENDING',
    total integer NOT NULL DEFAULT 0,
    payment_status text NOT NULL DEFAULT 'unpaid',
    timeout_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    confirmed_at timestamptz, preparing_at timestamptz, ready_at timestamptz,
    in_delivery_at timestamptz, delivered_at timestamptz, picked_up_at timestamptz
  );
  CREATE TABLE order_items (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id uuid REFERENCES orders(id),
    name_snapshot text, price_snapshot integer, quantity integer
  );
  CREATE TABLE order_status_history (
    id bigserial PRIMARY KEY,
    order_id uuid, location_id uuid, from_status text, to_status text,
    actor text, comment text, created_at timestamptz NOT NULL DEFAULT now()
  );
  CREATE TABLE courier_assignments (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    courier_id uuid NOT NULL REFERENCES couriers(id),
    location_id uuid NOT NULL REFERENCES locations(id),
    order_id uuid REFERENCES orders(id),
    shift_id uuid REFERENCES courier_shifts(id),
    status text NOT NULL,
    cash_collected boolean NOT NULL DEFAULT false,
    cash_amount integer,
    delivered_at timestamptz,
    cancelled_at timestamptz,
    cancellation_reason text,
    settlement_item_id uuid
  );

  -- real DDL: 1780421100043
  CREATE TABLE courier_payouts (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    courier_id uuid NOT NULL REFERENCES couriers(id),
    location_id uuid NOT NULL REFERENCES locations(id),
    period_start timestamptz NOT NULL,
    period_end timestamptz NOT NULL,
    deliveries_count int NOT NULL DEFAULT 0,
    total_earned integer NOT NULL DEFAULT 0 CHECK (total_earned >= 0),
    status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'paid', 'disputed')),
    approved_at timestamptz,
    approved_by_owner_id uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (courier_id, location_id, period_start, period_end)
  );

  -- real DDL: 1780421100045
  CREATE TABLE settlement_items (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    payout_id uuid NOT NULL REFERENCES courier_payouts(id) ON DELETE RESTRICT,
    assignment_id uuid NOT NULL REFERENCES courier_assignments(id) ON DELETE RESTRICT,
    location_id uuid NOT NULL REFERENCES locations(id),
    amount integer NOT NULL CHECK (amount >= 0),
    currency_code text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
  );
  CREATE UNIQUE INDEX settlement_items_assignment_uniq ON settlement_items(assignment_id);
  CREATE INDEX settlement_items_payout_idx ON settlement_items(payout_id);

  -- real DDL: 1780421100046
  CREATE TABLE settlement_audit_log (
    id bigserial PRIMARY KEY,
    payout_id uuid NOT NULL REFERENCES courier_payouts(id) ON DELETE CASCADE,
    action text NOT NULL CHECK (action IN ('generated', 'approved', 'paid', 'disputed', 'reopened', 'item_added', 'item_voided')),
    actor_kind text NOT NULL CHECK (actor_kind IN ('owner', 'courier', 'system')),
    actor_id uuid,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    location_id uuid NOT NULL REFERENCES locations(id),
    created_at timestamptz NOT NULL DEFAULT now()
  );

  -- real trigger: 1780421100052 (payout immutability)
  CREATE OR REPLACE FUNCTION prevent_payout_mutation() RETURNS trigger AS $$
  BEGIN
    IF OLD.status IN ('approved', 'paid') THEN
      IF OLD.deliveries_count IS DISTINCT FROM NEW.deliveries_count
         OR OLD.total_earned IS DISTINCT FROM NEW.total_earned THEN
        RAISE EXCEPTION 'payout immutable after approval';
      END IF;
    END IF;
    RETURN NEW;
  END;
  $$ LANGUAGE plpgsql;
  CREATE TRIGGER courier_payouts_immutable
    BEFORE UPDATE ON courier_payouts
    FOR EACH ROW EXECUTE FUNCTION prevent_payout_mutation();

  -- real DDL: 1790000000083 (payments ledger) — member arm stubbed empty (no memberships here)
  CREATE FUNCTION app_member_location_ids() RETURNS SETOF uuid
    LANGUAGE sql STABLE AS $$ SELECT id FROM locations WHERE false $$;
  CREATE TABLE payments (
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
    CONSTRAINT payments_money_residual CHECK (refunded_amount_minor <= captured_amount_minor
                                              AND captured_amount_minor <= amount_minor),
    CONSTRAINT payments_provider_unique UNIQUE (provider, provider_payment_id)
  );
  CREATE INDEX payments_order_idx ON payments(order_id);
  CREATE TABLE payment_events (
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
  CREATE INDEX payment_events_payment_idx ON payment_events(payment_id);
  ALTER TABLE payments ENABLE ROW LEVEL SECURITY;        ALTER TABLE payments FORCE ROW LEVEL SECURITY;
  ALTER TABLE payment_events ENABLE ROW LEVEL SECURITY;  ALTER TABLE payment_events FORCE ROW LEVEL SECURITY;
  CREATE POLICY tenant_dual ON payments FOR ALL
    USING (location_id IN (SELECT app_member_location_ids())
           OR location_id = nullif(current_setting('app.current_tenant', true), '')::uuid)
    WITH CHECK (location_id IN (SELECT app_member_location_ids())
           OR location_id = nullif(current_setting('app.current_tenant', true), '')::uuid);
  CREATE POLICY tenant_dual ON payment_events FOR ALL
    USING (location_id IN (SELECT app_member_location_ids())
           OR location_id = nullif(current_setting('app.current_tenant', true), '')::uuid)
    WITH CHECK (location_id IN (SELECT app_member_location_ids())
           OR location_id = nullif(current_setting('app.current_tenant', true), '')::uuid);

  -- VERBATIM pre-fix sweep fn: 1790000000078 (byte-identical body — v2 design leaves it untouched)
  CREATE OR REPLACE FUNCTION app_sweep_timeout_orders() RETURNS TABLE(id uuid, location_id uuid)
    LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
      WITH cancelled AS (
        UPDATE orders SET status='CANCELLED', timeout_at=NULL
          WHERE status='PENDING' AND timeout_at IS NOT NULL AND timeout_at < now()
          RETURNING id, location_id),
      hist AS (
        INSERT INTO order_status_history (order_id, location_id, from_status, to_status, actor)
          SELECT id, location_id, 'PENDING','CANCELLED','system:timeout' FROM cancelled)
      SELECT id, location_id FROM cancelled $fn$;

  -- VERBATIM pre-fix settlement fn: 1790000000078 (the M-2 draft CREATE OR REPLACEs this)
  CREATE OR REPLACE FUNCTION app_generate_settlements(p_period_start timestamptz, p_period_end timestamptz)
    RETURNS void LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, public, pg_temp AS $fn$
  DECLARE v_pair record; v_payout record; v_item record; v_added_items integer; v_added_total integer;
  BEGIN
    FOR v_pair IN
      SELECT DISTINCT courier_id, location_id FROM courier_assignments
      WHERE status='delivered' AND cash_collected=true AND delivered_at >= p_period_start AND delivered_at < p_period_end
    LOOP
      INSERT INTO courier_payouts (courier_id, location_id, period_start, period_end, status)
      VALUES (v_pair.courier_id, v_pair.location_id, p_period_start, p_period_end, 'pending')
      ON CONFLICT (courier_id, location_id, period_start, period_end) DO UPDATE SET status = courier_payouts.status
      RETURNING id, status INTO v_payout;
      v_added_items := 0; v_added_total := 0;
      FOR v_item IN
        SELECT ca.id, ca.cash_amount, loc.currency_code FROM courier_assignments ca
        JOIN locations loc ON loc.id = ca.location_id
        WHERE ca.courier_id=v_pair.courier_id AND ca.location_id=v_pair.location_id AND ca.status='delivered'
          AND ca.cash_collected=true AND ca.delivered_at >= p_period_start AND ca.delivered_at < p_period_end
          AND NOT EXISTS (SELECT 1 FROM settlement_items si WHERE si.assignment_id = ca.id)
        FOR UPDATE OF ca SKIP LOCKED
      LOOP
        INSERT INTO settlement_items (payout_id, assignment_id, location_id, amount, currency_code)
        VALUES (v_payout.id, v_item.id, v_pair.location_id, v_item.cash_amount, v_item.currency_code)
        ON CONFLICT (assignment_id) DO NOTHING;
        UPDATE courier_assignments SET settlement_item_id = (SELECT id FROM settlement_items WHERE assignment_id = v_item.id)
        WHERE id = v_item.id;
        v_added_items := v_added_items + 1; v_added_total := v_added_total + v_item.cash_amount;
      END LOOP;
      IF v_added_items > 0 THEN
        UPDATE courier_payouts SET deliveries_count = deliveries_count + v_added_items, total_earned = total_earned + v_added_total
        WHERE id = v_payout.id;
        INSERT INTO settlement_audit_log (payout_id, location_id, action, actor_kind, metadata)
        VALUES (v_payout.id, v_pair.location_id, 'generated', 'system', jsonb_build_object('added_items', v_added_items, 'added_total', v_added_total));
      END IF;
    END LOOP;
  END $fn$;
`;

/**
 * Per-process isolated database: `node --test` runs test FILES in parallel child processes, so two
 * suites sharing one database race each other's resetSchema (DROP SCHEMA mid-run). Each suite gets
 * its own `money_spine_<pid>` database, dropped on teardown.
 */
export async function createIsolatedPool(baseUrl: string): Promise<{ pool: Pool; dbUrl: string; teardown: () => Promise<void> }> {
  const { Pool: PgPool } = await import('pg');
  const admin = new PgPool({ connectionString: baseUrl, max: 1 });
  const dbName = `money_spine_${process.pid}`;
  await admin.query(`DROP DATABASE IF EXISTS ${dbName} WITH (FORCE)`);
  await admin.query(`CREATE DATABASE ${dbName}`);
  const u = new URL(baseUrl);
  u.pathname = '/' + dbName;
  const dbUrl = u.toString();
  const pool = new PgPool({ connectionString: dbUrl, max: 4 });
  return {
    pool,
    dbUrl,
    teardown: async () => {
      await pool.end().catch(() => {});
      await admin.query(`DROP DATABASE IF EXISTS ${dbName} WITH (FORCE)`).catch(() => {});
      await admin.end().catch(() => {});
    },
  };
}

/** Resolve a money migration by number — prefers the operator-placed location, falls back to the draft. */
function resolveMigration(basename: string): string {
  const placed = path.join(REPO_ROOT, 'packages/db/migrations', basename);
  const draft = path.join(REPO_ROOT, 'docs/design/audit-fix-money/migration-drafts', basename);
  if (fs.existsSync(placed)) return placed;
  if (fs.existsSync(draft)) return draft;
  throw new Error(`money migration not found in either location: ${basename}`);
}

/** Execute a draft/placed migration's up() against the client (pgm.sql shim, statements in order). */
export async function applyMigration(client: PoolClient, basename: string): Promise<void> {
  const mod = await import(pathToFileURL(resolveMigration(basename)).href);
  const stmts: string[] = [];
  await mod.up({ sql: (s: string) => { stmts.push(s); } });
  for (const s of stmts) await client.query(s);
}

/** Drop + rebuild the subset schema, verbatim pre-fix fns included. Does NOT apply the fix drafts. */
export async function resetSchema(pool: Pool): Promise<void> {
  const client = await pool.connect();
  try {
    await client.query('DROP SCHEMA public CASCADE');
    await client.query('CREATE SCHEMA public');
    await client.query('GRANT ALL ON SCHEMA public TO public');
    await client.query(SCHEMA_SQL);
  } finally {
    client.release();
  }
}

/** Apply the three ADR-audit-fix-money migrations (M-2, M-1, M-3) in order. */
export async function applyMoneyFix(pool: Pool): Promise<void> {
  const client = await pool.connect();
  try {
    await applyMigration(client, '1790000000085_settlements-catchup.ts');
    await applyMigration(client, '1790000000086_refund-due-trigger.ts');
    await applyMigration(client, '1790000000087_refund-due-reconciler.ts');
  } finally {
    client.release();
  }
}

// ── seed helpers ─────────────────────────────────────────────────────────────
export async function seedLocation(pool: Pool): Promise<string> {
  const r = await pool.query(`INSERT INTO locations DEFAULT VALUES RETURNING id`);
  return r.rows[0].id;
}
export async function seedPair(pool: Pool): Promise<{ locationId: string; courierId: string; shiftId: string }> {
  const locationId = await seedLocation(pool);
  const c = await pool.query(`INSERT INTO couriers DEFAULT VALUES RETURNING id`);
  const s = await pool.query(`INSERT INTO courier_shifts DEFAULT VALUES RETURNING id`);
  return { locationId, courierId: c.rows[0].id, shiftId: s.rows[0].id };
}
export async function seedDeliveredCash(
  pool: Pool,
  p: { locationId: string; courierId: string; shiftId?: string },
  deliveredAt: string,
  cash = 1500,
): Promise<string> {
  const r = await pool.query(
    `INSERT INTO courier_assignments (courier_id, location_id, shift_id, status, cash_collected, cash_amount, delivered_at)
     VALUES ($1, $2, $3, 'delivered', true, $4, $5) RETURNING id`,
    [p.courierId, p.locationId, p.shiftId ?? null, cash, deliveredAt],
  );
  return r.rows[0].id;
}
export async function seedOrder(pool: Pool, locationId: string, opts: { status?: string; timeoutAt?: string | null; paymentStatus?: string } = {}): Promise<string> {
  const r = await pool.query(
    `INSERT INTO orders (location_id, status, timeout_at, payment_status, total)
     VALUES ($1, $2, $3, $4, 1000) RETURNING id`,
    [locationId, opts.status ?? 'PENDING', opts.timeoutAt ?? null, opts.paymentStatus ?? 'unpaid'],
  );
  return r.rows[0].id;
}
export async function seedPaidPayment(pool: Pool, locationId: string, orderId: string, providerPaymentId: string | null, amount = 1000): Promise<string> {
  const r = await pool.query(
    `INSERT INTO payments (location_id, order_id, provider, provider_payment_id, status, amount_minor, captured_amount_minor, currency_code)
     VALUES ($1, $2, 'plisio', $3, 'paid', $4, $4, 'ALL') RETURNING id`,
    [locationId, orderId, providerPaymentId, amount],
  );
  return r.rows[0].id;
}
export async function refundDueCount(pool: Pool, orderId: string): Promise<number> {
  const r = await pool.query(
    `SELECT count(*)::int AS n FROM payment_events e JOIN payments p ON p.id = e.payment_id
      WHERE p.order_id = $1 AND e.type = 'refund_due'`,
    [orderId],
  );
  return r.rows[0].n;
}
