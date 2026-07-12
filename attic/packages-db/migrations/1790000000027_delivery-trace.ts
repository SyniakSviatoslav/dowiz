import type { MigrationBuilder } from 'node-pg-migrate';

// delivery_trace — immutable audit record of a completed delivery (one row per order).
// Additive observability only; orders/courier_assignments remain the operational
// source of truth. Written by the DELIVERED handler inside its txn (ON CONFLICT DO
// NOTHING → idempotent / recoverable).

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE delivery_trace (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      order_id uuid NOT NULL UNIQUE REFERENCES orders(id) ON DELETE CASCADE,
      location_id uuid NOT NULL,
      courier_id uuid,
      total integer,
      delivered_at timestamptz NOT NULL DEFAULT now(),
      created_at timestamptz NOT NULL DEFAULT now()
    );
    CREATE INDEX delivery_trace_location_idx ON delivery_trace(location_id);
    CREATE INDEX delivery_trace_courier_idx ON delivery_trace(courier_id) WHERE courier_id IS NOT NULL;

    ALTER TABLE delivery_trace ENABLE ROW LEVEL SECURITY;
    ALTER TABLE delivery_trace FORCE  ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON delivery_trace
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);

  // DML grants mirror orders (so the operational role can INSERT the trace at delivery).
  pgm.sql(`
    DO $$
    DECLARE r record;
    BEGIN
      FOR r IN
        SELECT DISTINCT grantee, privilege_type FROM information_schema.role_table_grants
        WHERE table_schema='public' AND table_name='orders'
          AND privilege_type IN ('SELECT','INSERT','UPDATE','DELETE') AND grantee <> 'PUBLIC'
      LOOP
        EXECUTE format('GRANT %s ON public.delivery_trace TO %I', r.privilege_type, r.grantee);
      END LOOP;
    END
    $$;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS delivery_trace CASCADE;`);
}
