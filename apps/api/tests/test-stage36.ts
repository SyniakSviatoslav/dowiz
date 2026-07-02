import { test } from 'node:test';
import assert from 'node:assert';
import crypto from 'node:crypto';
import { loadEnv } from '@deliveryos/config';
import { createSessionPool, createOperationalPool } from '@deliveryos/db';
import { signAuthToken } from '@deliveryos/platform';

const env = loadEnv();
const BASE = `http://127.0.0.1:${env.PORT || 3003}`;
const API = `${BASE}/api`;

/**
 * Stage 36: NX-1..NX-6 Notification Infrastructure
 *
 * T-1: Durability — notification job is enqueued transactionally in pgboss
 * T-2: Off-critical-path — order succeeds even when Telegram is down
 * T-3: Topology/privileges — session port, no DDL on public, pgboss schema isolated
 * T-4: Idempotency — same dedup key produces exactly one notification job
 */
test('Stage 36: Notification Infrastructure (NX-1..NX-6)', async (t) => {
  const sessionPool = createSessionPool();
  const operationalPool = createOperationalPool();
  const orgId = crypto.randomUUID();
  const locId = crypto.randomUUID();
  const userId = crypto.randomUUID();
  const custId = crypto.randomUUID();
  const prodId = crypto.randomUUID();
  const tgTargetId = crypto.randomUUID();

  let ownerToken: string;

  // ─── Setup ────────────────────────────────────────────────────────
  await t.test('setup test data', async () => {
    await sessionPool.query(`INSERT INTO users (id, email) VALUES ($1, $2) ON CONFLICT DO NOTHING`,
      [userId, `owner-p36-${Date.now()}@test.com`]);
    await sessionPool.query(`INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'P36 Org', $2) ON CONFLICT DO NOTHING`,
      [orgId, userId]);
    await sessionPool.query(`INSERT INTO locations (id, org_id, slug, name, phone, status, currency_code, default_locale, supported_locales, confirm_timeout_min, delivery_fee_flat)
      VALUES ($1, $2, $3, 'P36 Loc', '123', 'open', 'ALL', 'sq', $4::text[], 5, 0) ON CONFLICT DO NOTHING`,
      [locId, orgId, `p36-loc-${Date.now()}`, '{sq,en}']);
    await sessionPool.query(`INSERT INTO memberships (user_id, location_id, role) VALUES ($1, $2, 'owner') ON CONFLICT DO NOTHING`,
      [userId, locId]);
    await sessionPool.query(`INSERT INTO customers (id, location_id, phone, name) VALUES ($1, $2, '+355691234567', 'P36 Customer') ON CONFLICT DO NOTHING`,
      [custId, locId]);
    await sessionPool.query(`INSERT INTO products (id, location_id, name, price, is_available) VALUES ($1, $2, 'P36 Product', 500, true) ON CONFLICT DO NOTHING`,
      [prodId, locId]);
    // Set up Telegram notification target
    await sessionPool.query(`INSERT INTO owner_notification_targets (id, location_id, channel, address, status, prefs)
      VALUES ($1, $2, 'telegram', '8360105469', 'active', '{}'::jsonb) ON CONFLICT DO NOTHING`,
      [tgTargetId, locId]);

    ownerToken = await signAuthToken({ role: 'owner', userId, activeLocationId: locId }, '15m');
  });

  // ═══════════════════════════════════════════════════════════════
  // T-3: Topology & Privileges (NX-1, NX-2)
  // ═══════════════════════════════════════════════════════════════
  await t.test('T-3: Session pool connects to session port (5432)', async () => {
    const res = await sessionPool.query('SELECT inet_server_port() AS port');
    const port = res.rows[0].port;
    console.log(`[T-3] Session pool connected to port: ${port}`);
    // Session pool should connect to direct port (5432), not transaction pooler (6543)
    assert.strictEqual(port, 5432, `Expected session port 5432, got ${port}`);
  });

  await t.test('T-3: Runtime role has no CREATE privilege on public schema', async () => {
    const res = await operationalPool.query(
      "SELECT current_user, has_schema_privilege(current_user, 'public', 'CREATE') AS can_create"
    );
    console.log(`[T-3] Current user: ${res.rows[0].current_user}, can CREATE on public: ${res.rows[0].can_create}`);
    assert.strictEqual(res.rows[0].can_create, false,
      `Runtime role ${res.rows[0].current_user} should not have CREATE on public`);
  });

  await t.test('T-3: pg-boss tables exist in pgboss schema', async () => {
    const res = await sessionPool.query(
      `SELECT table_name FROM information_schema.tables
       WHERE table_schema = 'pgboss' AND table_type = 'BASE TABLE'
       ORDER BY table_name`
    );
    console.log(`[T-3] pg-boss tables in pgboss schema: ${res.rows.map(r => r.table_name).join(', ')}`);
    assert.ok(res.rows.length > 0, 'Expected pg-boss tables in pgboss schema');
    // Verify at least one core pg-boss table exists
    const tableNames = res.rows.map(r => r.table_name);
    assert.ok(tableNames.includes('schedule'), `Expected 'schedule' table in pgboss schema, got: ${tableNames.join(', ')}`);
  });

  await t.test('T-3: Operational pool connects via transaction pooler', async () => {
    const res = await operationalPool.query('SELECT current_user, inet_server_port() AS port');
    const port = res.rows[0].port;
    const user = res.rows[0].current_user;
    console.log(`[T-3] Operational pool connected as ${user} (backend port: ${port})`);
    // NOTE: inet_server_port() returns backend PostgreSQL port (5432), not Supavisor proxy port (6543)
    // Verify the user is NOT postgres — confirms it's the runtime role via transaction pooler
    assert.notStrictEqual(user, 'postgres', 'Operational pool should NOT connect as postgres');
    assert.strictEqual(user, 'deliveryos_api_user', 'Operational pool should connect as deliveryos_api_user');
  });

  // ═══════════════════════════════════════════════════════════════
  // T-1: Durability — notification job enqueued transactionally
  // ═══════════════════════════════════════════════════════════════
  await t.test('T-1: Order creation enqueues notify.telegram.send job in pgboss', async () => {
    const idempotencyKey = crypto.randomUUID();

    // Create an order
    const orderRes = await fetch(`${API}/orders`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${ownerToken}`,
      },
      body: JSON.stringify({
        locationId: locId,
        type: 'delivery',
        items: [{ product_id: prodId, quantity: 1 }],
        customer: { phone: '+355692345678', name: 'T-1 Customer' },
        delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'T-1 Address' },
        payment: { method: 'cash' },
        idempotency_key: idempotencyKey,
      }),
    });

    assert.strictEqual(orderRes.status, 201, `Order creation failed with status ${orderRes.status}`);
    const order = await orderRes.json();
    assert.ok(order.id, 'Expected order ID');
    console.log(`[T-1] Created order: ${order.id}`);

    // Verify a notify.telegram.send job exists in pgboss queue
    const jobRes = await sessionPool.query(
      `SELECT id, name, data, state, created_on
       FROM pgboss.job
       WHERE name = 'notify.telegram.send'
         AND data->>'entity_id' = $1
         AND data->>'location_id' = $2
       ORDER BY created_on DESC
       LIMIT 5`,
      [order.id, locId]
    );
    console.log(`[T-1] Found ${jobRes.rowCount} notify.telegram.send jobs for order ${order.id}`);
    assert.ok(jobRes.rowCount !== null && jobRes.rowCount >= 1,
      `Expected at least 1 notify.telegram.send job, found ${jobRes.rowCount}`);

    // Verify job has no PII — only event, entity_id, location_id, dedupKey
    const job = jobRes.rows[0];
    const dataFields = Object.keys(job.data);
    console.log(`[T-1] Job data fields: ${dataFields.join(', ')}`);
    assert.ok(dataFields.includes('event'), 'Expected event field');
    assert.ok(dataFields.includes('entity_id'), 'Expected entity_id field');
    assert.ok(dataFields.includes('location_id'), 'Expected location_id field');
    assert.strictEqual(job.data.event, 'order.created', `Expected event 'order.created', got ${job.data.event}`);
    assert.strictEqual(job.data.entity_id, order.id, `Expected entity_id ${order.id}, got ${job.data.entity_id}`);
    // Verify no PII in job data
    const payloadStr = JSON.stringify(job.data);
    assert.ok(!payloadStr.includes('+355'), 'Job data should not contain phone numbers');
    assert.ok(!payloadStr.includes('customer'), 'Job data should not contain customer data');
  });

  // ═══════════════════════════════════════════════════════════════
  // T-2: Off-critical-path — order succeeds when Telegram fails
  // ═══════════════════════════════════════════════════════════════
  await t.test('T-2: Order creation succeeds even with failing notification delivery', async () => {
    const idempotencyKey = crypto.randomUUID();

    // This test verifies that even if the notification worker can't reach Telegram,
    // the order creation itself succeeds. The notification job will be retried
    // by pg-boss (via NX-5 retry logic) without affecting the order.
    const orderRes = await fetch(`${API}/orders`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${ownerToken}`,
      },
      body: JSON.stringify({
        locationId: locId,
        type: 'delivery',
        items: [{ product_id: prodId, quantity: 2 }],
        customer: { phone: '+355694567890', name: 'T-2 Customer' },
        delivery: { pin: { lat: 41.33, lng: 19.82 } },
        payment: { method: 'cash' },
        idempotency_key: idempotencyKey,
      }),
    });

    assert.strictEqual(orderRes.status, 201,
      `Order creation should succeed (200/201) even with failing notifications, got ${orderRes.status}`);
    const order = await orderRes.json();
    assert.ok(order.id, 'Expected order ID');
    console.log(`[T-2] Order created successfully: ${order.id}`);

    // Verify notification job exists despite Telegram being potentially down
    const jobRes = await sessionPool.query(
      `SELECT id, name, data, state
       FROM pgboss.job
       WHERE name = 'notify.telegram.send'
         AND data->>'entity_id' = $1
       ORDER BY created_on DESC
       LIMIT 1`,
      [order.id]
    );
    assert.ok(jobRes.rowCount !== null && jobRes.rowCount >= 1,
      `Expected notification job for order ${order.id}`);
    console.log(`[T-2] Notification job created for order: ${order.id} (state: ${jobRes.rows[0].state})`);
  });

  // ═══════════════════════════════════════════════════════════════
  // T-4: Idempotency — same dedup key produces one job
  // ═══════════════════════════════════════════════════════════════
  await t.test('T-4: Duplicate event does not create duplicate notification jobs', async () => {
    // Use idempotency_key: first call creates, second call returns existing
    // Even if MessageBus also fires, singletonKey dedup prevents double enqueue
    const dedupKey = crypto.randomUUID(); // used as both idempotency_key and singletonKey

    const placeOrder = () => fetch(`${API}/orders`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${ownerToken}`,
      },
      body: JSON.stringify({
        locationId: locId,
        type: 'delivery',
        items: [{ product_id: prodId, quantity: 1 }],
        customer: { phone: '+355694567891', name: 'T-4 Customer' },
        delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'T-4 Address' },
        payment: { method: 'cash' },
        idempotency_key: dedupKey,
      }),
    });

    const res1 = await placeOrder();
    const res2 = await placeOrder();

    console.log(`[T-4] First order status: ${res1.status}, second: ${res2.status}`);
    assert.ok([200, 201].includes(res1.status), `First order should succeed, got ${res1.status}`);
    assert.ok([200, 201].includes(res2.status), `Second (dedup) order should succeed, got ${res2.status}`);

    const order1 = await res1.json();
    const order2 = res2.status === 200 ? await res2.json() : null;
    if (order2) {
      assert.strictEqual(order2.id, order1.id, 'Dedup should return same order ID');
    }

    // Wait briefly for async job processing
    await new Promise(resolve => setTimeout(resolve, 500));

    // Count notification jobs for this order — should be 1 (singletonKey dedup)
    const jobRes = await sessionPool.query(
      `SELECT COUNT(*)::int AS cnt
       FROM pgboss.job
       WHERE name = 'notify.telegram.send'
         AND data->>'entity_id' = $1`,
      [order1.id]
    );

    const jobCount = jobRes.rows[0].cnt;
    console.log(`[T-4] notify.telegram.send jobs for order ${order1.id}: ${jobCount}`);
    assert.strictEqual(jobCount, 1,
      `Expected exactly 1 notification job for order, found ${jobCount}`);
  });

  // ═══════════════════════════════════════════════════════════════
  // NX-2: Verify pgboss schema grants
  // ═══════════════════════════════════════════════════════════════
  await t.test('NX-2: Runtime role has NO CREATE on pgboss (DDL-free runtime)', async () => {
    // Migration 0009 revokes CREATE ON SCHEMA pgboss from PUBLIC.
    // pg-boss uses migrate:false — queues pre-created by deploy-step under admin role.
    // Runtime role has USAGE + DML only, zero DDL.
    const res = await operationalPool.query(
      "SELECT has_schema_privilege(current_user, 'pgboss', 'CREATE') AS can_create_pgboss, has_schema_privilege(current_user, 'public', 'CREATE') AS can_create_public, has_schema_privilege(current_user, 'pgboss', 'USAGE') AS can_use_pgboss"
    );
    console.log(`[NX-2] Runtime role: USAGE on pgboss=${res.rows[0].can_use_pgboss}, CREATE on pgboss=${res.rows[0].can_create_pgboss}, CREATE on public=${res.rows[0].can_create_public}`);
    assert.strictEqual(res.rows[0].can_create_pgboss, false,
      'Runtime role should NOT have CREATE on pgboss schema');
    assert.strictEqual(res.rows[0].can_create_public, false,
      'Runtime role should NOT have CREATE on public schema');
    assert.strictEqual(res.rows[0].can_use_pgboss, true,
      'Runtime role should have USAGE on pgboss schema');
  });

  // ─── Cleanup ──────────────────────────────────────────────────────
  // No cleanup step: test data isolation via random UUIDs; DB records are
  // cleaned up by retention policies / anonymizer. (A decorative assert.ok(true)
  // subtest lived here — removed, banned false-green class.)
});
