import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE couriers (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      email_encrypted bytea NOT NULL,
      email_hash text NOT NULL UNIQUE,
      phone_encrypted bytea,
      phone_hash text,
      full_name_encrypted bytea NOT NULL,
      status text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'deactivated', 'suspended')),
      password_hash text NOT NULL,
      created_at timestamptz NOT NULL DEFAULT now(),
      last_login_at timestamptz,
      deactivated_at timestamptz,
      deactivated_by_owner_id uuid REFERENCES users(id)
    );
    CREATE INDEX couriers_email_hash_idx ON couriers(email_hash);

    CREATE TABLE courier_locations (
      courier_id uuid NOT NULL REFERENCES couriers(id) ON DELETE CASCADE,
      location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      role text NOT NULL DEFAULT 'courier' CHECK (role IN ('courier', 'dispatcher')),
      added_at timestamptz NOT NULL DEFAULT now(),
      added_by_owner_id uuid REFERENCES users(id),
      PRIMARY KEY (courier_id, location_id)
    );
    
    ALTER TABLE courier_locations ENABLE ROW LEVEL SECURITY;
    CREATE POLICY isolate_courier_locations ON courier_locations
      USING (location_id = current_setting('app.current_tenant')::uuid);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE courier_locations;
    DROP TABLE couriers;
  `);
}
