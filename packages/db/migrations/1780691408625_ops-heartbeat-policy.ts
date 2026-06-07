import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE POLICY allow_ops_heartbeat_all ON public.ops_worker_heartbeat
      FOR ALL
      USING (true)
      WITH CHECK (true);
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    DROP POLICY allow_ops_heartbeat_all ON public.ops_worker_heartbeat;
  `);
}
