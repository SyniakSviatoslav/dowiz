import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE settlement_audit_log (
      id bigserial PRIMARY KEY,
      payout_id uuid NOT NULL REFERENCES courier_payouts(id) ON DELETE CASCADE,
      action text NOT NULL CHECK (action IN ('generated', 'approved', 'paid', 'disputed', 'reopened', 'item_added', 'item_voided')),
      actor_kind text NOT NULL CHECK (actor_kind IN ('owner', 'courier', 'system')),
      actor_id uuid,
      metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
      created_at timestamptz NOT NULL DEFAULT now()
    );

    CREATE INDEX settlement_audit_log_payout_idx ON settlement_audit_log(payout_id, created_at DESC);

    ALTER TABLE settlement_audit_log ENABLE ROW LEVEL SECURITY;
    
    -- In a real scenario we might denormalize location_id for easier RLS, or join:
    -- Since payout_id has location_id, we can check that. For brevity, assuming owner can read if they can read the payout.
    -- However, we can also just rely on app logic to restrict it or add location_id.
    -- I will add location_id to make RLS fast and bulletproof.
    ALTER TABLE settlement_audit_log ADD COLUMN location_id uuid NOT NULL REFERENCES locations(id);
    
    CREATE POLICY isolate_settlement_audit_log ON settlement_audit_log
      USING (location_id = current_setting('app.current_tenant')::uuid);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP TABLE settlement_audit_log;
  `);
}
