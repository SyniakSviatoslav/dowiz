/**
 * STAGED MIGRATION — ADR-admin-platform-authz (B4). OPERATOR places this into
 * packages/db/migrations/ at a timestamp exceeding the current max (e.g.
 * 1790000000030_platform-admins-and-audit.ts), runs it staging-first, then deploys the app code.
 *
 * Non-tenant GLOBAL tables — NO RLS (RA2-3). Protection = table GRANTs + the application gate
 * (requirePlatformAdmin). A NOBYPASSRLS role with GRANT SELECT reads every row identically to a
 * BYPASSRLS role → genuinely B3-independent (no GUC, no DEFINER fn, no FORCE-RLS).
 *
 * ORDER: the app code's requirePlatformAdmin point-read + the admin handlers' audit writes need
 * these tables. Deploying the app WITHOUT this migration makes the admin plane 503 (fail-closed —
 * the point-read errors) — safe, but the plane is dark until applied. Apply migration FIRST, then
 * deploy the code, then provision ≥2 admins via scripts/platform-admin-grant.ts (bus-factor R3).
 *
 * Bootstrap is DECOUPLED from this migration (F7): it creates tables + GRANTs ONLY, never reads
 * PLATFORM_ADMIN_BOOTSTRAP_USER_ID and never INSERTs an FK-bearing row → cannot FK-fail / FATAL the
 * deploy. 0 admins = safe fail-closed, recoverable via the ops CLI.
 */
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    -- platform_admins: the allowlist. Non-tenant GLOBAL table — NO RLS.
    -- 'active' is revoked_at IS NULL (no separate status column).
    CREATE TABLE platform_admins (
      user_id     uuid PRIMARY KEY REFERENCES users(id),
      granted_by  uuid,                       -- another platform_admin's user_id, or NULL for bootstrap
      granted_at  timestamptz NOT NULL DEFAULT now(),
      revoked_at  timestamptz                 -- NULL = active; non-NULL = revoked
    );
    CREATE INDEX platform_admins_active_idx ON platform_admins(user_id) WHERE revoked_at IS NULL;

    -- platform_admin_audit_log: append-only actor trail (mirror courier_audit_log). Non-tenant — NO RLS.
    -- 'status' supports the WRITE-AHEAD INTENT pattern (F5/RA2-4): a 'started' row is committed in its
    -- OWN short tx BEFORE any destructive drill, then UPDATEd to 'completed'/'failed'. Read-only
    -- endpoints write a single 'completed' row.
    CREATE TABLE platform_admin_audit_log (
      id              bigserial PRIMARY KEY,
      actor_id        uuid NOT NULL,
      action          text NOT NULL,
      target          text,
      status          text NOT NULL DEFAULT 'completed'
                        CHECK (status IN ('started','completed','failed')),
      metadata        jsonb NOT NULL DEFAULT '{}'::jsonb,
      ip_hash         text,
      user_agent_hash text,
      created_at      timestamptz NOT NULL DEFAULT now()
    );
    CREATE INDEX platform_admin_audit_actor_idx ON platform_admin_audit_log(actor_id, created_at DESC);

    -- Privilege posture: table GRANTs, NOT RLS (RA2-3).
    REVOKE ALL   ON TABLE platform_admins          FROM PUBLIC;
    REVOKE ALL   ON TABLE platform_admin_audit_log FROM PUBLIC;

    -- Operational role (NOBYPASSRLS): SELECT the allowlist (NO write → self-serve escalation is
    -- structurally impossible from any API path); append+read audit.
    GRANT SELECT          ON TABLE platform_admins          TO deliveryos_operational_user;
    GRANT SELECT, INSERT  ON TABLE platform_admin_audit_log TO deliveryos_operational_user;
    GRANT USAGE, SELECT   ON SEQUENCE platform_admin_audit_log_id_seq TO deliveryos_operational_user;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  // Additive + reversible. Safe rollback is NEVER reverting the app to requireRole(['owner']) (the bug).
  pgm.sql(`
    DROP TABLE IF EXISTS platform_admin_audit_log;
    DROP TABLE IF EXISTS platform_admins;
  `);
}
