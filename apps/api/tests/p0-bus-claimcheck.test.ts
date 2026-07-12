import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { Pool } from 'pg';

// P0-3 proof (ADR-p0-privacy-hardening): MessageBus claim-check — ZERO customer PII
// (name / phone / item names) on the bus. Requires seeded local `dowiz_sag`.
const DB_URL = process.env.SAG_TEST_DB_URL
  || 'postgresql://postgres:postgres@127.0.0.1:5432/dowiz_sag?sslmode=disable';
function ensureEnv() {
  const d: Record<string, string> = {
    NODE_ENV: 'test', APP_BASE_URL: 'http://localhost:3000',
    DATABASE_URL_OPERATIONAL: DB_URL, DATABASE_URL_SESSION: DB_URL, DATABASE_URL_MIGRATIONS: DB_URL,
    REDIS_URL: 'redis://localhost:6379', JWT_PRIVATE_KEY: 'x', JWT_PUBLIC_KEY: 'x', JWT_KID: 'x',
    GOOGLE_CLIENT_ID: 'x', GOOGLE_CLIENT_SECRET: 'x', VAPID_PUBLIC_KEY: 'x', VAPID_PRIVATE_KEY: 'x', IP_HASH_SALT: 'x',
  };
  for (const [k, v] of Object.entries(d)) if (!process.env[k]) process.env[k] = v;
}
ensureEnv();

const REPO = fileURLToPath(new URL('../../../', import.meta.url));
const SECRET_ITEM = 'Insulin-Friendly Diabetic Special';
// Synthetic PII seeded into a REAL customer row so a value-level leak is detectable
// (a vacuous pass is impossible: the order now references a customer that has a name+phone).
const SECRET_CUSTOMER_NAME = 'Testname Leakcanary';
const SECRET_CUSTOMER_PHONE = '+355690000777';
const PII_TOKENS = ['customername', 'customerphone', 'customernamemasked', 'customerphonemasked', 'itemssummary', 'name_snapshot'];

let pool: Pool;
before(async () => { pool = new Pool({ connectionString: DB_URL, max: 3 }); });
after(async () => { await pool?.end(); });

test('runtime: updateOrderStatus publishes a dashboard delta with NO customer PII', async () => {
  const { updateOrderStatus } = await import('../src/lib/orderStatusService.js');
  const loc = await pool.query(`SELECT id FROM locations LIMIT 1`);
  const locationId = loc.rows[0].id;
  // Seed a REAL customer (name + phone) and link the order to it, so the PII the
  // bus must NOT carry actually exists in the DB — otherwise the test passes vacuously.
  const cust = await pool.query(
    `INSERT INTO customers (location_id, name, phone) VALUES ($1, $2, $3)
     ON CONFLICT (location_id, phone) DO UPDATE SET name = EXCLUDED.name RETURNING id`,
    [locationId, SECRET_CUSTOMER_NAME, SECRET_CUSTOMER_PHONE]);
  const customerId = cust.rows[0].id;
  const ord = await pool.query(
    `INSERT INTO orders (location_id, customer_id, subtotal, total, request_hash, status)
     VALUES ($1, $2, 1000, 1000, 'p0bus-'||gen_random_uuid()::text, 'PENDING') RETURNING id`, [locationId, customerId]);
  const orderId = ord.rows[0].id;
  await pool.query(
    `INSERT INTO order_items (order_id, name_snapshot, price_snapshot, quantity) VALUES ($1, $2, 1000, 1)`,
    [orderId, SECRET_ITEM]);

  const published: any[] = [];
  const messageBus: any = { publish: async (channel: string, payload: any) => { published.push({ channel, payload }); } };
  const client = await pool.connect();
  try {
    await updateOrderStatus(client, orderId, locationId, 'CONFIRMED' as any, { messageBus });
  } finally { client.release(); }

  const dash = published.find(p => /:dashboard$/.test(p.channel));
  assert.ok(dash, 'a dashboard publish happened');
  const blob = JSON.stringify(published).toLowerCase();
  // No item name, no customer name/phone anywhere on the bus.
  assert.ok(!blob.includes(SECRET_ITEM.toLowerCase()), 'item name (special-category) must NOT be on the bus');
  for (const t of PII_TOKENS) assert.ok(!blob.includes(t), `bus payload must not carry "${t}"`);
  // VALUE-level checks: a field carrying the real name/phone under ANY key (e.g. `contact`,
  // `metadata`) would slip past key-name scanning — assert the actual seeded VALUES are absent.
  assert.ok(!blob.includes(SECRET_CUSTOMER_NAME.toLowerCase()), 'customer name VALUE must NOT be on the bus');
  assert.ok(!blob.includes(SECRET_CUSTOMER_PHONE.toLowerCase()), 'customer phone VALUE must NOT be on the bus');
  // But the useful non-PII fields ARE present.
  assert.equal(dash.payload.data.status, 'CONFIRMED');
  assert.equal(dash.payload.data.itemCount, 1, 'itemCount (non-PII) preserved');
  assert.ok(dash.payload.data.orderId && dash.payload.data.shortId, 'ids present for the client to re-fetch');

  await pool.query('DELETE FROM order_items WHERE order_id=$1', [orderId]);
  await pool.query('DELETE FROM orders WHERE id=$1', [orderId]);
  await pool.query('DELETE FROM customers WHERE id=$1', [customerId]);
});

test('static census: both order producers publish no customer PII keys to the bus', () => {
  // EVERY dashboard producer in orders.ts (not just the first) — scan all matches.
  const orders = readFileSync(REPO + 'apps/api/src/routes/orders.ts', 'utf8');
  const blocks = orders.match(/publish\(dashboardChannel[\s\S]{0,600}/g) || [];
  assert.ok(blocks.length > 0, 'at least one dashboard publish exists in orders.ts');
  for (const block of blocks) {
    for (const t of ['customerNameMasked', 'customerPhoneMasked', 'itemsSummary', 'courierName']) {
      assert.ok(!block.includes(t), `orders.ts dashboard bus payload must not include ${t}`);
    }
  }
  // order.status delta producer
  const svc = readFileSync(REPO + 'apps/api/src/lib/orderStatusService.ts', 'utf8');
  for (const t of ['itemsSummary', 'items_summary', 'string_agg']) {
    assert.ok(!svc.includes(t), `orderStatusService delta must not build ${t}`);
  }
});
