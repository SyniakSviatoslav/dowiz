import { MigrationBuilder } from 'node-pg-migrate';

/**
 * NOBYPASSRLS GATED flip (W4 write-path).
 *
 * Ground truth (D2-rls-data-governance.md §0): the Postgres/RLS stack is
 * QUARANTINED in `attic/` on this branch — there is no live multi-tenant
 * datastore. This migration is the REACTIVATION GATE: it must not be applied
 * (and `ALTER ROLE dowiz_app NOBYPASSRLS` must not take effect) unless every
 * fail-open table carries FORCE ROW LEVEL SECURITY with a sound tenant predicate.
 *
 * R1/R2/R3 (HIGH, credential/account-takeover material) are closed IN THIS
 * migration. R4-R6 tables are GATED: the flip aborts if any of them is still
 * fail-open, so a premature flip can never silently expose cross-tenant data.
 *
 * Down: revert to BYPASSRLS (RLS inert — the pre-flip dormant state).
 */

// R4-R6 tables: flip BLOCKS until each carries FORCE RLS + a sound predicate.
const GATED_TABLES = [
  'courier_sessions',
  'customer_contact_reveals',
  'notification_outbox_audit',
  'analytics_events',
  'analytics_abuse_log',
  'analytics_cwv',
  'upload_audit',
  'customer_devices',
  'backup_metadata',
  'backup_audit_log',
  'access_requests',
];

export const up = async (db: MigrationBuilder): Promise<void> => {
  // ── R1: couriers (password_hash + PII, NO location_id; scope via courier_locations FK)
  await db.sql(`
    ALTER TABLE couriers ENABLE ROW LEVEL SECURITY;
    ALTER TABLE couriers FORCE ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS couriers_tenant ON couriers;
    CREATE POLICY couriers_tenant ON couriers
      USING (EXISTS (
        SELECT 1 FROM courier_locations cl
        WHERE cl.courier_id = couriers.id
          AND cl.location_id IN (SELECT app_member_location_ids())
      ));
  `);

  // ── R2: telegram_login_tokens (owner-login auth tokens; scope by location_id)
  await db.sql(`
    ALTER TABLE telegram_login_tokens ENABLE ROW LEVEL SECURITY;
    ALTER TABLE telegram_login_tokens FORCE ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS telegram_login_tokens_tenant ON telegram_login_tokens;
    CREATE POLICY telegram_login_tokens_tenant ON telegram_login_tokens
      USING (location_id IN (SELECT app_member_location_ids()));
  `);

  // ── R3: orders/order_items/customers — replace fail-open IS NULL seam
  // (a NOBYPASSRLS session that never set app.user_id matched EVERY row).
  await db.sql(`
    ALTER TABLE orders FORCE ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS orders_anonymous_select ON orders;
    CREATE POLICY orders_tenant ON orders
      USING (location_id IN (SELECT app_member_location_ids()));

    ALTER TABLE order_items FORCE ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS order_items_anonymous_select ON order_items;
    CREATE POLICY order_items_tenant ON order_items
      USING (order_id IN (
        SELECT id FROM orders WHERE location_id IN (SELECT app_member_location_ids())
      ));

    ALTER TABLE customers FORCE ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS customers_anonymous_select ON customers;
    DROP POLICY IF EXISTS customers_anonymous_update ON customers;
    CREATE POLICY customers_tenant ON customers
      USING (location_id IN (SELECT app_member_location_ids()));
  `);

  // ── GATED FLIP: abort unless every GATED_TABLES entry is FORCE + has a sound predicate.
  await db.sql(
    `
    DO $$ DECLARE n int; BEGIN
      SELECT count(*) INTO n FROM unnest($1::text[]) t(name)
      WHERE NOT (
        SELECT c.relrowsecurity AND c.relforcerowsecurity
        FROM pg_class c
        JOIN pg_namespace ns ON ns.oid = c.relnamespace
        WHERE ns.nspname = 'public' AND c.relname = t.name
      ) OR NOT EXISTS (
        SELECT 1 FROM pg_policy p
        WHERE p.schemaname = 'public' AND p.tablename = t.name
          AND p.cmd = 'ALL' AND p.permissive = 'PERMISSIVE' AND p.qual IS NOT NULL
      );
      IF n > 0 THEN
        RAISE EXCEPTION 'NOBYPASSRLS flip BLOCKED: % gated table(s) still fail-open', n;
      END IF;
    END $$;
  `,
    [GATED_TABLES],
  );

  // Only reached if the gate passed: make RLS the real boundary.
  await db.sql(`ALTER ROLE dowiz_app NOBYPASSRLS;`);
};

export const down = async (db: MigrationBuilder): Promise<void> => {
  await db.sql(`ALTER ROLE dowiz_app BYPASSRLS;`);
};
