import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    -- Orders: client IP hash (sha256(ip || daily_salt)), NOT raw IP
    ALTER TABLE orders
      ADD COLUMN client_ip_hash text CHECK (
        client_ip_hash IS NULL
        OR client_ip_hash ~ '^[a-f0-9]{64}$'
      );
    COMMENT ON COLUMN orders.client_ip_hash IS 'sha256(ip || daily_salt). Raw IP NEVER stored. Salt rotates daily via app layer.';

    -- Locations: OTP toggle (owner-controlled, off by default = zero friction)
    ALTER TABLE locations
      ADD COLUMN require_phone_otp boolean NOT NULL DEFAULT false;
    COMMENT ON COLUMN locations.require_phone_otp IS 'Owner toggle. Off by default. When on, customers must verify phone via OTP before order placement.';

    -- Phone OTP seam (runtime minimal - full verify in E26)
    CREATE TABLE phone_otp (
      id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      phone       text NOT NULL,
      code_hash   text NOT NULL,
      expires_at  timestamptz NOT NULL,
      attempts    int NOT NULL DEFAULT 0 CHECK (attempts >= 0),
      consumed_at timestamptz,
      created_at  timestamptz NOT NULL DEFAULT now()
    );

    CREATE INDEX phone_otp_lookup_idx ON phone_otp(location_id, phone, expires_at DESC);
    CREATE INDEX phone_otp_active_idx ON phone_otp(location_id, expires_at) WHERE consumed_at IS NULL;

    COMMENT ON TABLE phone_otp IS 'OTP seam. Off by default. Full verify logic in E26. Raw code NEVER stored.';
    COMMENT ON COLUMN phone_otp.code_hash IS 'argon2id hash; raw code NEVER stored in DB';

    -- RLS + FORCE
    ALTER TABLE phone_otp ENABLE ROW LEVEL SECURITY;
    ALTER TABLE phone_otp FORCE ROW LEVEL SECURITY;
    CREATE POLICY phone_otp_tenant_isolation ON phone_otp
      USING (location_id IN (SELECT app_member_location_ids()))
      WITH CHECK (location_id IN (SELECT app_member_location_ids()));

    -- Trigger guard: immutability of code_hash
    CREATE OR REPLACE FUNCTION phone_otp_code_immutable() RETURNS trigger AS $$
    BEGIN
      IF OLD.code_hash IS DISTINCT FROM NEW.code_hash THEN
        RAISE EXCEPTION 'code_hash is immutable after insert';
      END IF;
      RETURN NEW;
    END;
    $$ LANGUAGE plpgsql;

    CREATE TRIGGER phone_otp_code_immutable_trg
      BEFORE UPDATE ON phone_otp
      FOR EACH ROW EXECUTE FUNCTION phone_otp_code_immutable();
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TRIGGER IF EXISTS phone_otp_code_immutable_trg ON phone_otp;
    DROP FUNCTION IF EXISTS phone_otp_code_immutable();
    DROP TABLE IF EXISTS phone_otp;
    ALTER TABLE locations DROP COLUMN IF EXISTS require_phone_otp;
    ALTER TABLE orders DROP COLUMN IF EXISTS client_ip_hash;
  `);
}
