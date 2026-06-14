import type { MigrationBuilder } from 'node-pg-migrate';

// Creates a dedicated non-superuser role for the operational pool.
// The operational pool previously used the 'postgres' superuser role,
// which bypasses RLS (BYPASSRLS). This migration:
//   1. Creates deliveryos_operational_user with LOGIN and NOBYPASSRLS
//   2. Grants only the DML privileges needed for operational queries
//   3. Revokes CREATE on public schema (defense-in-depth)
//
// After deploying, update ***REDACTED*** in .env / Fly secrets
// to use this role instead of 'postgres'.

export async function up(pgm: MigrationBuilder): Promise<void> {
  // Only create if not exists (idempotent)
  pgm.sql(`
    DO $$
    BEGIN
      IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'deliveryos_operational_user') THEN
        CREATE ROLE deliveryos_operational_user WITH LOGIN NOBYPASSRLS INHERIT;
      END IF;
    END
    $$;
  `);

  // Grant connect on the database
  pgm.sql(`GRANT CONNECT ON DATABASE ${process.env.PG_DATABASE || 'deliveryos'} TO deliveryos_operational_user;`);

  // Grant usage on schema
  pgm.sql(`GRANT USAGE ON SCHEMA public TO deliveryos_operational_user;`);

  // Grant SELECT on all tenant tables (operational queries are read-only)
  pgm.sql(`
    GRANT SELECT ON ALL TABLES IN SCHEMA public TO deliveryos_operational_user;
  `);

  // Grant SELECT on pgboss schema tables (monitoring)
  pgm.sql(`
    GRANT USAGE ON SCHEMA pgboss TO deliveryos_operational_user;
    GRANT SELECT ON ALL TABLES IN SCHEMA pgboss TO deliveryos_operational_user;
  `);

  // Default privileges for future tables
  pgm.sql(`
    ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA public
    GRANT SELECT ON TABLES TO deliveryos_operational_user;
  `);

  // Revoke DDL on public schema
  pgm.sql(`REVOKE CREATE ON SCHEMA public FROM deliveryos_operational_user;`);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP ROLE IF EXISTS deliveryos_operational_user;`);
}
