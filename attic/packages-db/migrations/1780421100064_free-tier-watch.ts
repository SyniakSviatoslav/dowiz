import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE IF NOT EXISTS free_tier_snapshots (
      id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      db_size_bytes   bigint NOT NULL,
      storage_bytes   bigint NOT NULL DEFAULT 0,
      active_connections integer NOT NULL DEFAULT 0,
      egress_estimate_bytes bigint NOT NULL DEFAULT 0,
      db_pct          numeric(5,2) NOT NULL,
      storage_pct     numeric(5,2) NOT NULL DEFAULT 0,
      connections_pct numeric(5,2) NOT NULL DEFAULT 0,
      egress_pct      numeric(5,2) NOT NULL DEFAULT 0,
      status          text NOT NULL DEFAULT 'ok' CHECK (status IN ('ok', 'warning', 'critical')),
      created_at      timestamptz NOT NULL DEFAULT now()
    );

    CREATE INDEX IF NOT EXISTS idx_free_tier_snapshots_created_at
      ON free_tier_snapshots (created_at DESC);

    CREATE INDEX IF NOT EXISTS idx_free_tier_snapshots_status
      ON free_tier_snapshots (status);

    COMMENT ON TABLE free_tier_snapshots IS
      'P5-5: Periodic snapshots of Free-tier resource usage. Used by free-tier-watch worker and health endpoint.';
    COMMENT ON COLUMN free_tier_snapshots.db_pct IS
      'Percentage of 500 MB Free-tier DB limit used.';
    COMMENT ON COLUMN free_tier_snapshots.storage_pct IS
      'Percentage of 1 GB Free-tier storage limit used.';
    COMMENT ON COLUMN free_tier_snapshots.connections_pct IS
      'Percentage of ~15 Free-tier connection limit used.';
    COMMENT ON COLUMN free_tier_snapshots.egress_pct IS
      'Percentage of 2 GB/month Free-tier egress limit used (estimate).';
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE IF EXISTS free_tier_snapshots CASCADE;
  `);
}
