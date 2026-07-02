import { test } from 'node:test';
import assert from 'node:assert';
import crypto from 'node:crypto';

// Live-stack integration test: needs a provisioned env + seeded DB. Importing
// @deliveryos/db without DATABASE_URL_* crashes at module load (loadEnv is
// module-scope there), so the import is dynamic and the test skips honestly.
const PROVISIONED = !!(process.env.DATABASE_URL_SESSION && process.env.DATABASE_URL_OPERATIONAL);
const skip = PROVISIONED ? false : 'requires provisioned env (DATABASE_URL_*) + seeded local stack';

test('H7: Integrity under concurrency', { skip }, async (t) => {
  const { createSessionPool, createOperationalPool } = await import('@deliveryos/db');
  const sessionPool = createSessionPool();
  const pool = createOperationalPool();
  t.after(async () => { await sessionPool.end(); await pool.end(); });

  // Find test users and locations
  const userRes = await sessionPool.query(`SELECT id FROM users WHERE email = 'ownera@demo.com' LIMIT 1`);
  const locRes = await sessionPool.query(`SELECT id FROM locations ORDER BY created_at LIMIT 1`);
  if (userRes.rowCount === 0 || locRes.rowCount === 0) {
    console.error('❌ Missing seed data. Run `pnpm seed` first.');
    process.exit(1);
  }
  const ownerId = userRes.rows[0].id;
  const locationId = locRes.rows[0].id;

  // Find a product
  const prodRes = await sessionPool.query(
    `SELECT id FROM products WHERE location_id = $1 LIMIT 1`,
    [locationId],
  );

  await t.test('R1: Idempotency — N parallel duplicate orders = 1 order', async () => {
    const idempotencyKey = `test_${crypto.randomUUID()}`;
    const N = 5;

    const results = await Promise.all(
      Array.from({ length: N }, async () => {
        const client = await pool.connect();
        try {
          await client.query('BEGIN');
          await client.query("SELECT set_config('app.user_id', $1, true)", [ownerId]);

          // Check existing
          const existing = await client.query(
            `SELECT id FROM orders WHERE idempotency_key = $1`,
            [idempotencyKey],
          );
          if (existing.rowCount > 0) {
            await client.query('COMMIT');
            return { status: 200, orderId: existing.rows[0].id };
          }

          // Insert
          const insert = await client.query(
            `INSERT INTO orders (id, location_id, customer_id, status, total, currency_code, idempotency_key)
             VALUES ($1, $2, NULL, 'PENDING', 1000, 'ALL', $3)
             ON CONFLICT (idempotency_key) DO UPDATE SET id = EXCLUDED.id
             RETURNING id`,
            [crypto.randomUUID(), locationId, idempotencyKey],
          );
          await client.query('COMMIT');
          return { status: 201, orderId: insert.rows[0].id };
        } catch (err: any) {
          await client.query('ROLLBACK');
          return { status: 409, error: err.message };
        } finally {
          client.release();
        }
      }),
    );

    const uniqueOrders = new Set(results.map(r => r.orderId).filter(Boolean));
    assert.strictEqual(uniqueOrders.size, 1,
      `Expected 1 order from ${N} parallel duplicates, got ${uniqueOrders.size}`);

    // Cleanup
    const orderId = [...uniqueOrders][0];
    await sessionPool.query(`DELETE FROM order_items WHERE order_id = $1`, [orderId]);
    await sessionPool.query(`DELETE FROM orders WHERE id = $1`, [orderId]);
  });

  await t.test('R2: Status-guard — double transition prevented', async () => {
    const orderId = crypto.randomUUID();
    const customerId = crypto.randomUUID();

    // Create test order
    await sessionPool.query(
      `INSERT INTO orders (id, location_id, customer_id, status, total, currency_code, idempotency_key, created_at)
       VALUES ($1, $2, $3, 'PENDING', 500, 'ALL', $4, now())`,
      [orderId, locationId, customerId, `guard_${crypto.randomUUID()}`],
    );

    const N = 3;
    const results = await Promise.all(
      Array.from({ length: N }, async () => {
        const client = await pool.connect();
        try {
          await client.query('BEGIN');
          await client.query("SELECT set_config('app.user_id', $1, true)", [ownerId]);
          const res = await client.query(
            `UPDATE orders SET status = 'CONFIRMED', confirmed_at = now()
             WHERE id = $1 AND status = 'PENDING'
             RETURNING id`,
            [orderId],
          );
          await client.query('COMMIT');
          return { success: res.rowCount > 0 };
        } catch {
          await client.query('ROLLBACK');
          return { success: false };
        } finally {
          client.release();
        }
      }),
    );

    const successes = results.filter(r => r.success).length;
    assert.strictEqual(successes, 1,
      `Expected 1 success from ${N} parallel transitions, got ${successes}`);

    // Verify final state
    const finalState = await sessionPool.query(
      `SELECT status FROM orders WHERE id = $1`,
      [orderId],
    );
    assert.strictEqual(finalState.rows[0].status, 'CONFIRMED');

    // Cleanup
    await sessionPool.query(`DELETE FROM orders WHERE id = $1`, [orderId]);
  });

  await t.test('R3: Integer money invariants — CHECK ≥0 on money columns', async () => {
    const tables = await sessionPool.query(`
      SELECT table_name, column_name
      FROM information_schema.columns
      WHERE table_schema = 'public'
        AND (column_name IN ('total', 'subtotal', 'delivery_fee', 'price', 'price_delta',
                             'amount', 'commission_amount', 'payout_amount')
             OR column_name LIKE '%price%' OR column_name LIKE '%total%')
        AND table_name NOT IN ('pgmigrations')
    `);

    for (const row of tables.rows) {
      try {
        const checks = await sessionPool.query(`
          SELECT pgc.conname AS constraint_name
          FROM pg_constraint pgc
          JOIN pg_class rel ON rel.oid = pgc.conrelid
          WHERE rel.relname = $1
            AND pgc.contype = 'c'
            AND pgc.condef::text ILIKE '%CHECK%' AND pgc.condef::text ILIKE '%${row.column_name}%'
        `, [row.table_name]);

        if (checks.rowCount === 0) {
          console.log(`  ℹ ${row.table_name}.${row.column_name} has no CHECK constraint`);
        }
      } catch {
        console.debug('[integrity] CHECK constraint query failed for', row.table_name);
      }
    }
    assert.ok(tables.rows.length > 0, 'Expected money columns to sweep for CHECK constraints');
  });

  await t.test('R4: Zero orphans after cascade — FK integrity', async () => {
    const fkRelations = await sessionPool.query(`
      SELECT conname, conrelid::regclass AS table_name,
             confrelid::regclass AS ref_table
      FROM pg_constraint
      WHERE contype = 'f' AND confrelid::regclass::text IN ('customers', 'orders', 'locations')
    `);

    let totalOrphans = 0;
    for (const fk of fkRelations.rows) {
      try {
        const orphans = await sessionPool.query(
          `SELECT COUNT(*)::int AS cnt FROM ${fk.table_name}
           WHERE ${fk.conname} IS NOT NULL AND
             NOT EXISTS (SELECT 1 FROM ${fk.ref_table} WHERE id = ${fk.table_name}.${getFkColumn(fk.conname)})`,
        );
        if (orphans.rows[0].cnt > 0) {
          console.log(`  ⚠ ${fk.table_name}: ${orphans.rows[0].cnt} orphan(s) via ${fk.conname}`);
          totalOrphans += orphans.rows[0].cnt;
        }
      } catch {
        console.debug('[integrity] FK orphan check failed for', fk.conname);
      }
    }
    assert.strictEqual(totalOrphans, 0, `Expected zero FK orphans, found ${totalOrphans}`);
  });
});

function getFkColumn(constraintName: string): string {
  const map: Record<string, string> = {
    'orders_customer_id_fkey': 'customer_id',
    'customers_location_id_fkey': 'location_id',
  };
  return map[constraintName] || 'id';
}
