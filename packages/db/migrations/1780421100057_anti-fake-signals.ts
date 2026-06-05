import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    -- Customer signals (advisory, human-in-loop)
    CREATE TABLE IF NOT EXISTS customer_signals (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      customer_id uuid NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
      location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      kind text NOT NULL CHECK (kind IN (
        'no_show_recent',
        'velocity_rapid',
        'velocity_high_volume',
        'ip_velocity_rapid',
        'ip_velocity_high_volume',
        'manual_flag'
      )),
      severity text NOT NULL CHECK (severity IN ('low', 'medium', 'high')),
      evidence jsonb NOT NULL DEFAULT '{}'::jsonb,
      raised_at timestamptz NOT NULL DEFAULT now(),
      acknowledged_at timestamptz,
      acknowledged_by_owner_id uuid REFERENCES users(id),
      dismissed_at timestamptz,
      dismissed_by_owner_id uuid REFERENCES users(id),
      expires_at timestamptz NOT NULL DEFAULT (now() + interval '7 days')
    );

    CREATE INDEX IF NOT EXISTS customer_signals_customer_idx
      ON customer_signals(customer_id, raised_at DESC);
    CREATE INDEX IF NOT EXISTS customer_signals_active_idx
      ON customer_signals(location_id, raised_at DESC)
      WHERE acknowledged_at IS NULL AND dismissed_at IS NULL;
    CREATE INDEX IF NOT EXISTS customer_signals_cleanup_idx
      ON customer_signals(expires_at);

    ALTER TABLE customer_signals ENABLE ROW LEVEL SECURITY;
    ALTER TABLE customer_signals FORCE ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS customer_signals_tenant_isolation ON customer_signals;
    CREATE POLICY customer_signals_tenant_isolation ON customer_signals
      USING (location_id IN (SELECT app_member_location_ids()))
      WITH CHECK (location_id IN (SELECT app_member_location_ids()));

    -- Velocity events (24h retention, PII-free)
    CREATE TABLE IF NOT EXISTS velocity_events (
      id bigserial PRIMARY KEY,
      location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      phone_hash text,
      client_ip_hash text,
      kind text NOT NULL DEFAULT 'order_placed' CHECK (kind IN ('order_placed', 'order_cancelled')),
      window_started_at timestamptz NOT NULL DEFAULT now(),
      CHECK (phone_hash IS NOT NULL OR client_ip_hash IS NOT NULL),
      CHECK (phone_hash IS NULL OR phone_hash ~ '^[a-f0-9]{64}$'),
      CHECK (client_ip_hash IS NULL OR client_ip_hash ~ '^[a-f0-9]{64}$')
    );

    CREATE INDEX IF NOT EXISTS velocity_events_phone_window_idx
      ON velocity_events(location_id, phone_hash, window_started_at DESC)
      WHERE phone_hash IS NOT NULL;
    CREATE INDEX IF NOT EXISTS velocity_events_ip_window_idx
      ON velocity_events(location_id, client_ip_hash, window_started_at DESC)
      WHERE client_ip_hash IS NOT NULL;
    CREATE INDEX IF NOT EXISTS velocity_events_cleanup_idx
      ON velocity_events(window_started_at);

    ALTER TABLE velocity_events ENABLE ROW LEVEL SECURITY;
    ALTER TABLE velocity_events FORCE ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS velocity_events_tenant_isolation ON velocity_events;
    CREATE POLICY velocity_events_tenant_isolation ON velocity_events
      USING (location_id IN (SELECT app_member_location_ids()))
      WITH CHECK (location_id IN (SELECT app_member_location_ids()));

    -- Customer OTP sessions (short-lived, order-scoped)
    CREATE TABLE IF NOT EXISTS customer_otp_sessions (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      customer_id uuid REFERENCES customers(id),
      location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      phone_hash text NOT NULL,
      purpose text NOT NULL CHECK (purpose IN ('otp_verified')),
      token_hash text NOT NULL,
      order_intent_hash text,
      expires_at timestamptz NOT NULL,
      consumed_at timestamptz,
      created_at timestamptz NOT NULL DEFAULT now()
    );

    CREATE INDEX IF NOT EXISTS customer_otp_sessions_token_idx
      ON customer_otp_sessions(token_hash)
      WHERE consumed_at IS NULL;
    CREATE INDEX IF NOT EXISTS customer_otp_sessions_cleanup_idx
      ON customer_otp_sessions(expires_at);

    ALTER TABLE customer_otp_sessions ENABLE ROW LEVEL SECURITY;
    ALTER TABLE customer_otp_sessions FORCE ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS customer_otp_sessions_tenant_isolation ON customer_otp_sessions;
    CREATE POLICY customer_otp_sessions_tenant_isolation ON customer_otp_sessions
      USING (location_id IN (SELECT app_member_location_ids()))
      WITH CHECK (location_id IN (SELECT app_member_location_ids()));

    -- Comments documenting red lines
    -- Orders metadata column for P26 signal/OTP data
    ALTER TABLE orders ADD COLUMN IF NOT EXISTS metadata jsonb NOT NULL DEFAULT '{}'::jsonb
      CHECK (jsonb_typeof(metadata) = 'object');

    COMMENT ON TABLE customer_signals IS 'Advisory signals for human review. NEVER used for auto-block. Decay over time. Owner acknowledge/dismiss only.';
    COMMENT ON TABLE velocity_events IS 'Privacy-first velocity counters. phone_hash and client_ip_hash — never raw. 24h retention via cron cleanup.';
    COMMENT ON TABLE customer_otp_sessions IS 'Short-lived (15min) tokens for OTP verification. Single-use, order-scoped. NOT a session token.';
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE IF EXISTS customer_otp_sessions CASCADE;
    DROP TABLE IF EXISTS velocity_events CASCADE;
    DROP TABLE IF EXISTS customer_signals CASCADE;
  `);
}
