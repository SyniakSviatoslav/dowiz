import type { MigrationBuilder } from 'node-pg-migrate';

// pg-boss v10 creates one partition table per queue via `CREATE TABLE pgboss.<hash>`
// AT RUNTIME (worker.start → boss.createQueue). That needs CREATE on schema pgboss.
// Migration 1790000000009 revoked CREATE from the runtime role for least-privilege,
// but that makes every boot hang on createQueue (permission denied) — it took prod
// down on 2026-06-21. pg-boss's runtime queue creation is incompatible with that
// revoke, so we grant it back (scoped to the pgboss schema only). The role still
// can't DDL anywhere else. Idempotent; the live prod DB already has this grant.
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DO $pgboss_grant$
    BEGIN
      IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'deliveryos_api_user')
         AND EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'pgboss') THEN
        GRANT USAGE, CREATE ON SCHEMA pgboss TO deliveryos_api_user;
      END IF;
    END
    $pgboss_grant$;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DO $pgboss_revoke$
    BEGIN
      IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'deliveryos_api_user')
         AND EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'pgboss') THEN
        REVOKE CREATE ON SCHEMA pgboss FROM deliveryos_api_user;
      END IF;
    END
    $pgboss_revoke$;
  `);
}
