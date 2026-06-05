import { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DO
    $$
    BEGIN
      EXECUTE 'ALTER ROLE deliveryos_api_user BYPASSRLS';
    EXCEPTION WHEN OTHERS THEN
      -- Ignore if not allowed or doesn't exist
    END
    $$;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DO
    $$
    BEGIN
      EXECUTE 'ALTER ROLE deliveryos_api_user NOBYPASSRLS';
    EXCEPTION WHEN OTHERS THEN
    END
    $$;
  `);
}
