import { MigrationBuilder } from 'node-pg-migrate';

/**
 * NX-2 fix: Grant CREATE on pgboss schema for pg-boss auto-migration.
 *
 * pg-boss requires DDL on its schema to create internal tables (job, schedule, archive, etc.)
 * even with migrate:false it reads pgboss.version which must exist.
 * The runtime role creates tables ONLY in pgboss schema (not public).
 *
 * 🔴 Least-privilege still holds: CREATE is scoped to pgboss schema only.
 *    Runtime role still has NO CREATE on public schema.
 */

export async function up(pgm: MigrationBuilder): Promise<void> {
  // 1. Allow runtime role to CREATE tables in pgboss (for pg-boss auto-migration)
  pgm.sql('GRANT CREATE ON SCHEMA pgboss TO PUBLIC;');

  // 2. Grant DML on ALL existing pgboss tables to runtime role
  //    Existing tables owned by postgres — default privileges don't apply
  pgm.sql(`
    DO $grant_existing$
    DECLARE
      t text;
    BEGIN
      FOR t IN SELECT table_name FROM information_schema.tables
               WHERE table_schema = 'pgboss' AND table_type = 'BASE TABLE'
      LOOP
        EXECUTE format('GRANT ALL PRIVILEGES ON pgboss.%I TO PUBLIC', t);
      END LOOP;
      EXECUTE 'GRANT USAGE, SELECT, UPDATE ON ALL SEQUENCES IN SCHEMA pgboss TO PUBLIC';
    END;
    $grant_existing$;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql('REVOKE CREATE ON SCHEMA pgboss FROM PUBLIC;');
}