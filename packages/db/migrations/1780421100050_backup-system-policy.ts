import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE POLICY "System can do everything" ON backup_metadata FOR ALL TO deliveryos_api_user USING (true) WITH CHECK (true);
    CREATE POLICY "System can do everything" ON backup_audit_log FOR ALL TO deliveryos_api_user USING (true) WITH CHECK (true);
    GRANT ALL ON backup_audit_log_id_seq TO deliveryos_api_user;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP POLICY IF EXISTS "System can do everything" ON backup_metadata;
    DROP POLICY IF EXISTS "System can do everything" ON backup_audit_log;
  `);
}
