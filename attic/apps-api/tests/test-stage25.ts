import { test } from 'node:test';
import assert from 'node:assert';
import crypto from 'node:crypto';
import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';
import { signAuthToken, verifyAuthToken } from '@deliveryos/platform';

const env = loadEnv();
const BASE = `http://127.0.0.1:${env.PORT || 3003}`;

function delay(ms: number) { return new Promise(r => setTimeout(r, ms)); }

test('Stage 25: Dwell Monitor + Alerts + Settings', async (t) => {
  const pool = createSessionPool();
  const orgId = crypto.randomUUID();
  const locId = crypto.randomUUID();
  const locIdB = crypto.randomUUID();
  const userId = crypto.randomUUID();
  const custId = crypto.randomUUID();
  const prodId = crypto.randomUUID();
  const orderId = crypto.randomUUID();
  const orderId2 = crypto.randomUUID();

  let ownerToken: string;
  let alertId: string;

  // ─── Setup ────────────────────────────────────────────────────────
  await t.test('setup test data', async () => {
    await pool.query(`INSERT INTO users (id, email) VALUES ($1, $2) ON CONFLICT DO NOTHING`,
      [userId, `owner-p25-${Date.now()}@test.com`]);
    await pool.query(`INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'P25 Org', $2) ON CONFLICT DO NOTHING`,
      [orgId, userId]);
    await pool.query(`INSERT INTO locations (id, org_id, slug, name, phone, status, confirm_timeout_min)
      VALUES ($1, $2, $3, 'P25 Loc', '123', 'open', 1) ON CONFLICT DO NOTHING`,
      [locId, orgId, `p25-loc-${Date.now()}`]);
    await pool.query(`INSERT INTO locations (id, org_id, slug, name, phone, status, confirm_timeout_min)
      VALUES ($1, $2, $3, 'P25 Loc B', '456', 'open', 1) ON CONFLICT DO NOTHING`,
      [locIdB, orgId, `p25-loc-b-${Date.now()}`]);
    await pool.query(`INSERT INTO customers (id, location_id, phone, name) VALUES ($1, $2, '+355691234567', 'Test Customer') ON CONFLICT DO NOTHING`,
      [custId, locId]);
    await pool.query(`INSERT INTO products (id, location_id, name, price, is_available) VALUES ($1, $2, 'Test Product', 500, true) ON CONFLICT DO NOTHING`,
      [prodId, locId]);

    // Orders with old timestamps to trigger dwell
    await pool.query(`INSERT INTO orders (id, location_id, customer_id, subtotal, total, request_hash, type, status, payment_method, payment_outcome, created_at, confirmed_at)
      VALUES ($1, $2, $3, 1000, 1200, 'p25-test-1', 'delivery', 'PENDING', 'cash', 'pending', now() - interval '5 minutes', now() - interval '5 minutes') ON CONFLICT DO NOTHING`,
      [orderId, locId, custId]);
    await pool.query(`INSERT INTO orders (id, location_id, customer_id, subtotal, total, request_hash, type, status, payment_method, payment_outcome, created_at, confirmed_at)
      VALUES ($1, $2, $3, 2000, 2400, 'p25-test-2', 'delivery', 'PENDING', 'cash', 'pending', now() - interval '3 minutes', now() - interval '3 minutes') ON CONFLICT DO NOTHING`,
      [orderId2, locId, custId]);
    await pool.query(`INSERT INTO order_items (order_id, product_id, name_snapshot, price_snapshot, quantity)
      VALUES ($1, $2, 'Test Product', 500, 2) ON CONFLICT DO NOTHING`,
      [orderId, prodId]);
    await pool.query(`INSERT INTO order_items (order_id, product_id, name_snapshot, price_snapshot, quantity)
      VALUES ($1, $2, 'Test Product', 500, 2) ON CONFLICT DO NOTHING`,
      [orderId2, prodId]);

    ownerToken = await signAuthToken({ role: 'owner', userId, activeLocationId: locId }, '15m');
  });

  // ─── Dwell Settings Tests ─────────────────────────────────────────
  await t.test('GET /settings/dwell returns defaults', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/settings/dwell`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    assert.strictEqual(res.status, 200);
    const data = await res.json();
    assert.ok(data.dwellThresholds);
    assert.strictEqual(data.dwellThresholds.pending_s, 60);
    assert.strictEqual(data.dwellThresholds.confirmed_s, 300);
    assert.strictEqual(data.dwellThresholds.preparing_s, 600);
    assert.strictEqual(data.dwellThresholds.en_route_s, 900);
  });

  await t.test('PUT /settings/dwell updates thresholds', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/settings/dwell`, {
      method: 'PUT',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ dwellThresholds: { pending_s: 120, confirmed_s: 600, preparing_s: 900, en_route_s: 1200 } }),
    });
    assert.strictEqual(res.status, 200);
    const data = await res.json();
    assert.strictEqual(data.dwellThresholds.pending_s, 120);
    assert.strictEqual(data.dwellThresholds.confirmed_s, 600);

    // Verify persisted
    const getRes = await fetch(`${BASE}/api/owner/locations/${locId}/settings/dwell`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    const getData = await getRes.json();
    assert.strictEqual(getData.dwellThresholds.pending_s, 120);
  });

  await t.test('PUT /settings/dwell validates thresholds', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/settings/dwell`, {
      method: 'PUT',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ dwellThresholds: { pending_s: -1 } }),
    });
    assert.strictEqual(res.status, 400);
  });

  await t.test('GET /settings/dwell — wrong location returns 404', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locIdB}/settings/dwell`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    assert.strictEqual(res.status, 404);
  });

  // ─── Alert Tests ──────────────────────────────────────────────────
  await t.test('GET /alerts — empty initially', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/alerts`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    assert.strictEqual(res.status, 200);
    const data = await res.json();
    assert.ok(Array.isArray(data.alerts));
  });

  await t.test('create alert directly in DB and verify it appears', async () => {
    const res = await pool.query(`
      INSERT INTO location_alerts (location_id, order_id, kind, status, escalation_level)
      VALUES ($1, $2, 'dwell_pending', 'active', 0)
      RETURNING id
    `, [locId, orderId]);
    assert.strictEqual(res.rowCount, 1);
    alertId = res.rows[0].id;
  });

  await t.test('GET /alerts returns created alert with PII masked', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/alerts`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    assert.strictEqual(res.status, 200);
    const data = await res.json();
    assert.ok(data.alerts.length >= 1);
    const alert = data.alerts.find((a: any) => a.id === alertId);
    assert.ok(alert);
    assert.strictEqual(alert.kind, 'dwell_pending');
    assert.strictEqual(alert.status, 'active');
    assert.strictEqual(alert.customerNameMasked, 'T***');
    assert.strictEqual(alert.customerPhoneMasked, '+*** *** 4567');
  });

  await t.test('GET /alerts — filter by kind', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/alerts?kind=dwell_pending`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    assert.strictEqual(res.status, 200);
    const data = await res.json();
    assert.ok(data.alerts.length >= 1);
    assert.ok(data.alerts.every((a: any) => a.kind === 'dwell_pending'));
  });

  await t.test('de-dup: same order+kind does not create duplicate active alert', async () => {
    const res = await pool.query(`
      INSERT INTO location_alerts (location_id, order_id, kind, status, escalation_level)
      VALUES ($1, $2, 'dwell_pending', 'active', 0)
      ON CONFLICT DO NOTHING
      RETURNING id
    `, [locId, orderId]);
    assert.strictEqual(res.rowCount, 0, 'Duplicate should be silently ignored');
  });

  // ─── Acknowledge ──────────────────────────────────────────────────
  await t.test('POST /alerts/:id/acknowledge resolves alert', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/alerts/${alertId}/acknowledge`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
    });
    assert.strictEqual(res.status, 200);
    const data = await res.json();
    assert.strictEqual(data.status, 'resolved');
    assert.ok(data.acknowledgedAt);

    // Verify in DB
    const dbRes = await pool.query(`SELECT status, resolved_at, resolution_reason FROM location_alerts WHERE id = $1`, [alertId]);
    assert.strictEqual(dbRes.rows[0].status, 'resolved');
    assert.strictEqual(dbRes.rows[0].resolution_reason, 'owner_acknowledge');
  });

  await t.test('POST /alerts/:id/acknowledge — double ack returns 404', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/alerts/${alertId}/acknowledge`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
    });
    assert.strictEqual(res.status, 404);
  });

  await t.test('POST /alerts/:id/acknowledge — wrong location returns 404', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locIdB}/alerts/${alertId}/acknowledge`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
    });
    assert.strictEqual(res.status, 404);
  });

  // ─── Bulk Acknowledge ─────────────────────────────────────────────
  await t.test('POST /alerts/acknowledge-all acknowledges all active alerts', async () => {
    // Create a few more alerts
    await pool.query(`
      INSERT INTO location_alerts (location_id, order_id, kind, status, escalation_level)
      VALUES ($1, $2, 'dwell_pending', 'active', 0)
    `, [locId, orderId2]);

    const res = await fetch(`${BASE}/api/owner/locations/${locId}/alerts/acknowledge-all`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: '{}',
    });
    assert.strictEqual(res.status, 200);
    const data = await res.json();
    assert.strictEqual(data.acknowledged, 1);

    // No more active alerts
    const getRes = await fetch(`${BASE}/api/owner/locations/${locId}/alerts?status=active`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    const getData = await getRes.json();
    assert.strictEqual(getData.alerts.length, 0);
  });

  // ─── Pagination ───────────────────────────────────────────────────
  await t.test('alerts pagination via cursor', async () => {
    // Create a resolved alert (should show in default list)
    const res = await pool.query(`
      INSERT INTO location_alerts (location_id, order_id, kind, status, escalation_level, created_at)
      VALUES ($1, $2, 'dwell_confirmed', 'active', 0, now())
      RETURNING id
    `, [locId, orderId]);
    const newAlertId = res.rows[0].id;

    const page1 = await fetch(`${BASE}/api/owner/locations/${locId}/alerts?limit=1`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    assert.strictEqual(page1.status, 200);
    const p1 = await page1.json();
    assert.strictEqual(p1.alerts.length, 1);
    assert.ok(p1.nextCursor);

    const page2 = await fetch(`${BASE}/api/owner/locations/${locId}/alerts?limit=1&cursor=${p1.nextCursor}`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    assert.strictEqual(page2.status, 200);
    const p2 = await page2.json();
    assert.strictEqual(p2.alerts.length, 1);
    assert.notStrictEqual(p2.alerts[0].id, p1.alerts[0].id);

    // Cleanup
    await pool.query(`DELETE FROM location_alerts WHERE id = $1`, [newAlertId]);
  });

  // ─── Tenant Isolation ─────────────────────────────────────────────
  await t.test('wrong location returns 404 (tenant isolation)', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locIdB}/alerts`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    assert.strictEqual(res.status, 404);
  });

  await t.test('customer token cannot access alerts', async () => {
    const custToken = await signAuthToken({ role: 'customer', userId: custId, activeLocationId: locId }, '15m');
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/alerts`, {
      headers: { Authorization: `Bearer ${custToken}` },
    });
    assert.strictEqual(res.status, 403);
  });

  // ─── Auth Guard ───────────────────────────────────────────────────
  await t.test('no token returns 401', async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/alerts`);
    assert.strictEqual(res.status, 401);
  });

  await t.test('expired token returns 401', async () => {
    const expired = await signAuthToken({ role: 'owner', userId, activeLocationId: locId }, '0s');
    await delay(100);
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/alerts`, {
      headers: { Authorization: `Bearer ${expired}` },
    });
    assert.strictEqual(res.status, 401);
  });

  // ─── PII Leak Check ───────────────────────────────────────────────
  await t.test('no raw PII in alert responses', async () => {
    // Re-create an alert to test fresh
    const alertRes = await pool.query(`
      INSERT INTO location_alerts (location_id, order_id, kind, status, escalation_level)
      VALUES ($1, $2, 'dwell_preparing', 'active', 0)
      RETURNING id
    `, [locId, orderId2]);
    const aid = alertRes.rows[0].id;

    const res = await fetch(`${BASE}/api/owner/locations/${locId}/alerts`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    const data = await res.json();
    const responseStr = JSON.stringify(data);
    // Should not contain raw customer name or full phone
    assert.ok(!responseStr.includes('Test Customer'), 'Should not contain raw customer name');
    assert.ok(!responseStr.includes('+355691234567'), 'Should not contain raw phone');
    assert.ok(data.alerts.every((a: any) => a.customerNameMasked && a.customerNameMasked.includes('***')));
    assert.ok(data.alerts.every((a: any) => a.customerPhoneMasked && a.customerPhoneMasked.includes('***')));

    // Cleanup
    await pool.query(`DELETE FROM location_alerts WHERE id = $1`, [aid]);
  });

  // ─── Cleanup ──────────────────────────────────────────────────────
  await t.test('cleanup test data', async () => {
    await pool.query(`DELETE FROM location_alerts WHERE location_id = $1`, [locId]);
    await pool.query(`DELETE FROM order_items WHERE order_id IN ($1, $2)`, [orderId, orderId2]);
    await pool.query(`DELETE FROM orders WHERE id IN ($1, $2)`, [orderId, orderId2]);
    await pool.query(`DELETE FROM products WHERE id = $1`, [prodId]);
    await pool.query(`DELETE FROM customers WHERE id = $1`, [custId]);
    await pool.query(`DELETE FROM locations WHERE id IN ($1, $2)`, [locId, locIdB]);
    await pool.query(`DELETE FROM organizations WHERE id = $1`, [orgId]);
    await pool.query(`DELETE FROM users WHERE id = $1`, [userId]);
  });

  await pool.end();
});
