import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE locations
      ADD COLUMN IF NOT EXISTS rate_limit_overrides jsonb NOT NULL DEFAULT '{}'::jsonb
        CHECK (jsonb_typeof(rate_limit_overrides) = 'object');

    CREATE TABLE IF NOT EXISTS upload_audit (
      id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id     uuid REFERENCES locations(id) ON DELETE SET NULL,
      uploaded_by     text NOT NULL,
      file_name       text NOT NULL,
      file_size_bytes integer NOT NULL CHECK (file_size_bytes > 0),
      mime_type       text NOT NULL,
      file_hash       text NOT NULL,
      status          text NOT NULL DEFAULT 'accepted' CHECK (status IN ('accepted', 'rejected')),
      rejection_reason text,
      created_at      timestamptz NOT NULL DEFAULT now()
    );

    COMMENT ON COLUMN locations.rate_limit_overrides IS
      'P5-4: Per-location rate-limit overrides. Keys: orders_per_min, inflight_per_tenant. Empty = env defaults.';
    COMMENT ON TABLE upload_audit IS
      'P5-4: Audit log for uploaded files (menu imports, photos). MIME + hash + size validated before accept.';
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE locations DROP COLUMN IF EXISTS rate_limit_overrides;
    DROP TABLE IF EXISTS upload_audit CASCADE;
  `);
}
