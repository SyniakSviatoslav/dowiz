import type { MigrationBuilder } from 'node-pg-migrate';

// courier_cash_ledger — append-only AUDIT trail of cash events at delivery.
// ponytail: this is an immutable audit log, NOT the authoritative settlement ledger
// (settlement_items + settlement.cron remain the source of truth for payouts). A
// 'hold' row is appended when a courier collects cash on delivery; 'release'/'settle'
// are reserved for a future settlement integration and are not written here.

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE courier_cash_ledger (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      courier_id uuid NOT NULL,
      location_id uuid NOT NULL,
      order_id uuid NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
      type text NOT NULL CHECK (type IN ('hold','release','settle')),
      amount integer NOT NULL CHECK (amount >= 0),
      created_at timestamptz NOT NULL DEFAULT now(),
      UNIQUE (order_id, type)
    );
    CREATE INDEX courier_cash_ledger_courier_idx ON courier_cash_ledger(courier_id);
    CREATE INDEX courier_cash_ledger_location_idx ON courier_cash_ledger(location_id);

    ALTER TABLE courier_cash_ledger ENABLE ROW LEVEL SECURITY;
    ALTER TABLE courier_cash_ledger FORCE  ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON courier_cash_ledger
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);

  pgm.sql(`
    DO $$
    DECLARE r record;
    BEGIN
      FOR r IN
        SELECT DISTINCT grantee, privilege_type FROM information_schema.role_table_grants
        WHERE table_schema='public' AND table_name='orders'
          AND privilege_type IN ('SELECT','INSERT','UPDATE','DELETE') AND grantee <> 'PUBLIC'
      LOOP
        EXECUTE format('GRANT %s ON public.courier_cash_ledger TO %I', r.privilege_type, r.grantee);
      END LOOP;
    END
    $$;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS courier_cash_ledger CASCADE;`);
}
