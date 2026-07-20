import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE courier_assignments (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      order_id uuid NOT NULL REFERENCES orders(id),
      location_id uuid NOT NULL REFERENCES locations(id),
      courier_id uuid NOT NULL REFERENCES couriers(id),
      shift_id uuid REFERENCES courier_shifts(id),
      status text NOT NULL DEFAULT 'assigned' CHECK (status IN ('assigned', 'accepted', 'picked_up', 'delivered', 'cancelled', 'rejected')),
      assigned_at timestamptz NOT NULL DEFAULT now(),
      accepted_at timestamptz,
      picked_up_at timestamptz,
      delivered_at timestamptz,
      cancelled_at timestamptz,
      cancellation_reason text,
      cash_collected boolean NOT NULL DEFAULT false,
      cash_amount integer CHECK (cash_amount IS NULL OR cash_amount >= 0),
      created_at timestamptz NOT NULL DEFAULT now()
    );
    
    CREATE UNIQUE INDEX courier_assignments_order_uniq ON courier_assignments(order_id);
    CREATE INDEX courier_assignments_courier_idx ON courier_assignments(courier_id, status);

    ALTER TABLE courier_assignments ENABLE ROW LEVEL SECURITY;
    
    CREATE POLICY isolate_courier_assignments ON courier_assignments
      USING (location_id = current_setting('app.current_tenant')::uuid);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE courier_assignments;
  `);
}
