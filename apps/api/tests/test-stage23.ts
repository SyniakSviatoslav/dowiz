import { test } from 'node:test';
import assert from 'node:assert';
import crypto from 'node:crypto';
import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';

const env = loadEnv();

test('Stage 23: Phase 4 Seam Migrations', async (t) => {
  const pool = createSessionPool();

  // Generate unique test IDs to avoid collisions
  const orgId = crypto.randomUUID();
  const locId = crypto.randomUUID();
  const custId = crypto.randomUUID();
  const userId = crypto.randomUUID();

  await t.test('setup test data', async () => {
    await pool.query(
      `INSERT INTO users (id, email) VALUES ($1, $2) ON CONFLICT DO NOTHING`,
      [userId, `test-p23-${Date.now()}@test.com`]
    );
    await pool.query(
      `INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'P23 Org', $2) ON CONFLICT DO NOTHING`,
      [orgId, userId]
    );
    await pool.query(
      `INSERT INTO locations (id, org_id, slug, name, phone, status, confirm_timeout_min)
       VALUES ($1, $2, $3, 'P23 Loc', '123', 'open', 1) ON CONFLICT DO NOTHING`,
      [locId, orgId, `p23-loc-${Date.now()}`]
    );
    await pool.query(
      `INSERT INTO customers (id, location_id, phone, name) VALUES ($1, $2, $3, 'P23 Cust') ON CONFLICT DO NOTHING`,
      [custId, locId, `+355${Date.now()}`]
    );
  });

  await t.test('customers.no_show_count CHECK >= 0', async () => {
    const { rows } = await pool.query(
      `SELECT no_show_count, completed_count, last_no_show_at FROM customers WHERE id = $1`,
      [custId]
    );
    assert.strictEqual(rows[0].no_show_count, 0);
    assert.strictEqual(rows[0].completed_count, 0);
    assert.strictEqual(rows[0].last_no_show_at, null);

    // Verify CHECK constraint rejects negative
    await assert.rejects(
      () => pool.query(`UPDATE customers SET no_show_count = -1 WHERE id = $1`, [custId]),
      { code: '23514' }
    );
    await assert.rejects(
      () => pool.query(`UPDATE customers SET completed_count = -5 WHERE id = $1`, [custId]),
      { code: '23514' }
    );
  });

  await t.test('orders.client_ip_hash format constraint', async () => {
    const orderId = crypto.randomUUID();
    await pool.query(
      `INSERT INTO orders (id, location_id, subtotal, total, request_hash, type, status, payment_method, payment_outcome)
       VALUES ($1, $2, 100, 100, 'p23-test', 'delivery', 'PENDING', 'cash', 'pending')`,
      [orderId, locId]
    );

    // Valid hash (64 hex chars)
    const validHash = 'a'.repeat(64);
    await pool.query(
      `UPDATE orders SET client_ip_hash = $1 WHERE id = $2`,
      [validHash, orderId]
    );
    const { rows } = await pool.query(
      `SELECT client_ip_hash FROM orders WHERE id = $1`,
      [orderId]
    );
    assert.strictEqual(rows[0].client_ip_hash, validHash);

    // Invalid hash (too short)
    await assert.rejects(
      () => pool.query(`UPDATE orders SET client_ip_hash = 'not-hash' WHERE id = $1`, [orderId]),
      { code: '23514' }
    );
  });

  await t.test('orders has no raw client_ip column', async () => {
    const { rows } = await pool.query(`
      SELECT column_name FROM information_schema.columns
      WHERE table_name = 'orders' AND column_name = 'client_ip'
    `);
    assert.strictEqual(rows.length, 0, 'Raw client_ip column must not exist');
  });

  await t.test('locations.new columns default values', async () => {
    const { rows } = await pool.query(
      `SELECT require_phone_otp, onboarding_state, dwell_thresholds, onboarding_completed_at
       FROM locations WHERE id = $1`,
      [locId]
    );
    assert.strictEqual(rows[0].require_phone_otp, false);
    assert.strictEqual(rows[0].onboarding_completed_at, null);
    const state = rows[0].onboarding_state;
    assert.strictEqual(typeof state, 'object');
    assert.strictEqual(Object.keys(state).length, 0);
    const thresholds = rows[0].dwell_thresholds;
    assert.strictEqual(thresholds.v, 1);
    assert.strictEqual(thresholds.pending_s, 60);
    assert.strictEqual(thresholds.confirmed_s, 300);
    assert.strictEqual(thresholds.preparing_s, 600);
    assert.strictEqual(thresholds.en_route_s, 900);
  });

  await t.test('onboarding_state JSONB typeof check', async () => {
    await assert.rejects(
      () => pool.query(`UPDATE locations SET onboarding_state = '[]'::jsonb WHERE id = $1`, [locId]),
      { code: '23514' }
    );
    await assert.rejects(
      () => pool.query(`UPDATE locations SET onboarding_state = '"string"'::jsonb WHERE id = $1`, [locId]),
      { code: '23514' }
    );
    await pool.query(
      `UPDATE locations SET onboarding_state = '{"v":1,"step":2}'::jsonb WHERE id = $1`,
      [locId]
    );
  });

  await t.test('dwell_thresholds JSONB typeof check', async () => {
    await assert.rejects(
      () => pool.query(`UPDATE locations SET dwell_thresholds = '[]'::jsonb WHERE id = $1`, [locId]),
      { code: '23514' }
    );
    await assert.rejects(
      () => pool.query(`UPDATE locations SET dwell_thresholds = '"invalid"'::jsonb WHERE id = $1`, [locId]),
      { code: '23514' }
    );
  });

  await t.test('phone_otp RLS tenant isolation', async () => {
    const otherLocId = crypto.randomUUID();
    const otpId = crypto.randomUUID();
    const client = await pool.connect();

    try {
      // Insert OTP under our test location
      await client.query('BEGIN');
      await client.query(`SET LOCAL app.location_id = '${locId}'`);
      await client.query(
        `INSERT INTO phone_otp (id, location_id, phone, code_hash, expires_at)
         VALUES ($1, $2, '+355123', 'test-hash', now() + interval '5 minutes')`,
        [otpId, locId]
      );
      await client.query('COMMIT');

      // Without SET LOCAL -> 0 rows (FORCE RLS)
      const { rows: anonRows } = await client.query('SELECT * FROM phone_otp');
      assert.strictEqual(anonRows.length, 0, 'Anonymous query must return 0 rows');

      // With correct tenant -> rows visible
      await client.query('BEGIN');
      await client.query(`SET LOCAL app.location_id = '${locId}'`);
      const { rows: tenantRows } = await client.query('SELECT * FROM phone_otp');
      await client.query('COMMIT');
      assert.ok(tenantRows.length > 0, 'Tenant query must return rows');

      // With wrong tenant -> 0 rows
      await client.query('BEGIN');
      await client.query(`SET LOCAL app.location_id = '${otherLocId}'`);
      const { rows: wrongRows } = await client.query('SELECT * FROM phone_otp');
      await client.query('COMMIT');
      assert.strictEqual(wrongRows.length, 0, 'Cross-tenant query must return 0 rows');

      // Cross-tenant INSERT blocked by WITH CHECK
      await client.query('BEGIN');
      await client.query(`SET LOCAL app.location_id = '${otherLocId}'`);
      await assert.rejects(
        () => client.query(
          `INSERT INTO phone_otp (id, location_id, phone, code_hash, expires_at)
           VALUES ($1, $2, '+355999', 'other-hash', now() + interval '5 minutes')`,
          [crypto.randomUUID(), locId]
        ),
        { code: '23505' }
      );
      await client.query('ROLLBACK');
    } finally {
      client.release();
    }
  });

  await t.test('phone_otp code_hash immutability', async () => {
    const otpId = crypto.randomUUID();
    const client = await pool.connect();

    try {
      // Insert OTP within tenant context
      await client.query('BEGIN');
      await client.query(`SET LOCAL app.location_id = '${locId}'`);
      await client.query(
        `INSERT INTO phone_otp (id, location_id, phone, code_hash, expires_at)
         VALUES ($1, $2, '+355456', 'original-hash', now() + interval '5 minutes')`,
        [otpId, locId]
      );
      await client.query('COMMIT');

      // UPDATE code_hash -> exception (immutable trigger)
      await assert.rejects(
        () => client.query('UPDATE phone_otp SET code_hash = $1 WHERE id = $2', ['new-hash', otpId]),
        { code: 'P0001' }
      );

      // UPDATE attempts -> allowed (mutable field)
      await client.query('UPDATE phone_otp SET attempts = attempts + 1 WHERE id = $1', [otpId]);
      const { rows } = await client.query('SELECT attempts FROM phone_otp WHERE id = $1', [otpId]);
      assert.strictEqual(rows[0].attempts, 1);
    } finally {
      client.release();
    }
  });

  await t.test('index usage: customers_no_show_idx', async () => {
    const { rows } = await pool.query(`
      SELECT indexname FROM pg_indexes
      WHERE tablename = 'customers' AND indexname = 'customers_no_show_idx'
    `);
    assert.strictEqual(rows.length, 1);
  });

  await t.test('index usage: phone_otp_lookup_idx', async () => {
    const { rows } = await pool.query(`
      SELECT indexname FROM pg_indexes
      WHERE tablename = 'phone_otp' AND indexname = 'phone_otp_lookup_idx'
    `);
    assert.strictEqual(rows.length, 1);
  });

  await t.test('index usage: locations_active_onboarding_idx', async () => {
    const { rows } = await pool.query(`
      SELECT indexname FROM pg_indexes
      WHERE tablename = 'locations' AND indexname = 'locations_active_onboarding_idx'
    `);
    assert.strictEqual(rows.length, 1);
  });

  await t.test('columns have COMMENTS', async () => {
    const { rows: noShowComment } = await pool.query(`
      SELECT col_description('customers'::regclass, ordinal_position) AS comment
      FROM information_schema.columns
      WHERE table_name = 'customers' AND column_name = 'no_show_count'
    `);
    assert.ok(noShowComment[0].comment?.length > 0, 'no_show_count needs COMMENT');

    const { rows: ipHashComment } = await pool.query(`
      SELECT col_description('orders'::regclass, ordinal_position) AS comment
      FROM information_schema.columns
      WHERE table_name = 'orders' AND column_name = 'client_ip_hash'
    `);
    assert.ok(ipHashComment[0].comment?.includes('sha256'), 'client_ip_hash COMMENT must mention sha256');
  });
});
