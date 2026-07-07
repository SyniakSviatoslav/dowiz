  import type { MigrationBuilder } from 'node-pg-migrate';

  export async function up(pgm: MigrationBuilder): Promise<void> {
    // Phase 1.2 — persistent append-only event log (the L architecture)
    pgm.sql(`
      CREATE TABLE order_events (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        order_id uuid NOT NULL REFERENCES orders(id),
        seq bigint NOT NULL,
        at timestamptz NOT NULL,
        cause_hash text NOT NULL,
        payload bytea NOT NULL,
        content_hash text NOT NULL,
        signature bytea,
        created_at timestamptz NOT NULL DEFAULT now(),
        UNIQUE (order_id, seq)
      );
      CREATE INDEX order_events_order_seq_idx ON order_events(order_id, seq);
    `);

    pgm.sql(`
      ALTER TABLE order_events ENABLE ROW LEVEL SECURITY;
      ALTER TABLE order_events FORCE ROW LEVEL SECURITY;
      CREATE POLICY tenant_isolation ON order_events
        USING ( EXISTS (SELECT 1 FROM orders o WHERE o.id = order_events.order_id AND
  o.location_id IN (SELECT app_member_location_ids())) );

      ALTER TABLE order_events REVOKE UPDATE, DELETE FROM app;
    `);
  }

  export async function down(): Promise<void> {}
