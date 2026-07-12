import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    -- Add anonymization columns
    ALTER TABLE customers ADD COLUMN IF NOT EXISTS anonymized_at timestamptz;
    ALTER TABLE orders ADD COLUMN IF NOT EXISTS anonymized_at timestamptz;
    ALTER TABLE locations ADD COLUMN IF NOT EXISTS retention_days integer NOT NULL DEFAULT 365
      CHECK (retention_days >= 30 AND retention_days <= 2555);

    -- GDPR erasure requests table
    CREATE TABLE IF NOT EXISTS gdpr_erasure_requests (
      id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id    uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      customer_id    uuid REFERENCES customers(id) ON DELETE SET NULL,
      subject_phone  text,
      reason         text,
      status         text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'in_progress', 'completed', 'failed')),
      requested_at   timestamptz NOT NULL DEFAULT now(),
      requested_by_owner_id uuid REFERENCES users(id),
      completed_at   timestamptz,
      error_message  text,
      metadata       jsonb NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(metadata) = 'object')
    );

    -- GDPR erasure indexes
    CREATE UNIQUE INDEX IF NOT EXISTS gdpr_dedup_per_customer ON gdpr_erasure_requests(location_id, customer_id)
      WHERE status IN ('pending', 'in_progress', 'completed') AND customer_id IS NOT NULL;
    CREATE INDEX IF NOT EXISTS gdpr_pending_idx ON gdpr_erasure_requests(location_id) WHERE status = 'pending';

    -- Anonymization audit log table
    CREATE TABLE IF NOT EXISTS anonymization_audit_log (
      id         bigserial PRIMARY KEY,
      scope      text NOT NULL CHECK (scope IN ('retention', 'gdpr')),
      subject_kind text NOT NULL CHECK (subject_kind IN ('customer', 'order')),
      subject_id uuid NOT NULL,
      location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      actor_kind text NOT NULL CHECK (actor_kind IN ('system', 'owner', 'customer')),
      actor_id   uuid,
      metadata   jsonb NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(metadata) = 'object'),
      created_at timestamptz NOT NULL DEFAULT now()
    );
    CREATE INDEX IF NOT EXISTS anonymization_audit_log_location_idx ON anonymization_audit_log(location_id, created_at DESC);

    -- RLS on gdpr_erasure_requests
    ALTER TABLE gdpr_erasure_requests ENABLE ROW LEVEL SECURITY;
    ALTER TABLE gdpr_erasure_requests FORCE ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS gdpr_tenant_isolation ON gdpr_erasure_requests;
    CREATE POLICY gdpr_tenant_isolation ON gdpr_erasure_requests
      USING (location_id IN (SELECT app_member_location_ids()))
      WITH CHECK (location_id IN (SELECT app_member_location_ids()));

    -- RLS on anonymization_audit_log
    ALTER TABLE anonymization_audit_log ENABLE ROW LEVEL SECURITY;
    ALTER TABLE anonymization_audit_log FORCE ROW LEVEL SECURITY;
    DROP POLICY IF EXISTS anonymization_audit_tenant_isolation ON anonymization_audit_log;
    CREATE POLICY anonymization_audit_tenant_isolation ON anonymization_audit_log
      USING (location_id IN (SELECT app_member_location_ids()))
      WITH CHECK (location_id IN (SELECT app_member_location_ids()));

    -- Indexes for retention scanning
    CREATE INDEX IF NOT EXISTS customers_anonymized_pending_idx ON customers(id) WHERE anonymized_at IS NULL;
    CREATE INDEX IF NOT EXISTS orders_anonymized_pending_idx ON orders(id, location_id) WHERE anonymized_at IS NULL;

    -- Allow NULL phone after anonymization
    ALTER TABLE customers ALTER COLUMN phone DROP NOT NULL;

    -- Comments documenting red lines
    COMMENT ON TABLE gdpr_erasure_requests IS 'P5-0: GDPR erasure requests. Deduped per customer. Owner-only initiate. Worker processes asynchronously.';
    COMMENT ON TABLE anonymization_audit_log IS 'P5-0: Append-only audit log for anonymization. 0 PII in metadata. Retention + GDPR triggers.';
    COMMENT ON COLUMN customers.anonymized_at IS 'P5-0: Set on anonymization. IS NOT NULL → subject is anonymized, skip on re-run.';
    COMMENT ON COLUMN orders.anonymized_at IS 'P5-0: Set on anonymization. PII fields set to NULL, business fields preserved.';
    COMMENT ON COLUMN locations.retention_days IS 'P5-0: Orders/customers older than this many days are anonymized nightly. Default 365, range 30-2555.';
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE IF EXISTS anonymization_audit_log CASCADE;
    DROP TABLE IF EXISTS gdpr_erasure_requests CASCADE;
    DROP INDEX IF EXISTS customers_anonymized_pending_idx;
    DROP INDEX IF EXISTS orders_anonymized_pending_idx;
    DROP INDEX IF EXISTS gdpr_dedup_per_customer;
    DROP INDEX IF EXISTS gdpr_pending_idx;
    DROP INDEX IF EXISTS anonymization_audit_log_location_idx;
    ALTER TABLE customers ALTER COLUMN phone SET NOT NULL;
    ALTER TABLE locations DROP COLUMN IF EXISTS retention_days;
    ALTER TABLE customers DROP COLUMN IF EXISTS anonymized_at;
    ALTER TABLE orders DROP COLUMN IF EXISTS anonymized_at;
  `);
}
