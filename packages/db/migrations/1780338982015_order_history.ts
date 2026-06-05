import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE order_status_history (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      order_id uuid NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
      location_id uuid NOT NULL,
      from_status order_status,
      to_status   order_status NOT NULL,
      actor text,
      created_at timestamptz NOT NULL DEFAULT now()
    );
  `);

  pgm.sql(`
    ALTER TABLE order_status_history ENABLE ROW LEVEL SECURITY;
    ALTER TABLE order_status_history FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON order_status_history
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);
}

export async function down(): Promise<void> {}
