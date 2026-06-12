import { MigrationBuilder } from 'node-pg-migrate';

/**
 * Pre-create reconciliation.nightly queue for pg-boss.
 * Runtime role lacks DDL on pgboss schema, so new queues must be pre-created.
 */

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`SELECT pgboss.create_queue('reconciliation.nightly');`);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TABLE IF EXISTS pgboss."reconciliation.nightly";`);
}
