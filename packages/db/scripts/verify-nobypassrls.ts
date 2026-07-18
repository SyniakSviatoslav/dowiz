import { Client } from 'pg';

/**
 * Boot-time RLS guard (R8 / reactivation gate).
 *
 * Asserts, at process start, that the connected role is NOT BYPASSRLS and that
 * every tenant table carries FORCE ROW LEVEL SECURITY with a sound predicate.
 * FATAL-exits if either condition fails — so a reactivated stack with a
 * fail-open table (D2 R1-R6) can never serve traffic.
 *
 * Run: `DATABASE_URL=... ts-node scripts/verify-nobypassrls.ts`
 */

const TENANT_TABLES = [
  'couriers',
  'telegram_login_tokens',
  'orders',
  'order_items',
  'customers',
  'courier_sessions',
  'customer_contact_reveals',
  'notification_outbox_audit',
  'analytics_events',
  'analytics_abuse_log',
  'analytics_cwv',
  'upload_audit',
  'customer_devices',
  'backup_metadata',
  'backup_audit_log',
  'access_requests',
];

async function main(): Promise<void> {
  const url = process.env.DATABASE_URL;
  if (!url) {
    console.error('FATAL: DATABASE_URL not set');
    process.exit(1);
  }
  const client = new Client({ connectionString: url });
  await client.connect();

  const roleRes = await client.query<{ rolbypassrls: boolean }>(
    `SELECT rolbypassrls FROM pg_roles WHERE rolname = current_user`,
  );
  if (roleRes.rows.length === 0) {
    console.error('FATAL: connected role not found in pg_roles');
    process.exit(1);
  }
  if (roleRes.rows[0].rolbypassrls) {
    console.error('FATAL: connected role has BYPASSRLS — RLS is inert, tenant isolation rests on app code only');
    process.exit(1);
  }

  const failOpen: string[] = [];
  for (const table of TENANT_TABLES) {
    const r = await client.query<{ relforcerowsecurity: boolean }>(
      `SELECT relforcerowsecurity FROM pg_class
       WHERE relname = $1 AND relnamespace = 'public'::regnamespace`,
      [table],
    );
    const forced = r.rows[0]?.relforcerowsecurity ?? false;
    const hasPolicy = (
      await client.query(
        `SELECT 1 FROM pg_policy
         WHERE schemaname = 'public' AND tablename = $1
           AND cmd = 'ALL' AND permissive = 'PERMISSIVE' AND qual IS NOT NULL`,
        [table],
      )
    ).rowCount;
    if (!forced || !hasPolicy) failOpen.push(table);
  }

  await client.end();

  if (failOpen.length > 0) {
    console.error(`FATAL: tables without FORCE RLS + sound predicate: ${failOpen.join(', ')}`);
    process.exit(1);
  }
  console.log('OK: connected role NOBYPASSRLS + all tenant tables FORCE RLS with sound predicates');
}

main().catch((e) => {
  console.error('FATAL:', e.message);
  process.exit(1);
});
