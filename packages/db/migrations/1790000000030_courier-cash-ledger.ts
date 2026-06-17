import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE IF NOT EXISTS courier_cash_ledger (
      id           uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
      courier_id   uuid        NOT NULL REFERENCES couriers(id),
      location_id  uuid        NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      order_id     uuid        NOT NULL REFERENCES orders(id),
      amount       numeric     NOT NULL,
      currency_code text       NOT NULL,
      type         text        NOT NULL CHECK (type IN ('hold', 'release', 'settle')),
      created_at   timestamptz NOT NULL DEFAULT now()
    );

    CREATE INDEX IF NOT EXISTS courier_cash_ledger_courier_id_idx  ON courier_cash_ledger (courier_id);
    CREATE INDEX IF NOT EXISTS courier_cash_ledger_location_id_idx ON courier_cash_ledger (location_id);
    CREATE INDEX IF NOT EXISTS courier_cash_ledger_order_id_idx    ON courier_cash_ledger (order_id);

    ALTER TABLE courier_cash_ledger ENABLE ROW LEVEL SECURITY;
    ALTER TABLE courier_cash_ledger FORCE ROW LEVEL SECURITY;

    CREATE POLICY owner_select_cash_ledger ON courier_cash_ledger
      FOR SELECT
      USING (location_id IN (SELECT app_member_location_ids()));

    CREATE POLICY courier_select_own_ledger ON courier_cash_ledger
      FOR SELECT
      USING (courier_id = app_current_user());
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS courier_cash_ledger CASCADE;`);
}
