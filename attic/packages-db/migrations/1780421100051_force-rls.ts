import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE courier_locations FORCE ROW LEVEL SECURITY;
    ALTER TABLE courier_invites FORCE ROW LEVEL SECURITY;
    ALTER TABLE courier_assignments FORCE ROW LEVEL SECURITY;
    ALTER TABLE courier_shifts FORCE ROW LEVEL SECURITY;
    ALTER TABLE courier_positions FORCE ROW LEVEL SECURITY;
    ALTER TABLE courier_audit_log FORCE ROW LEVEL SECURITY;
    ALTER TABLE courier_payouts FORCE ROW LEVEL SECURITY;
    ALTER TABLE settlement_items FORCE ROW LEVEL SECURITY;
    ALTER TABLE settlement_audit_log FORCE ROW LEVEL SECURITY;
    ALTER TABLE courier_dispatch_queue FORCE ROW LEVEL SECURITY;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    ALTER TABLE courier_locations NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE courier_invites NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE courier_assignments NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE courier_shifts NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE courier_positions NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE courier_audit_log NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE courier_payouts NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE settlement_items NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE settlement_audit_log NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE courier_dispatch_queue NO FORCE ROW LEVEL SECURITY;
  `);
}
