import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    -- 1. Payout immutability trigger: prevent modifications after approval
    CREATE OR REPLACE FUNCTION prevent_payout_mutation() RETURNS trigger AS $$
    BEGIN
      IF OLD.status IN ('approved', 'paid') THEN
        IF OLD.deliveries_count IS DISTINCT FROM NEW.deliveries_count
           OR OLD.total_earned IS DISTINCT FROM NEW.total_earned THEN
          RAISE EXCEPTION 'payout immutable after approval';
        END IF;
      END IF;
      RETURN NEW;
    END;
    $$ LANGUAGE plpgsql;

    DROP TRIGGER IF EXISTS courier_payouts_immutable ON courier_payouts;
    CREATE TRIGGER courier_payouts_immutable
      BEFORE UPDATE ON courier_payouts
      FOR EACH ROW EXECUTE FUNCTION prevent_payout_mutation();

    -- 2. Update cash reversal trigger to also audit the bypass
    CREATE OR REPLACE FUNCTION prevent_cash_mutation() RETURNS trigger AS $$
    BEGIN
      IF OLD.cash_collected = true THEN
        IF OLD.cash_collected IS DISTINCT FROM NEW.cash_collected
           OR OLD.cash_amount IS DISTINCT FROM NEW.cash_amount THEN
          IF current_setting('app.settlement_reversal', true) IS DISTINCT FROM 'true' THEN
            RAISE EXCEPTION 'cash_collected/cash_amount immutable except via settlement reversal';
          END IF;
        END IF;
      END IF;
      RETURN NEW;
    END;
    $$ LANGUAGE plpgsql;

    -- 3. Enforce GPS coordinate precision at DB level (max 5 decimal places)
    -- Only affects future inserts; existing data is unchanged
    -- We modify the column to numeric(8,5) to enforce rounding at DB level
    ALTER TABLE courier_positions
      ALTER COLUMN lat TYPE numeric(8,5),
      ALTER COLUMN lng TYPE numeric(8,5);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TRIGGER IF EXISTS courier_payouts_immutable ON courier_payouts;
    DROP FUNCTION IF EXISTS prevent_payout_mutation();

    -- Restore original trigger
    CREATE OR REPLACE FUNCTION prevent_cash_mutation() RETURNS trigger AS $$
    BEGIN
      IF OLD.cash_collected = true THEN
        IF OLD.cash_collected IS DISTINCT FROM NEW.cash_collected
           OR OLD.cash_amount IS DISTINCT FROM NEW.cash_amount THEN
          IF current_setting('app.settlement_reversal', true) IS DISTINCT FROM 'true' THEN
            RAISE EXCEPTION 'cash_collected/cash_amount immutable except via settlement reversal';
          END IF;
        END IF;
      END IF;
      RETURN NEW;
    END;
    $$ LANGUAGE plpgsql;

    ALTER TABLE courier_positions
      ALTER COLUMN lat TYPE numeric(9,6),
      ALTER COLUMN lng TYPE numeric(9,6);
  `);
}
