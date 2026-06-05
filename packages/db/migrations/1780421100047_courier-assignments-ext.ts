import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE courier_assignments
      ADD COLUMN IF NOT EXISTS voided_at timestamptz,
      ADD COLUMN IF NOT EXISTS voided_reason text,
      ADD COLUMN IF NOT EXISTS settlement_item_id uuid REFERENCES settlement_items(id);

    -- Update CHECK constraint to include 'voided' status
    ALTER TABLE courier_assignments DROP CONSTRAINT IF EXISTS courier_assignments_status_check;
    ALTER TABLE courier_assignments ADD CONSTRAINT courier_assignments_status_check 
      CHECK (status IN ('assigned', 'accepted', 'picked_up', 'delivered', 'cancelled', 'rejected', 'voided'));

    -- Trigger: prevent UPDATE on cash_collected/cash_amount after initial write (except via reversal flag)
    CREATE OR REPLACE FUNCTION prevent_cash_mutation() RETURNS trigger AS $$
    BEGIN
      -- Only block if OLD cash_collected was already true
      IF OLD.cash_collected = true THEN
        IF OLD.cash_collected IS DISTINCT FROM NEW.cash_collected 
           OR OLD.cash_amount IS DISTINCT FROM NEW.cash_amount THEN
          
          -- Check if it is a specific allowed reversal transaction
          IF current_setting('app.settlement_reversal', true) IS DISTINCT FROM 'true' THEN
            RAISE EXCEPTION 'cash_collected/cash_amount immutable except via settlement reversal';
          END IF;
        END IF;
      END IF;
      RETURN NEW;
    END;
    $$ LANGUAGE plpgsql;

    CREATE TRIGGER courier_assignments_cash_immutable 
      BEFORE UPDATE ON courier_assignments 
      FOR EACH ROW EXECUTE FUNCTION prevent_cash_mutation();
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TRIGGER IF EXISTS courier_assignments_cash_immutable ON courier_assignments;
    DROP FUNCTION IF EXISTS prevent_cash_mutation();

    ALTER TABLE courier_assignments DROP CONSTRAINT IF EXISTS courier_assignments_status_check;
    ALTER TABLE courier_assignments ADD CONSTRAINT courier_assignments_status_check 
      CHECK (status IN ('assigned', 'accepted', 'picked_up', 'delivered', 'cancelled', 'rejected'));

    ALTER TABLE courier_assignments
      DROP COLUMN IF EXISTS settlement_item_id,
      DROP COLUMN IF EXISTS voided_reason,
      DROP COLUMN IF EXISTS voided_at;
  `);
}
