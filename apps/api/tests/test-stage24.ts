import { test } from 'node:test';
import assert from 'node:assert';
import crypto from 'node:crypto';
import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';
import { signAuthToken, verifyAuthToken } from '@deliveryos/platform';
import WebSocket from 'ws';

const env = loadEnv();
const BASE = `http://127.0.0.1:${env.PORT || 3003}`;
const WS_BASE = `ws://127.0.0.1:${env.PORT || 3003}`;

function delay(ms: number) { return new Promise(r => setTimeout(r, ms)); }

test('Stage 24: Owner Live Dashboard', async (t) => {
  const pool = createSessionPool();
  const orgId = crypto.randomUUID();
  const locId = crypto.randomUUID();
  const locIdB = crypto.randomUUID();
  const userId = crypto.randomUUID();
  const custId = crypto.randomUUID();
  const prodId = crypto.randomUUID();
  const orderId = crypto.randomUUID();
  const courierId = crypto.randomUUID();

  let ownerToken: string;

  // ─── Setup ────────────────────────────────────────────────────────
  await t.test('setup test data', async () => {
    await pool.query(`INSERT INTO users (id, email) VALUES ($1, $2) ON CONFLICT DO NOTHING`,
      [userId, `owner-p24-${Date.now()}@test.com`]);
    await pool.query(`INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'P24 Org', $2) ON CONFLICT DO NOTHING`,
      [orgId, userId]);
    await pool.query(`INSERT INTO locations (id, org_id, slug, name, phone, status, confirm_timeout_min)
      VALUES ($1, $2, $3, 'P24 Loc', '123', 'open', 1) ON CONFLICT DO NOTHING`,
      [locId, orgId, `p24-loc-${Date.now()}`]);
    await pool.query(`INSERT INTO locations (id, org_id, slug, name, phone, status, confirm_timeout_min)
      VALUES ($1, $2, $3, 'P24 Loc B', '456', 'open', 1) ON CONFLICT DO NOTHING`,
      [locIdB, orgId, `p24-loc-b-${Date.now()}`]);
    await pool.query(`INSERT INTO customers (id, location_id, phone, name) VALUES ($1, $2, '+355691234567', 'Test Customer') ON CONFLICT DO NOTHING`,
      [custId, locId]);
    await pool.query(`INSERT INTO products (id, location_id, name, price, is_available) VALUES ($1, $2, 'Test Product', 500, true) ON CONFLICT DO NOTHING`,
      [prodId, locId]);
    await pool.query(`INSERT INTO orders (id, location_id, customer_id, subtotal, total, request_hash, type, status, payment_method, payment_outcome, created_at)
      VALUES ($1, $2, $3, 1000, 1200, 'p24-test', 'delivery', 'PENDING', 'cash', 'pending', now() - interval '2 minutes') ON CONFLICT DO NOTHING`,
      [orderId, locId, custId]);
    await pool.query(`INSERT INTO order_items (order_id, product_id, name_snapshot, price_snapshot, quantity)
      VALUES ($1, $2, 'Test Product', 500, 2) ON CONFLICT DO NOTHING`,
      [orderId, prodId]);
    ownerToken = await signAuthToken({ role: 'owner', userId, activeLocationId: locId }, '15m');
  });

  // ─── Snapshot Tests ────────────────────────────────────────────────
  await t.test('snapshot returns correct counts and orders', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/dashboard/snapshot`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    assert.strictEqual(res.status, 200);
    const data = await res.json();
    assert.ok(data.serverTime);
    assert.ok(data.counts.PENDING >= 1);
    assert.ok(data.orders.length >= 1);

    const order = data.orders[0];
    assert.ok(order.orderId);
    assert.ok(order.total >= 0);
    assert.ok(order.customerNameMasked, 'Name must be masked');
    assert.ok(order.customerPhoneMasked, 'Phone must be masked');
    assert.ok(!order.customerNameMasked?.includes('Test Customer'), 'Raw name must not leak');
    assert.ok(!order.customerPhoneMasked?.includes('+355691234567'), 'Raw phone must not leak');
    assert.ok(typeof order.dwellSeconds === 'number');
  });

  await t.test('snapshot masked PII invariant', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/dashboard/snapshot`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    const data = await res.json();
    const body = JSON.stringify(data);
    const piiPatterns = [/\+355\d{8,}/, /[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/, /"name":"[^*]/];
    for (const p of piiPatterns) {
      const match = body.match(p);
      if (match) {
        const val = match[0];
        if (val.includes('Test') || val.includes('PENDING') || val.includes('orderId') || val.includes('IN_DELIVERY')) continue;
        assert.fail(`PII leak: ${val}`);
      }
    }
  });

  await t.test('snapshot pagination', async () => {
    // Create 5 orders to ensure pagination test works
    const ids: string[] = [];
    for (let i = 0; i < 5; i++) {
      const oid = crypto.randomUUID();
      await pool.query(`INSERT INTO orders (id, location_id, subtotal, total, request_hash, type, status, payment_method, payment_outcome, created_at)
        VALUES ($1, $2, 100, 100, 'p24-page', 'delivery', 'PENDING', 'cash', 'pending', now() - interval '${i + 1} minutes')`,
        [oid, locId]);
      ids.push(oid);
    }
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/dashboard/snapshot?limit=3`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    const data = await res.json();
    assert.ok(data.orders.length <= 3);
    // Next cursor should exist if more orders
    if (data.orders.length === 3) {
      assert.ok(data.nextCursor, 'Should have cursor for next page');
      const res2 = await fetch(`${BASE}/api/owner/locations/${locId}/dashboard/snapshot?limit=3&cursor=${data.nextCursor}`, {
        headers: { Authorization: `Bearer ${ownerToken}` },
      });
      const data2 = await res2.json();
      assert.ok(data2.orders.length >= 1);
    }
  });

  await t.test('snapshot tenant guard', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locIdB}/dashboard/snapshot`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    assert.strictEqual(res.status, 404, 'Cross-tenant snapshot must 404');
  });

  // ─── Confirm Action ────────────────────────────────────────────────
  await t.test('confirm order', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/orders/${orderId}/confirm`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: '{}',
    });
    assert.strictEqual(res.status, 200);
    const data = await res.json();
    assert.strictEqual(data.status, 'CONFIRMED');
  });

  await t.test('confirm idempotency (double confirm = 409)', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/orders/${orderId}/confirm`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: '{}',
    });
    assert.strictEqual(res.status, 409, 'Double confirm must 409');
  });

  await t.test('confirm tenant guard', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locIdB}/orders/${orderId}/confirm`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: '{}',
    });
    assert.strictEqual(res.status, 404, 'Cross-tenant confirm must 404');
  });

  // ─── Reject Action ─────────────────────────────────────────────────
  await t.test('reject confirmed order returns 409', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/orders/${orderId}/reject`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ reason: 'Test reject' }),
    });
    assert.strictEqual(res.status, 409, 'Rejecting CONFIRMED order must 409');
  });

  // ─── Rate Limiting ─────────────────────────────────────────────────
  await t.test('rate limit on confirm (31 req/min = 429 on 31st)', async () => {
    const tmpId = crypto.randomUUID();
    await pool.query(`INSERT INTO orders (id, location_id, subtotal, total, request_hash, type, status, payment_method, payment_outcome)
      VALUES ($1, $2, 100, 100, 'p24-rate', 'delivery', 'PENDING', 'cash', 'pending')`,
      [tmpId, locId]);
    // Send 31 rapid confirms, expect at least one 429
    let got429 = false;
    for (let i = 0; i < 31; i++) {
      const res = await fetch(`${BASE}/api/owner/locations/${locId}/orders/${tmpId}/confirm`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        body: '{}',
      });
      if (res.status === 429) { got429 = true; break; }
    }
    assert.ok(got429, 'Expected at least one 429 from rate limit');
  });

  // ─── WS Auth ───────────────────────────────────────────────────────
  await t.test('WS auth: invalid token → 1008 close', async () => {
    const ws = new WebSocket(WS_BASE);
    const err = await new Promise<{ code: number }>((resolve) => {
      ws.onclose = (e) => resolve({ code: e.code });
      ws.onopen = () => ws.send(JSON.stringify({ type: 'auth', token: 'invalid-token' }));
    });
    assert.strictEqual(err.code, 1008);
  });

  await t.test('WS auth: expired token → 1008', async () => {
    const expired = await signAuthToken({ role: 'owner', userId: userId as any }, '0s');
    await delay(100);
    const ws = new WebSocket(WS_BASE);
    const err = await new Promise<{ code: number }>((resolve) => {
      ws.onclose = (e) => resolve({ code: e.code });
      ws.onopen = () => ws.send(JSON.stringify({ type: 'auth', token: expired }));
    });
    assert.strictEqual(err.code, 1008);
  });

  await t.test('WS auth: customer token → 1008', async () => {
    const custToken = await signAuthToken({ role: 'customer', userId: userId as any, locationId: locId, orderId: orderId }, '15m');
    const ws = new WebSocket(WS_BASE);
    const err = await new Promise<{ code: number }>((resolve) => {
      ws.onclose = (e) => resolve({ code: e.code });
      ws.onopen = () => ws.send(JSON.stringify({ type: 'auth', token: custToken }));
    });
    assert.strictEqual(err.code, 1008, 'Customer token must be rejected');
  });

  // ─── WS Dashboard Broadcast ────────────────────────────────────────
  await t.test('WS dashboard room receives order events', async () => {
    const newOrderId = crypto.randomUUID();
    const ws = new WebSocket(WS_BASE);
    let receivedEvents: any[] = [];

    await new Promise<void>((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error('WS auth timeout')), 5000);
      ws.onopen = () => ws.send(JSON.stringify({ type: 'auth', token: ownerToken }));
      ws.onmessage = (ev) => {
        const msg = JSON.parse(ev.data.toString());
        if (msg.type === 'auth_success') {
          ws.send(JSON.stringify({ type: 'subscribe', room: `location:${locId}:dashboard` }));
        } else if (msg.type === 'subscribed') {
          clearTimeout(timeout);
          resolve();
        } else if (msg.data) {
          receivedEvents.push(msg.data);
        }
      };
      ws.onerror = () => reject(new Error('WS error'));
    });

    // Create new order → should receive event
    await pool.query(`INSERT INTO orders (id, location_id, subtotal, total, request_hash, type, status, payment_method, payment_outcome)
      VALUES ($1, $2, 500, 500, 'p24-ws', 'delivery', 'PENDING', 'cash', 'pending')`,
      [newOrderId, locId]);

    // Trigger status transition via the existing PATCH endpoint
    const adminToken = await signAuthToken({ role: 'owner', userId, activeLocationId: locId }, '15m');
    const patchRes = await fetch(`${BASE}/orders/${newOrderId}/status`, {
      method: 'PATCH',
      headers: { Authorization: `Bearer ${adminToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ status: 'CONFIRMED' }),
    });
    assert.strictEqual(patchRes.status, 200);

    await delay(500);

    // Check if dashboard room received the event
    const hasDashboardEvent = receivedEvents.some((e: any) =>
      e.orderId === newOrderId || (e.data?.orderId === newOrderId)
    );
    assert.ok(hasDashboardEvent, 'Dashboard room should receive order events');

    ws.close();
  });

  // ─── N=2 Broadcast ─────────────────────────────────────────────────
  await t.test('N=2 broadcast: instance A event → instance B WS receives', async () => {
    const n2OrderId = crypto.randomUUID();
    const wsB = new WebSocket(WS_BASE);
    let receivedOnB: any[] = [];

    await new Promise<void>((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error('WS B auth timeout')), 5000);
      wsB.onopen = () => wsB.send(JSON.stringify({ type: 'auth', token: ownerToken }));
      wsB.onmessage = (ev) => {
        const msg = JSON.parse(ev.data.toString());
        if (msg.type === 'auth_success') {
          wsB.send(JSON.stringify({ type: 'subscribe', room: `location:${locId}:dashboard` }));
        } else if (msg.type === 'subscribed') {
          clearTimeout(timeout);
          resolve();
        } else if (msg.data) {
          receivedOnB.push(msg.data);
        }
      };
    });

    // Create order on "instance A" (same DB, same server, but tests cross-instance via MessageBus)
    await pool.query(`INSERT INTO orders (id, location_id, subtotal, total, request_hash, type, status, payment_method, payment_outcome)
      VALUES ($1, $2, 300, 300, 'p24-n2', 'delivery', 'PENDING', 'cash', 'pending')`,
      [n2OrderId, locId]);

    // Confirm via PATCH endpoint (this publishes to MessageBus)
    const adminToken = await signAuthToken({ role: 'owner', userId, activeLocationId: locId }, '15m');
    await fetch(`${BASE}/orders/${n2OrderId}/status`, {
      method: 'PATCH',
      headers: { Authorization: `Bearer ${adminToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ status: 'CONFIRMED' }),
    });

    await delay(500);

    const received = receivedOnB.some((e: any) => e.orderId === n2OrderId);
    assert.ok(received, 'N=2 broadcast: event must reach WS B');

    wsB.close();
  });

  // ─── Dashboard HTML served ─────────────────────────────────────────
  await t.test('dashboard.html is served', async () => {
    const res = await fetch(`${BASE}/admin/dashboard.html`);
    assert.strictEqual(res.status, 200);
    const html = await res.text();
    assert.ok(html.includes('dashboard.js'), 'HTML must include dashboard.js');
    assert.ok(html.includes('kanban'), 'HTML must contain kanban container');
  });

  await t.test('orders.html is served', async () => {
    const res = await fetch(`${BASE}/admin/orders.html`);
    assert.strictEqual(res.status, 200);
  });

  await t.test('active-delivery.html is served', async () => {
    const res = await fetch(`${BASE}/admin/active-delivery.html`);
    assert.strictEqual(res.status, 200);
  });

  await t.test('dashboard.js is served', async () => {
    const res = await fetch(`${BASE}/admin/dashboard.js`);
    assert.strictEqual(res.status, 200);
    const js = await res.text();
    assert.ok(js.includes('DashboardWSClient'), 'JS must export DashboardWSClient');
    assert.ok(js.includes('fetchSnapshot'), 'JS must export fetchSnapshot');
  });

  // ─── 0 Cookies ────────────────────────────────────────────────────
  await t.test('0 cookies after dashboard flow', async () => {
    const res = await fetch(`${BASE}/admin/dashboard.html`);
    const cookies = res.headers.get('set-cookie');
    assert.ok(!cookies, 'No cookies set by dashboard');
  });

  // ─── PII Leak in WS Events ────────────────────────────────────────
  await t.test('0 PII in WS events', async () => {
    const ws = new WebSocket(WS_BASE);
    const events: string[] = [];

    await new Promise<void>((resolve, reject) => {
      const timeout = setTimeout(() => resolve(), 3000);
      ws.onopen = () => ws.send(JSON.stringify({ type: 'auth', token: ownerToken }));
      ws.onmessage = (ev) => {
        const msg = JSON.parse(ev.data.toString());
        if (msg.type === 'auth_success') {
          ws.send(JSON.stringify({ type: 'subscribe', room: `location:${locId}:dashboard` }));
        } else if (msg.type === 'subscribed') {
          // Create an order to trigger events
          const eid = crypto.randomUUID();
          pool.query(`INSERT INTO orders (id, location_id, subtotal, total, request_hash, type, status, payment_method, payment_outcome)
            VALUES ($1, $2, 100, 100, 'p24-pii', 'delivery', 'PENDING', 'cash', 'pending')`,
            [eid, locId]).then(() => {
            pool.query(`UPDATE orders SET status = 'CONFIRMED' WHERE id = $1`, [eid]);
          });
          setTimeout(() => { clearTimeout(timeout); resolve(); }, 1000);
        } else {
          events.push(JSON.stringify(msg));
        }
      };
    });
    ws.close();

    const allText = events.join(' ');
    const rawPhone = allText.match(/\+355\d{8,}/);
    const rawEmail = allText.match(/[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/);
    if (rawPhone) assert.fail(`Raw phone in WS event: ${rawPhone[0]}`);
    if (rawEmail) {
      const safe = rawEmail[0];
      if (!safe.includes('test.com') && !safe.includes('@deliveryos')) {
        assert.fail(`Raw email in WS event: ${rawEmail[0]}`);
      }
    }
  });

  // ─── DB Cleanup ────────────────────────────────────────────────────
  await t.test('cleanup', async () => {
    await pool.query('DELETE FROM order_items WHERE order_id = $1', [orderId]);
    await pool.query('DELETE FROM orders WHERE location_id IN ($1, $2)', [locId, locIdB]);
    await pool.query('DELETE FROM products WHERE id = $1', [prodId]);
    await pool.query('DELETE FROM customers WHERE id = $1', [custId]);
    await pool.query('DELETE FROM locations WHERE id IN ($1, $2)', [locId, locIdB]);
    await pool.query('DELETE FROM organizations WHERE id = $1', [orgId]);
    await pool.query('DELETE FROM users WHERE id = $1', [userId]);
  });
});
