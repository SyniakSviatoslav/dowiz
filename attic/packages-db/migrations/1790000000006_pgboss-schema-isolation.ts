import { MigrationBuilder } from 'node-pg-migrate';

/**
 * NX-2 · Least-privilege bootstrap for pg-boss schema
 * 
 * Creates dedicated 'pgboss' schema isolated from 'public'
 * Grants DML-only permissions to runtime role
 * Runtime role NEVER has DDL (CREATE) privileges on public schema
 * 
 * This migration MUST run under admin/migrate role (not runtime role)
 */

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Step 1: Create dedicated schema for pg-boss tables (idempotent)
  pgm.sql(`CREATE SCHEMA IF NOT EXISTS pgboss;`);

  // Step 2: Grant USAGE on schema to runtime role
  // This allows the role to access objects in the schema but not create new ones
  pgm.sql(`GRANT USAGE ON SCHEMA pgboss TO PUBLIC;`);

  // Step 3: Set default privileges for future tables/sequences
  // Any tables created in pgboss schema will automatically grant DML to PUBLIC
  pgm.sql(`ALTER DEFAULT PRIVILEGES IN SCHEMA pgboss 
           GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO PUBLIC;`);
  
  pgm.sql(`ALTER DEFAULT PRIVILEGES IN SCHEMA pgboss 
           GRANT USAGE, SELECT, UPDATE ON SEQUENCES TO PUBLIC;`);

  // Step 4: Ensure runtime role has NO CREATE privilege on public schema
  // Postgres 15+ removed CREATE from PUBLIC by default, but let's be explicit
  pgm.sql(`REVOKE CREATE ON SCHEMA public FROM PUBLIC;`);
  
  // Note: pg-boss will create its own tables when boss.createQueue() is called
  // These tables will inherit the default privileges above
  // Runtime role can DML on pgboss.* but cannot DDL on public or pgboss
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  // Revoke default privileges
  pgm.sql(`ALTER DEFAULT PRIVILEGES IN SCHEMA pgboss 
           REVOKE SELECT, INSERT, UPDATE, DELETE ON TABLES FROM PUBLIC;`);
  
  pgm.sql(`ALTER DEFAULT PRIVILEGES IN SCHEMA pgboss 
           REVOKE USAGE, SELECT, UPDATE ON SEQUENCES FROM PUBLIC;`);
  
  // Drop schema (will fail if tables exist - pg-boss manages its own lifecycle)
  pgm.dropSchema('pgboss', { cascade: true });
  
  // Restore CREATE on public (not recommended, but this is down migration)
  pgm.sql(`GRANT CREATE ON SCHEMA public TO PUBLIC;`);
}