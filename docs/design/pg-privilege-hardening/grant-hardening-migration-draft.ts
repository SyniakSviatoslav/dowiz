// B3 remediation — GRANT HARDENING (from the 2026-06-30 policy/role security re-audit). Operator places into
// packages/db/migrations/. These close attack surface that NOBYPASSRLS alone does NOT (grants + TRUNCATE are
// not governed by RLS). Safe + flip-independent: the app uses only SELECT/INSERT/UPDATE/DELETE (kept); it never
// writes platform_admins (verified — managed out-of-band) nor uses TRUNCATE/TRIGGER/REFERENCES. Idempotent.
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DO $$
    BEGIN
      -- HIGH-2: the request-serving op role must NOT be able to write the platform-admin allowlist
      -- (platform_admins has RLS off → grants alone govern; a write path = self-promotion to platform admin).
      -- Keep SELECT (the root onRequest gate reads it). Audit log left writable (append-only audit path).
      BEGIN EXECUTE 'REVOKE INSERT, UPDATE, DELETE, TRUNCATE ON platform_admins FROM dowiz_app'; EXCEPTION WHEN OTHERS THEN END;

      -- MEDIUM-1: strip GRANT-ALL residue. TRUNCATE is NOT subject to RLS (NOBYPASSRLS can't constrain it) → a
      -- compromised/injected op pool could wipe any tenant's table. TRIGGER/REFERENCES are unused by the app
      -- (schema is migration-only). Keep SELECT/INSERT/UPDATE/DELETE (the app's DML).
      BEGIN EXECUTE 'REVOKE TRUNCATE, TRIGGER, REFERENCES ON ALL TABLES IN SCHEMA public FROM dowiz_app'; EXCEPTION WHEN OTHERS THEN END;
    END $$;

    -- LOW-3: FORCE-RLS parity. These carry RLS but not FORCE; harmless today (postgres owns the tables, so RLS
    -- still applies to the non-owner dowiz_app), but latent if ownership ever moves. Assert FORCE for parity.
    ALTER TABLE owner_notification_targets FORCE ROW LEVEL SECURITY;
    ALTER TABLE telegram_connect_tokens   FORCE ROW LEVEL SECURITY;
    ALTER TABLE import_sessions           FORCE ROW LEVEL SECURITY;
    ALTER TABLE order_routes              FORCE ROW LEVEL SECURITY;
    ALTER TABLE theme_versions            FORCE ROW LEVEL SECURITY;
  `);
}

export async function down(): Promise<void> {
  // Forward-only. (Re-granting TRUNCATE/platform_admins-write would re-open the surface — never auto-reverse.)
}
