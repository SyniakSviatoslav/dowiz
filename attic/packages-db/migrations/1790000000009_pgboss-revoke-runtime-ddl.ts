import { MigrationBuilder } from 'node-pg-migrate';

/**
 * NX-2 · Revoke runtime DDL on pgboss schema · Narrow DML-only grants
 *
 * After initial pg-boss bootstrap (0006/0008), all queue tables exist.
 * Runtime role needs DML only — no CREATE, no ALTER, no DROP.
 *
 * 🔴 This migration REVOKES CREATE ON SCHEMA pgboss from PUBLIC.
 *    The create_queue() function returns early (no-op) for existing queues,
 *    so runtime DDL is never exercised under normal operation.
 *    New queues are created via deploy-step under admin role.
 *
 * 🔴 Table grants narrowed from ALL to SELECT, INSERT, UPDATE, DELETE.
 *    Default privileges already set by migration 0006.
 */

export async function up(pgm: MigrationBuilder): Promise<void> {
  // 1. Revoke runtime DDL on pgboss schema
  pgm.sql('REVOKE CREATE ON SCHEMA pgboss FROM PUBLIC;');

  // 2. Ensure USAGE is granted (may be redundant, explicit is safe)
  pgm.sql('GRANT USAGE ON SCHEMA pgboss TO PUBLIC;');

  // 3. Reset table-level grants to specific DML only
  pgm.sql(`
    DO $reset_grants$
    DECLARE
      t text;
    BEGIN
      FOR t IN SELECT table_name FROM information_schema.tables
               WHERE table_schema = 'pgboss' AND table_type = 'BASE TABLE'
      LOOP
        EXECUTE format('REVOKE ALL PRIVILEGES ON pgboss.%I FROM PUBLIC', t);
        EXECUTE format('GRANT SELECT, INSERT, UPDATE, DELETE ON pgboss.%I TO PUBLIC', t);
      END LOOP;
    END;
    $reset_grants$;
  `);

  // 4. Sequence grants remain USAGE, SELECT, UPDATE
  pgm.sql('GRANT USAGE, SELECT, UPDATE ON ALL SEQUENCES IN SCHEMA pgboss TO PUBLIC;');

  // 5. Default privileges for future tables (already set in 0006, reinforce)
  pgm.sql(`ALTER DEFAULT PRIVILEGES IN SCHEMA pgboss 
           GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO PUBLIC;`);
  pgm.sql(`ALTER DEFAULT PRIVILEGES IN SCHEMA pgboss 
           GRANT USAGE, SELECT, UPDATE ON SEQUENCES TO PUBLIC;`);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql('GRANT CREATE ON SCHEMA pgboss TO PUBLIC;');
}
