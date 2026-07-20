import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE courier_audit_log (
      id bigserial PRIMARY KEY,
      courier_id uuid REFERENCES couriers(id),
      location_id uuid REFERENCES locations(id),
      action text NOT NULL,
      actor_kind text NOT NULL CHECK (actor_kind IN ('owner', 'courier', 'system')),
      actor_id uuid,
      metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
      ip_hash text,
      user_agent_hash text,
      created_at timestamptz NOT NULL DEFAULT now()
    );
    CREATE INDEX courier_audit_log_courier_idx ON courier_audit_log(courier_id, created_at DESC);
    CREATE INDEX courier_audit_log_location_idx ON courier_audit_log(location_id, created_at DESC);
    
    ALTER TABLE courier_audit_log ENABLE ROW LEVEL SECURITY;
    CREATE POLICY isolate_courier_audit_log ON courier_audit_log
      USING (location_id = current_setting('app.current_tenant')::uuid);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE courier_audit_log;
  `);
}
