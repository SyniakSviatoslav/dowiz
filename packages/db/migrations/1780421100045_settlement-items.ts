import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE settlement_items (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      payout_id uuid NOT NULL REFERENCES courier_payouts(id) ON DELETE RESTRICT,
      assignment_id uuid NOT NULL REFERENCES courier_assignments(id) ON DELETE RESTRICT,
      location_id uuid NOT NULL REFERENCES locations(id),
      amount integer NOT NULL CHECK (amount >= 0),
      currency_code text NOT NULL,
      created_at timestamptz NOT NULL DEFAULT now()
    );

    CREATE UNIQUE INDEX settlement_items_assignment_uniq ON settlement_items(assignment_id);
    CREATE INDEX settlement_items_payout_idx ON settlement_items(payout_id);

    ALTER TABLE settlement_items ENABLE ROW LEVEL SECURITY;
    
    CREATE POLICY isolate_settlement_items ON settlement_items
      USING (location_id = current_setting('app.current_tenant')::uuid);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE settlement_items;
  `);
}
