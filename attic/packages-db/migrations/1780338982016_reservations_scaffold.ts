import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE reservations (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id),
      customer_id uuid REFERENCES customers(id),
      slot_at timestamptz NOT NULL,
      party_size int NOT NULL,
      status text NOT NULL DEFAULT 'pending',
      created_at timestamptz NOT NULL DEFAULT now()
    );
  `);

  pgm.sql(`
    ALTER TABLE reservations ENABLE ROW LEVEL SECURITY;
    ALTER TABLE reservations FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON reservations
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);
}

export async function down(): Promise<void> {}
