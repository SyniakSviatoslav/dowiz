import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    -- Add CHECK constraint to existing no_show_count
    ALTER TABLE customers
      ADD CONSTRAINT customers_no_show_count_check CHECK (no_show_count >= 0),
      ADD COLUMN completed_count integer NOT NULL DEFAULT 0 CHECK (completed_count >= 0),
      ADD COLUMN last_no_show_at timestamptz;

    -- Partial index for "has no-show" queries (used by E26 signals)
    CREATE INDEX customers_no_show_idx ON customers(id) WHERE no_show_count > 0;

    COMMENT ON COLUMN customers.no_show_count IS 'Advisory: incremented by owner confirmation. Soft signal, decays via last_no_show_at. Never used for auto-ban.';
    COMMENT ON COLUMN customers.completed_count IS 'Advisory: incremented on order DELIVERED. Positive counter.';
    COMMENT ON COLUMN customers.last_no_show_at IS 'Used to decay no_show signal. Older = weaker signal.';
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP INDEX IF EXISTS customers_no_show_idx;
    ALTER TABLE customers
      DROP CONSTRAINT IF EXISTS customers_no_show_count_check,
      DROP COLUMN IF EXISTS completed_count,
      DROP COLUMN IF EXISTS last_no_show_at;
  `);
}
