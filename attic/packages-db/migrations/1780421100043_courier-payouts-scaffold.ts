import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE courier_payouts (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      courier_id uuid NOT NULL REFERENCES couriers(id),
      location_id uuid NOT NULL REFERENCES locations(id),
      period_start timestamptz NOT NULL,
      period_end timestamptz NOT NULL,
      deliveries_count int NOT NULL DEFAULT 0,
      total_earned integer NOT NULL DEFAULT 0 CHECK (total_earned >= 0),
      status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'paid', 'disputed')),
      approved_at timestamptz,
      approved_by_owner_id uuid REFERENCES users(id),
      created_at timestamptz NOT NULL DEFAULT now(),
      UNIQUE (courier_id, location_id, period_start, period_end)
    );

    ALTER TABLE courier_payouts ENABLE ROW LEVEL SECURITY;
    
    CREATE POLICY isolate_courier_payouts ON courier_payouts
      USING (location_id = current_setting('app.current_tenant')::uuid);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE courier_payouts;
  `);
}
