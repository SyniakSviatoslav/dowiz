import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * TG-NOTIF-4 · Consent/preference-change audit log (GDPR proof) for category toggles.
 *
 * Renamed consent_log -> notification_prefs_audit per Counsel. PII-FREE (no addresses /
 * chat ids). Written in the SAME transaction as the prefs UPDATE (BR-16) so the consent
 * trail can never desync from the change. Low write volume (only on toggle), so a plain
 * (location_id, created_at) index suffices — no partitioning (unlike the high-volume
 * notification_outbox_audit).
 *
 * RLS mirrors order_ratings: FORCE + tenant_isolation on app.user_id.
 */
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE TABLE notification_prefs_audit (
      id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      target_id uuid REFERENCES owner_notification_targets(id) ON DELETE SET NULL,
      location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
      user_id uuid,
      category text NOT NULL,
      old_value boolean,
      new_value boolean,
      changed_via text NOT NULL CHECK (changed_via IN ('web','telegram')),
      created_at timestamptz NOT NULL DEFAULT now()
    );
    CREATE INDEX notification_prefs_audit_location_idx ON notification_prefs_audit(location_id, created_at);
  `);

  pgm.sql(`
    ALTER TABLE notification_prefs_audit ENABLE ROW LEVEL SECURITY;
    ALTER TABLE notification_prefs_audit FORCE ROW LEVEL SECURITY;
    CREATE POLICY tenant_isolation ON notification_prefs_audit
      USING ( location_id IN (SELECT app_member_location_ids()) );
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS notification_prefs_audit;`);
}
