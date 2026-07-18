import { Client } from 'pg';

/**
 * Staging RED→GREEN probe for the NOBYPASSRLS flip (P8-NOBYPASSRLS-FLAG).
 *
 * Creates a throwaway NOBYPASSRLS role with NO tenant GUC set, then reads the
 * two credential/account-takeover tables (couriers, telegram_login_tokens).
 * If RLS is correctly enforced, the probe sees 0 rows (GREEN). If any table is
 * fail-open, the probe sees cross-tenant rows → FATAL (RED, flip must not ship).
 *
 * Run on STAGING ONLY: `DATABASE_URL=... ts-node scripts/staging-probe-nobypassrls.ts`
 */

const PROBE_ROLE = 'nobypass_probe';
const PROBE_PASSWORD = 'nobypass-probe-rotate-me';

async function main(): Promise<void> {
  const url = process.env.DATABASE_URL;
  if (!url) {
    console.error('FATAL: DATABASE_URL not set');
    process.exit(1);
  }
  const admin = new Client({ connectionString: url });
  await admin.connect();

  await admin.query(`DROP ROLE IF EXISTS ${PROBE_ROLE}`);
  await admin.query(`CREATE ROLE ${PROBE_ROLE} NOBYPASSRLS LOGIN PASSWORD '${PROBE_PASSWORD}'`);
  // grant minimal read so the probe exercises RLS, not a permission error
  await admin.query(`GRANT SELECT ON couriers, telegram_login_tokens TO ${PROBE_ROLE}`);

  const probe = new Client({
    connectionString: url,
    user: PROBE_ROLE,
    password: PROBE_PASSWORD,
  });
  await probe.connect();
  // deliberately do NOT call set_config('app.user_id' | 'app.current_tenant')

  const couriers = await probe.query(`SELECT count(*)::int AS n FROM couriers`);
  const tokens = await probe.query(`SELECT count(*)::int AS n FROM telegram_login_tokens`);

  await probe.end();
  await admin.query(`DROP ROLE IF EXISTS ${PROBE_ROLE}`);
  await admin.end();

  const crossTenant = couriers.rows[0].n + tokens.rows[0].n;
  if (crossTenant > 0) {
    console.error(
      `FATAL: NOBYPASSRLS probe read ${crossTenant} cross-tenant rows (couriers=${couriers.rows[0].n}, tokens=${tokens.rows[0].n}) — RLS not enforced`,
    );
    process.exit(1);
  }
  console.log('OK: NOBYPASSRLS probe returns 0 cross-tenant rows (RLS enforced, flip GREEN)');
}

main().catch((e) => {
  console.error('FATAL:', e.message);
  process.exit(1);
});
