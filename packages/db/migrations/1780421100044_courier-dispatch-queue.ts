import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE courier_dispatch_queue (
      order_id uuid PRIMARY KEY REFERENCES orders(id),
      location_id uuid NOT NULL REFERENCES locations(id),
      enqueued_at timestamptz NOT NULL DEFAULT now(),
      attempts int NOT NULL DEFAULT 0,
      last_attempt_at timestamptz
    );

    ALTER TABLE courier_dispatch_queue ENABLE ROW LEVEL SECURITY;
    
    CREATE POLICY isolate_courier_dispatch_queue ON courier_dispatch_queue
      USING (location_id = current_setting('app.current_tenant')::uuid);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE courier_dispatch_queue;
  `);
}
