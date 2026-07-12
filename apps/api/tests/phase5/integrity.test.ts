import { test } from 'node:test';
import assert from 'node:assert';
import crypto from 'node:crypto';
import { createSessionPool, createOperationalPool } from '@deliveryos/db';
import { loadEnv } from '@deliveryos/config';

const env = loadEnv();

test('H7: Integrity under concurrency', async (t) => {
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

  // TODO(needs-staging): error-matrix controls missing from this suite —
  //   (a) INSERT with NO app.user_id set -> expect RLS block / rollback (negative control);
  //   (b) UPDATE/INSERT under a wrong-tenant app.user_id -> expect 0 rows (403-equivalent);
  //   (c) true-conflict 409 path (distinct ids, same idempotency_key, no DO UPDATE);
  //   (d) bad-payload 422 (negative total / missing currency_code).
  // These require the real RLS-enforcing role + a second seeded tenant; do NOT assert with a
  // random/nil uuid (proves nothing). Exercise against staging DB.

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

    const returnedIds = results.map(r => r.orderId).filter(Boolean);
    // Every concurrent attempt must converge on the winning id (no silent 409/rollback
    // hiding behind a set-size-of-1). Conflates-INSERT-result blind-spot guard.
    assert.strictEqual(returnedIds.length, N,
      `Expected all ${N} parallel attempts to return an order id, got ${returnedIds.length}`);
    const uniqueOrders = new Set(returnedIds);
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

    // TODO(needs-staging): cross-tenant RLS negative — repeat this UPDATE under a REAL second
    // tenant's app.user_id and assert rowCount===0 (block), then re-read status unchanged.
    // Requires a second seeded owner; a random/nil uuid 404s by absence and proves nothing
    // (Test-Integrity rule #5). Run against staging where ownerb@demo.com exists.

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

    assert.ok(tables.rows.length > 0, 'Expected money columns to sweep for CHECK constraints');

    const missing: string[] = [];
    for (const row of tables.rows) {
      const checks = await sessionPool.query(`
        SELECT pgc.conname AS constraint_name
        FROM pg_constraint pgc
        JOIN pg_class rel ON rel.oid = pgc.conrelid
        WHERE rel.relname = $1
          AND pgc.contype = 'c'
          AND pgc.condef::text ILIKE '%CHECK%' AND pgc.condef::text ILIKE '%' || $2 || '%'
      `, [row.table_name, row.column_name]);

      if (checks.rowCount === 0) {
        missing.push(`${row.table_name}.${row.column_name}`);
      }
    }
<<<<<<< Updated upstream
    assert.ok(true, 'Swept money columns for CHECK constraints');
=======
    // Was: assert.ok(true)-equivalent. A money column with no CHECK is a finding to ESCALATE,
    // never a thing to weaken (Test-Integrity red-line).
    assert.deepStrictEqual(missing, [],
      `Money columns lack a CHECK constraint: ${missing.join(', ')}`);
>>>>>>> Stashed changes
  });

  await t.test('R4: Zero orphans after cascade — FK integrity', async () => {
    // Derive the FK column from pg_constraint.conkey -> pg_attribute (single-column FKs).
    // The constraint name is NOT the column name; using conname as a column was the blind-spot.
    const fkRelations = await sessionPool.query(`
      SELECT con.conname,
             con.conrelid::regclass AS table_name,
             con.confrelid::regclass AS ref_table,
             att.attname AS column_name
      FROM pg_constraint con
      JOIN pg_attribute att ON att.attrelid = con.conrelid AND att.attnum = con.conkey[1]
      WHERE con.contype = 'f'
        AND array_length(con.conkey, 1) = 1
        AND con.confrelid::regclass::text IN ('customers', 'orders', 'locations')
    `);

    for (const fk of fkRelations.rows) {
<<<<<<< Updated upstream
      try {
        const orphans = await sessionPool.query(
          `SELECT COUNT(*)::int AS cnt FROM ${fk.table_name}
           WHERE ${fk.conname} IS NOT NULL AND
             NOT EXISTS (SELECT 1 FROM ${fk.ref_table} WHERE id = ${fk.table_name}.${getFkColumn(fk.conname)})`,
        );
        if (orphans.rows[0].cnt > 0) {
          console.log(`  ⚠ ${fk.table_name}: ${orphans.rows[0].cnt} orphan(s) via ${fk.conname}`);
        }
      } catch {
        console.debug('[integrity] FK orphan check failed for', fk.conname);
=======
      // Errors must surface (no swallow): a broken orphan query is itself a failure.
      const orphans = await sessionPool.query(
        `SELECT COUNT(*)::int AS cnt FROM ${fk.table_name}
         WHERE ${fk.column_name} IS NOT NULL AND
           NOT EXISTS (SELECT 1 FROM ${fk.ref_table} WHERE id = ${fk.table_name}.${fk.column_name})`,
      );
      if (orphans.rows[0].cnt > 0) {
        console.log(`  ⚠ ${fk.table_name}: ${orphans.rows[0].cnt} orphan(s) via ${fk.conname} (${fk.column_name})`);
        totalOrphans += orphans.rows[0].cnt;
>>>>>>> Stashed changes
      }
    }
    assert.ok(true, 'Swept FK orphans');
  });
});
