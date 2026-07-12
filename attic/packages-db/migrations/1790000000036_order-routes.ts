import type { MigrationBuilder } from 'node-pg-migrate';

// order_routes — durable copy of the planned road route (polyline) for a delivery.
// Previously the route lived only in Redis with a 2h TTL, so it was lost after the
// window / a flush. The worker upserts here once at picked_up (and on a re-route);
// the customer status endpoint serves it as a fallback when Redis has expired.
// Advisory data — writes are best-effort and never block the delivery flow.
//
// RLS mirrors courier_positions exactly (ENABLE + current_tenant USING policy), so
// it behaves identically to that already-working table for the same pools.

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE order_routes (
      order_id uuid PRIMARY KEY REFERENCES orders(id) ON DELETE CASCADE,
      location_id uuid NOT NULL REFERENCES locations(id),
      polyline text NOT NULL,
      distance_meters integer,
      duration_seconds integer,
      updated_at timestamptz NOT NULL DEFAULT now()
    );

    CREATE INDEX order_routes_location_idx ON order_routes(location_id);

    ALTER TABLE order_routes ENABLE ROW LEVEL SECURITY;

    CREATE POLICY isolate_order_routes ON order_routes
      USING (location_id = current_setting('app.current_tenant')::uuid);
  `);

  // Mirror DML grants from orders so the operational role can read/write the route.
  pgm.sql(`
    DO \$\$
    DECLARE r record;
    BEGIN
      FOR r IN
        SELECT DISTINCT grantee, privilege_type FROM information_schema.role_table_grants
        WHERE table_schema='public' AND table_name='orders'
          AND privilege_type IN ('SELECT','INSERT','UPDATE','DELETE') AND grantee <> 'PUBLIC'
      LOOP
        EXECUTE format('GRANT %s ON public.order_routes TO %I', r.privilege_type, r.grantee);
      END LOOP;
    END
    \$\$;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS order_routes CASCADE;`);
}
