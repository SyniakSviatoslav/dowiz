import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`CREATE TYPE message_sender AS ENUM ('courier', 'customer', 'owner');`);

  pgm.sql(`
    CREATE TABLE order_messages (
      id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      order_id    uuid NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
      location_id uuid NOT NULL REFERENCES locations(id),
      sender      message_sender NOT NULL,
      preset_key  text NOT NULL,
      params      jsonb NOT NULL DEFAULT '{}',
      body        text,
      read_at     timestamptz,
      created_at  timestamptz NOT NULL DEFAULT now(),
      CHECK (body IS NULL)
    );
  `);

  pgm.sql(`CREATE INDEX IF NOT EXISTS order_messages_order_idx ON order_messages (order_id, created_at);`);

  pgm.sql(`ALTER TABLE order_messages ENABLE ROW LEVEL SECURITY;`);
  pgm.sql(`ALTER TABLE order_messages FORCE ROW LEVEL SECURITY;`);

  pgm.sql(`
    CREATE POLICY tenant_isolation ON order_messages
      USING (order_id IN (SELECT id FROM orders WHERE location_id IN (SELECT app_member_location_ids())))
      WITH CHECK (order_id IN (SELECT id FROM orders WHERE location_id IN (SELECT app_member_location_ids())));
  `);

  pgm.sql(`
    CREATE POLICY customer_self_select ON order_messages
      FOR SELECT
      USING (order_id IN (SELECT id FROM orders WHERE customer_id = app_current_user()));
  `);

  pgm.sql(`
    CREATE POLICY customer_insert ON order_messages
      FOR INSERT
      WITH CHECK (
        order_id IN (SELECT id FROM orders WHERE customer_id = app_current_user())
        AND sender = 'customer'
      );
  `);

  pgm.sql(`COMMENT ON TABLE order_messages IS 'Order-scoped preset messages (CR-8). body=RESERVED NULL until MSG-5 free-text flag.';`);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS order_messages;`);
  pgm.sql(`DROP TYPE IF EXISTS message_sender;`);
}
