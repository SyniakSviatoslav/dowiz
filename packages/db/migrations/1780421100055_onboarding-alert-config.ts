import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    -- Onboarding state (JSONB, versioned)
    ALTER TABLE locations
      ADD COLUMN onboarding_state jsonb NOT NULL DEFAULT '{}'::jsonb
        CHECK (jsonb_typeof(onboarding_state) = 'object'),
      ADD COLUMN onboarding_completed_at timestamptz;

    -- Dwell thresholds (JSONB, versioned, owner-configurable)
    ALTER TABLE locations
      ADD COLUMN dwell_thresholds jsonb NOT NULL DEFAULT '{"v":1,"pending_s":60,"confirmed_s":300,"preparing_s":600,"en_route_s":900}'::jsonb
        CHECK (jsonb_typeof(dwell_thresholds) = 'object');

    -- Partial index for active onboarding (used by E29)
    CREATE INDEX locations_active_onboarding_idx ON locations(id)
      WHERE onboarding_completed_at IS NULL AND onboarding_state != '{}'::jsonb;

    COMMENT ON COLUMN locations.onboarding_state IS 'Self-serve onboarding progress. JSONB {v, step, completedSteps[], skippedSteps[], data{}}. Saved between sessions. Full flow in E29.';
    COMMENT ON COLUMN locations.onboarding_completed_at IS 'NULL = in progress. Set on go-live (E29 step 8).';
    COMMENT ON COLUMN locations.dwell_thresholds IS 'Per-state dwell timeout in seconds. Owner-configurable. E25 monitor checks these. Default: pending=60s, confirmed=300s, preparing=600s, en_route=900s.';
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP INDEX IF EXISTS locations_active_onboarding_idx;
    ALTER TABLE locations
      DROP COLUMN IF EXISTS onboarding_state,
      DROP COLUMN IF EXISTS onboarding_completed_at,
      DROP COLUMN IF EXISTS dwell_thresholds;
  `);
}
