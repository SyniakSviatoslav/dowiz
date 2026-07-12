import { test, expect, type APIRequestContext } from '@playwright/test';
import crypto from 'node:crypto';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

/**
 * SENSOR-BUS §1.1 runtime proof (ADR-0009 v4) — the courier_geofence_enter sensor.
 * Run: VITE_BASE_URL=https://dowiz-staging.fly.dev DEV_AUTH_SECRET=stg-e2e-secret \
 *      [SENSOR_DB_URL=postgres://… via fly proxy] \
 *      pnpm exec playwright test flow-sensor-geofence --reporter=list
 *
 * Drives a real courier onto an active delivery and pings AROUND the venue, exercising the geofence
 * detection path in apps/api/src/routes/courier/shifts.ts. Asserts both the API-observable signals
 * and (when SENSOR_DB_URL is exported) the definitive DB state:
 *   - the courier accepts their OWN assignment (order_id is read server-side, never payload — R2-H1),
 *     and a SECOND courier is 404'd on the same assignment id (cross-courier IDOR control),
 *   - an OUTSIDE-the-radius ping stores GPS but writes NO geofence row (boundary logic, count 0),
 *   - an AT-venue ping writes exactly ONE row (count 1), and a re-cross is a no-op (UNIQUE +
 *     ON CONFLICT exactly-once — count stays 1).
 *
 * The DB-state assertions need a proxied staging connection string (SENSOR_DB_URL); without it the
 * spec still proves the API-observable path (status + gps_stored + IDOR + end-state). The ping
 * sequence respects the per-courier 10s rate-limit (shifts.ts ping config) with explicit waits.
 *
 * Skips cleanly unless the dev-mint secret is provided (the mock-auth + create-assignment helpers
 * are dev-gated by ALLOW_DEV_LOGIN + x-dev-auth-secret — ADR-0003).
 */

const DEV_SECRET = process.env.DEV_AUTH_SECRET;
const devHeaders = DEV_SECRET ? { 'x-dev-auth-secret': DEV_SECRET } : undefined;
const RATE_LIMIT_COOLDOWN_MS = 11_000; // ping rate-limit is max 1 / 10s per courier token

test.describe.configure({ mode: 'serial' });
test.skip(!DEV_SECRET, 'requires DEV_AUTH_SECRET to mint a courier + assignment (dev-gated, ADR-0003)');
test.beforeAll(() => requireStaging(process.env.VITE_BASE_URL)); // mutating spec — never write to prod

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/**
 * Count courier_geofence_enter rows for one order, directly against the staging DB.
 * Returns null when SENSOR_DB_URL is unset (the portable API-only path still runs).
 * TODO(needs_staging): requires SENSOR_DB_URL = a proxied staging connection string.
 */
async function geofenceEventCount(orderId: string): Promise<number | null> {
  const url = process.env.SENSOR_DB_URL;
  if (!url) return null;
  // pg is a workspace dep (apps/api); resolve from there so e2e need not declare it.
  const pgPath = createRequire(import.meta.url).resolve('pg', {
    paths: [fileURLToPath(new URL('../../apps/api/', import.meta.url))],
  });
  const { Client } = (await import(pgPath)) as typeof import('pg');
  const client = new Client({ connectionString: url });
  await client.connect();
  try {
    const r = await client.query(
      `SELECT count(*)::int AS n FROM order_sensor_events
        WHERE order_id = $1 AND event_type = 'courier_geofence_enter'`,
      [orderId],
    );
    return Number(r.rows[0].n);
  } finally {
    await client.end();
  }
}

async function demoTarget(request: APIRequestContext) {
  const [info, menu] = await Promise.all([
    request.get('/public/locations/demo/info', { headers: { accept: 'application/json' } }),
    request.get('/public/locations/demo/menu', { headers: { accept: 'application/json' } }),
  ]);
  expect(info.ok() && menu.ok(), 'demo info+menu load').toBeTruthy();
  const loc = await info.json();
  const m = await menu.json();
  const products: any[] = (m.categories ?? []).flatMap((c: any) => c.products ?? []);
  expect(products.length, 'demo has products').toBeGreaterThan(0);
  return { locationId: m.locationId ?? m.location_id, lat: loc.lat, lng: loc.lng, productId: products[0].id };
}

let phoneSeq = 0;
function uniquePhone() {
  phoneSeq += 1;
  return `+35565${String(Date.now()).slice(-4)}${String(phoneSeq).padStart(2, '0')}`;
}

