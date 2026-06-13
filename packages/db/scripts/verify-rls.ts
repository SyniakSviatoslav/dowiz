import { createOperationalPool, createSessionPool } from '../src/index.js';

async function run() {
  const sessionPool = createSessionPool();
  const pool = createOperationalPool();
  try {
    // 1. Fetch the user IDs created by the seed script (using session pool which has no RLS on users).
    const resA = await sessionPool.query(`SELECT id FROM users WHERE email = 'ownera@demo.com'`);
    const resB = await sessionPool.query(`SELECT id FROM users WHERE email = 'ownerb@demo2.com'`);

    if (resA.rowCount === 0 || resB.rowCount === 0) {
      console.error('❌ Missing seeded users. Did you run `pnpm seed`?');
      process.exit(1);
    }

    const ownerAId = resA.rows[0].id;
    const ownerBId = resB.rows[0].id;

    // Check if the 'postgres' role bypasses RLS by default.
    // If we are connecting with Supabase's default postgres role over 6543, it might bypass RLS 
    // unless we SET default_transaction_isolation or use a different role.
    // But the prompt says "Передумова коректності: роль ... не суперюзер і без BYPASSRLS. FORCE робить RLS чинною для власника таблиць."
    // Actually, on Supabase, `postgres` IS a superuser (or has BYPASSRLS).
    // If the test fails, we will see it here. We will temporarily remove BYPASSRLS or assume the authenticated role if needed.
    // Let's test it directly first.

    const TENANT_TABLES = [
      'memberships',
      'modifier_groups',
      'modifiers',
      'product_modifier_groups',
      'order_item_modifiers',
      'order_status_history',
      'delivery_tiers',
      'reservations',
      'product_translations',
      'category_translations',
      // Phase 3 tables
      'courier_locations',
      'courier_invites',
      'courier_assignments',
      'courier_shifts',
      'courier_positions',
      'courier_audit_log',
      'courier_payouts',
      'settlement_items',
      'settlement_audit_log',
      'courier_dispatch_queue',
      // Phase 4 tables
      'customer_signals',
      'velocity_events',
      'customer_otp_sessions',
      'phone_otp',
      'customer_devices',
    ];

    for (const table of TENANT_TABLES) {
      const expectedOwnerB = table === 'memberships' ? 1 : 0;

      // TEST 1: No user_id set -> Deny default.
      /* eslint-disable local/no-raw-sql */
      const resNoUser = await pool.query(`SELECT count(*) as count FROM ${table}`);
      /* eslint-enable local/no-raw-sql */
      if (parseInt(resNoUser.rows[0].count, 10) > 0) {
        console.error(`❌ Isolation leak: Anonymous query returned ${resNoUser.rows[0].count} for ${table}. Expected 0.`);
        process.exit(1);
      }

      // TEST 2: Owner A
      await pool.query('BEGIN');
      await pool.query('SET LOCAL app.user_id = $1', [ownerAId]);
      /* eslint-disable local/no-raw-sql */
      const resOwnerA = await pool.query(`SELECT count(*) as count FROM ${table}`);
      /* eslint-enable local/no-raw-sql */
      await pool.query('COMMIT');

      // TEST 3: Owner B
      await pool.query('BEGIN');
      await pool.query('SET LOCAL app.user_id = $1', [ownerBId]);
      /* eslint-disable local/no-raw-sql */
      const resOwnerB = await pool.query(`SELECT count(*) as count FROM ${table}`);
      /* eslint-enable local/no-raw-sql */
      await pool.query('COMMIT');

      if (parseInt(resOwnerB.rows[0].count, 10) !== expectedOwnerB) {
        console.error(`❌ Isolation leak: Owner B saw ${resOwnerB.rows[0].count} for ${table}. Expected ${expectedOwnerB}.`);
        process.exit(1);
      }

      console.log(`✅ ${table.padEnd(25, ' ')} isolation verified (Anon: 0, OwnerA: ${resOwnerA.rows[0].count}, OwnerB: ${resOwnerB.rows[0].count})`);
    }

    console.log('🎉 All RLS empirical tests passed!');

    // ── API Exposure check: zero grants for anon/authenticated/service_role on non-tenant tables ──
    console.log('\n🔍 Checking Data API perimeter...');
    const NON_TENANT_TABLES = ['users', 'ops_worker_heartbeat', 'auth_refresh_tokens', 'exchange_rates'];

    for (const table of NON_TENANT_TABLES) {
      const grantRes = await pool.query(
        `SELECT grantee, privilege_type
         FROM information_schema.role_table_grants
         WHERE table_schema = 'public'
           AND table_name = $1
           AND grantee IN ('anon', 'authenticated', 'service_role')`,
        [table],
      );
      if (grantRes.rowCount && grantRes.rowCount > 0) {
        console.error(`❌ API exposure: ${table} has ${grantRes.rowCount} grant(s) for API roles:`);
        for (const row of grantRes.rows) {
          console.error(`   ${row.grantee}: ${row.privilege_type}`);
        }
        process.exit(1);
      }
      console.log(`✅ ${table.padEnd(25, ' ')} zero API role grants`);

      // Verify RLS is ENABLED (linter 0013 check)
      const rlsRes = await pool.query(
        `SELECT relrowsecurity, relforcerowsecurity
         FROM pg_class
         WHERE relnamespace = 'public'::regnamespace
           AND relname = $1`,
        [table],
      );
      if (rlsRes.rows.length === 0) {
        console.error(`❌ ${table}: table not found in pg_class`);
        process.exit(1);
      }
      const rls = rlsRes.rows[0];
      if (!rls.relrowsecurity) {
        console.error(`❌ ${table}: RLS NOT ENABLED`);
        process.exit(1);
      }
      console.log(`   RLS: ${rls.relrowsecurity ? 'ENABLED' : '❌'} / FORCE: ${rls.relforcerowsecurity ? 'YES' : 'NO'}`);
    }

    console.log('\n🎉 Non-tenant API perimeter locked down!');

  } catch (err) {
    console.error('❌ Verification failed:', err);
    process.exit(1);
  } finally {
    await pool.end();
    await sessionPool.end();
  }
}

run();
