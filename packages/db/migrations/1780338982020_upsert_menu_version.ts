import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE OR REPLACE FUNCTION upsert_menu_version(p_location_id uuid)
    RETURNS void
    SECURITY DEFINER
    LANGUAGE plpgsql
    AS $$
    BEGIN
      INSERT INTO menu_versions (location_id, version)
      VALUES (p_location_id, 1)
      ON CONFLICT (location_id)
      DO UPDATE SET
        version = menu_versions.version + 1,
        updated_at = now();
    END;
    $$;
  `);
}

export async function down(): Promise<void> {}
