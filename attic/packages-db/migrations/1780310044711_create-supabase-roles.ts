import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * P0-ROLES · Idempotently create the Supabase-convention roles the later
 * migrations reference in GRANT / REVOKE / CREATE POLICY ... TO clauses:
 *   authenticated, anon, service_role, deliveryos_api_user
 *
 * On Supabase these roles already exist (created by the platform), so this is a
 * no-op there. On bare Postgres (fresh from-scratch provisioning) they do not
 * exist, and the first migration that says e.g. `... TO authenticated`
 * (1780338982030_theme_versions) would fail with "role ... does not exist".
 *
 * `CREATE ROLE` has no `IF NOT EXISTS`, so we use a DO block that swallows
 * duplicate_object. Roles are created NOLOGIN (no password) — they exist only
 * as grant targets / RLS policy roles, never as connection identities. This is
 * safe on Supabase because we never ALTER an existing role's attributes.
 *
 * Placed immediately after 1780310044710_extensions-and-enums and before any
 * migration that references these roles.
 */

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DO $create_roles$
    BEGIN
      BEGIN
        CREATE ROLE authenticated NOLOGIN;
      EXCEPTION WHEN duplicate_object THEN NULL;
      END;

      BEGIN
        CREATE ROLE anon NOLOGIN;
      EXCEPTION WHEN duplicate_object THEN NULL;
      END;

      BEGIN
        CREATE ROLE service_role NOLOGIN;
      EXCEPTION WHEN duplicate_object THEN NULL;
      END;

      BEGIN
        CREATE ROLE deliveryos_api_user NOLOGIN;
      EXCEPTION WHEN duplicate_object THEN NULL;
      END;
    END;
    $create_roles$;
  `);
}

export async function down(): Promise<void> {
  // No-op: dropping shared cluster roles is unsafe (other objects/grants may
  // depend on them, and on Supabase they are platform-owned). Leave intact.
}