test('courier crossing the venue geofence on their own active order fires the sensor path', async ({ request }) => {
  test.setTimeout(120_000); // two rate-limit cooldowns between pings

  const t = await demoTarget(request);

  // 1. A real delivery order pinned at the venue.
  const created = await request.post('/api/orders', {
    data: {
      locationId: t.locationId,
      type: 'delivery',
      items: [{ product_id: t.productId, quantity: 1 }],
      customer: { phone: uniquePhone(), name: 'E2E Geofence' },
      delivery: { pin: { lat: t.lat, lng: t.lng }, address_text: 'Demo HQ' },
      payment: { method: 'cash' },
      idempotency_key: crypto.randomUUID(),
      acknowledged_codes: ['velocity'],
    },
  });
  expect(created.status(), `create order: ${await created.text()}`).toBe(201);
  const orderId = (await created.json()).id as string;
  expectUuid(orderId, 'create order returns an order uuid');

  // 2. Mint a courier at the demo location + create their assignment (dev-gated).
  const mock = await request.post('/api/dev/mock-auth', { data: { role: 'courier', locationId: t.locationId }, headers: devHeaders });
  expect(mock.status(), `mock-auth courier: ${await mock.text()}`).toBe(200);
  const { access_token: courierTok, userId: courierId } = await mock.json();
  expectJwt(courierTok, 'courier access token');
  expectUuid(courierId, 'courier id');

  const asg = await request.post('/api/dev/create-assignment', { data: { orderId, courierId, locationId: t.locationId }, headers: devHeaders });
  expect(asg.status(), `create-assignment: ${await asg.text()}`).toBe(200);
  const assignmentId = (await asg.json()).assignmentId as string;
  expectUuid(assignmentId, 'assignment id');

  // 3. Cross-courier IDOR control — a SECOND courier must NOT be able to accept courier #1's
  //    assignment (the service scopes by courier_id and 404s another courier's id).
  const mock2 = await request.post('/api/dev/mock-auth', { data: { role: 'courier', locationId: t.locationId }, headers: devHeaders });
  expect(mock2.status(), `mock-auth courier #2: ${await mock2.text()}`).toBe(200);
  const { access_token: courier2Tok } = await mock2.json();
  expectJwt(courier2Tok, 'courier #2 access token');

  const idor = await request.post(`/api/courier/assignments/${assignmentId}/accept`, {
    headers: { Authorization: `Bearer ${courier2Tok}` },
  });
  expect(idor.status(), `cross-courier accept must 404: ${await idor.text()}`).toBe(404);

  // 4. Courier #1 accepts THEIR OWN assignment (scoped to token.sub — R2-H1).
  const accept = await request.post(`/api/courier/assignments/${assignmentId}/accept`, {
    headers: { Authorization: `Bearer ${courierTok}` },
  });
  expect(accept.status(), `accept: ${await accept.text()}`).toBe(200);

  // 5. OUTSIDE the geofence: ping ~110km away. On an active delivery so GPS is stored, but the
  //    venue-distance check must NOT write a geofence row → boundary logic exercised (count 0).
  const farPing = await request.post('/api/courier/shifts/ping', {
    data: { lat: t.lat + 1, lng: t.lng + 1 },
    headers: { Authorization: `Bearer ${courierTok}` },
  });
  expect(farPing.status(), `far ping: ${await farPing.text()}`).toBe(200);
  expect((await farPing.json()).gps_stored, 'far ping on active delivery still stores GPS').toBe(true);
  const afterFar = await geofenceEventCount(orderId);
  if (afterFar !== null) expect(afterFar, 'no geofence row when outside the radius').toBe(0);

  await sleep(RATE_LIMIT_COOLDOWN_MS);

  // 6. AT the venue → onActiveDelivery → the geofence block writes exactly ONE
  //    courier_geofence_enter row (order_id read server-side from the assignment).
  const ping = await request.post('/api/courier/shifts/ping', {
    data: { lat: t.lat, lng: t.lng },
    headers: { Authorization: `Bearer ${courierTok}` },
  });
  expect(ping.status(), `ping: ${await ping.text()}`).toBe(200);
  expect((await ping.json()).gps_stored, 'ping on active delivery stores GPS (geofence path executed)').toBe(true);
  const afterEnter = await geofenceEventCount(orderId);
  if (afterEnter !== null) expect(afterEnter, 'exactly one geofence row landed inside the radius').toBe(1);

  await sleep(RATE_LIMIT_COOLDOWN_MS);

  // 7. Re-cross the SAME geofence → idempotent no-op (UNIQUE(order_id,event_type) + ON CONFLICT
  //    DO NOTHING). Still 200; the row count must stay exactly 1 (exactly-once, the C2 ship-blocker).
  const reCross = await request.post('/api/courier/shifts/ping', {
    data: { lat: t.lat, lng: t.lng },
    headers: { Authorization: `Bearer ${courierTok}` },
  });
  expect(reCross.status(), `re-cross ping: ${await reCross.text()}`).toBe(200);
  expect((await reCross.json()).gps_stored, 're-cross ping still stores GPS').toBe(true);
  const afterReCross = await geofenceEventCount(orderId);
  if (afterReCross !== null) expect(afterReCross, 're-cross is a no-op — still exactly one row').toBe(1);

  // 8. End-state: the courier's own task list reflects the accepted assignment for this order.
  const mine = await request.get('/api/courier/me/assignments', {
    headers: { Authorization: `Bearer ${courierTok}` },
  });
  expect(mine.status(), `me/assignments: ${await mine.text()}`).toBe(200);
  const tasks: any[] = (await mine.json()).assignments ?? [];
  const myTask = tasks.find((a) => a.orderId === orderId);
  expect(myTask, 'accepted assignment surfaces in the courier task list').toBeTruthy();
  expect(myTask.status, 'assignment is in accepted state after accept').toBe('accepted');
});
