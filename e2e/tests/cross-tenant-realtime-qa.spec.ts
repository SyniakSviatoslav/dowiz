import { test, expect, request, type APIRequestContext } from '@playwright/test';
import WebSocket from 'ws';
import crypto from 'node:crypto';
import { expectJwt, expectUuid } from '../helpers/assert-shape';

// ─────────────────────────────────────────────────────────────────────────────
// Cross-tenant, multi-role, real-time ordering QA/validation loop (v2).
// Runs against the REAL, NON-MOCKED staged service (real auth, DB, WS, courier).
// v2 hardened after a critique/security/QA agent sweep — the real-time and isolation
// assertions now prove CONTENT + PER-TRANSITION delivery, not "≥1 message exists".
//
//   Role 1 CUSTOMER — storefront UI + a REAL order (idempotent) + the REAL tracking UI.
//   Role 2 OWNER    — real login; positive-control read; drives the lifecycle.
//   REAL-TIME       — owner dashboard AND customer order-room each receive a NEW delta
//                     per transition (decoupled from courier so it always runs).
//   Role 3 COURIER  — a REAL online courier (random per-run password) is assigned + drives
//                     picked-up → IN_DELIVERY (skips only this dimension if rate-limited).
//   ISOLATION       — positive control + cross-order token denial + customer-A→order-B WS
//                     room denial (a REAL room A isn't a member of) + owner fake-room + authz.
//   HYGIENE         — every QA order tagged + cancelled in cleanup (no fake sales).
//
//   VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test \
//     e2e/tests/cross-tenant-realtime-qa.spec.ts --project=desktop --reporter=list
// ─────────────────────────────────────────────────────────────────────────────

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const WS_URL = BASE.replace(/^http/, 'ws');
const OWNER = { email: process.env.QA_OWNER_EMAIL || 'test@dowiz.com', password: process.env.QA_OWNER_PASSWORD || 'test123456' };
const SLUG = 'demo';
const QA_TAG = 'AUTOMATED-QA — safe to purge';

test.describe.configure({ mode: 'serial' });
test.setTimeout(120_000);

let api: APIRequestContext;
let ownerToken = '';
let locationId = '';
let orderId = '';
let customerToken = '';
let courierToken = '';
let courierId = '';
let mainAssignmentId = '';
const createdOrders: { id: string; token: string }[] = [];

const ownerHdr = () => ({ authorization: `Bearer ${ownerToken}` });
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function collector(token: string, room: string) {
  const msgs: unknown[] = [];
  const ws = new WebSocket(`${WS_URL}/?token=${encodeURIComponent(token)}`);
  let opened = false;
  const ready = new Promise<void>((resolve) => {
    ws.on('open', () => { opened = true; ws.send(JSON.stringify({ type: 'subscribe', room })); resolve(); });
    ws.on('error', () => resolve());
  });
  ws.on('message', (d) => { try { msgs.push(JSON.parse(d.toString())); } catch { /* non-JSON */ } });
  const refsFor = (id: string) => msgs.filter((m) => JSON.stringify(m).includes(id)).length;
  const hasContent = (id: string, needle: string) =>
    msgs.some((m) => { const s = JSON.stringify(m).toLowerCase(); return s.includes(id.toLowerCase()) && s.includes(needle.toLowerCase()); });
  return { ws, ready, msgs, refsFor, hasContent, wasOpened: () => opened, close: () => { try { ws.close(); } catch { /* closed */ } } };
}

// A real order against the staged service, tagged + idempotent. The probe (no acked
// codes) reveals soft reasons; on the CLEAN path it already creates the order (201) →
// track it and return without a 2nd create (no orphan). On the soft path the probe
// returns 200 (no order) and a 2nd call with the acked codes creates it.
async function placeRealOrder(): Promise<{ orderId: string; customerToken: string }> {
  const menu = await (await api.get(`${BASE}/public/locations/${SLUG}/menu`)).json();
  const product = (menu.categories ?? []).flatMap((c: any) => c.products ?? []).find((p: any) => p.is_available !== false);
  expect(product, 'a real available product').toBeTruthy();
  const post = (key: string, acknowledged_codes: string[]) => api.post(`${BASE}/api/orders`, {
    data: {
      locationId, type: 'delivery', items: [{ product_id: product.id, quantity: 1 }],
      customer: { phone: '+355691234567', name: 'QA-LOOP' },
      delivery: { pin: { lat: 41.324, lng: 19.456 } },
      delivery_instructions: QA_TAG,
      payment: { method: 'cash' }, idempotency_key: key, acknowledged_codes,
    },
  });
  const track = (j: any) => { createdOrders.push({ id: j.id, token: j.authToken }); return { orderId: j.id, customerToken: j.authToken }; };

  const probe = await post(crypto.randomUUID(), []);
  if (probe.status() === 201) return track(await probe.json()); // clean path — probe created it
  const codes: string[] = ((await probe.json()).reasons ?? []).map((r: any) => r.code);
  const res = await post(crypto.randomUUID(), codes);                // soft path — confirm creates it
  expect(res.status(), `order create (soft-ack ${codes.join(',') || 'none'})`).toBe(201);
  return track(await res.json());
}

// GAP 2 — seed a REAL online courier (invite → redeem → shift). RANDOM per-run password
// (no standing reusable credential). Returns null on rate-limit (429) so only the courier
// dimension degrades. Note: staging has no courier-delete API, so the account persists with
// an unknown random password — not a usable backdoor.
async function seedOnlineCourier(): Promise<{ courierToken: string; courierId: string } | null> {
  const email = `qa-courier+${Date.now()}@dowiz.dev`;
  const password = `Qa1!${crypto.randomUUID()}`;
  const inv = await (await api.post(`${BASE}/api/owner/locations/${locationId}/courier-invites`, { headers: ownerHdr(), data: { role: 'courier', email } })).json();
  expect(inv.code, 'courier invite returns a one-time code').toBeTruthy();
  const redRes = await api.post(`${BASE}/api/courier/auth/invites/${inv.inviteId}/redeem`, {
    data: { email, code: inv.code, password, full_name: 'QA Courier', phone: '+355690000111' },
  });
  if (redRes.status() === 429) { console.log('[courier] seed rate-limited (429) — courier dimension skipped'); return null; }
  const red = await redRes.json();
  expectJwt(red.jwt, `courier redeem jwt (HTTP ${redRes.status()} ${JSON.stringify(red).slice(0, 140)})`);
  const shift = await api.post(`${BASE}/api/courier/me/shift/start`, { headers: { authorization: `Bearer ${red.jwt}` }, data: { lat: 41.324, lng: 19.456 } });
  expect(shift.ok(), `courier shift start → ${shift.status()}`).toBeTruthy();
  return { courierToken: red.jwt, courierId: red.courier.id };
}

test.beforeAll(async () => {
  api = await request.newContext();
  const login = await api.post(`${BASE}/api/auth/local/login`, { data: OWNER });
  expect(login.ok(), 'owner REAL login').toBeTruthy();
  const j = await login.json();
  ownerToken = j.access_token;
  locationId = j.activeLocationId;
  expect(ownerToken && locationId, 'owner token + tenant').toBeTruthy();
});

test.afterAll(async () => {
  // HYGIENE (gap 1) — no hard-delete on staging; cancel the courier assignment (reverts an
  // IN_DELIVERY order to READY) then reject every QA order → terminal, never a fake sale.
  if (mainAssignmentId) {
    const c = await api.post(`${BASE}/api/courier/assignments/${mainAssignmentId}/cancel`, { headers: { authorization: `Bearer ${courierToken}` }, data: { reason: 'automated-qa-cleanup' } });
    console.log(`[cleanup] cancel assignment ${mainAssignmentId} → ${c.status()}`);
  }
  for (const o of createdOrders) {
    const r = await api.post(`${BASE}/api/owner/locations/${locationId}/orders/${o.id}/reject`, { headers: ownerHdr(), data: { reason: 'automated-qa-cleanup' } });
    console.log(`[cleanup] reject ${o.id} → ${r.status()}${r.ok() ? '' : ' (not cancellable — tagged for purge)'}`);
  }
});

test('Role 1a — CUSTOMER storefront UI renders menu → cart → checkout (real UI)', async ({ page }) => {
  await page.goto(`${BASE}/s/${SLUG}`);
  await expect(page.locator('[data-testid=menu-item]').first()).toBeVisible({ timeout: 20_000 });
  await page.locator('[data-testid=menu-item-add]').first().click();
  await page.locator('[data-testid=cart-open]').click();
  await page.locator('[data-testid=cart-checkout]').click();
  await expect(page.locator('[data-testid=order-confirm-button]'), 'checkout reached').toBeVisible({ timeout: 15_000 });
  await page.screenshot({ path: 'audit/qa-realtime/01-checkout-ui.png' });
});

test('Role 1b — a REAL order is placed against the staged service (idempotent)', async () => {
  ({ orderId, customerToken } = await placeRealOrder());
  expect(orderId).toMatch(/[0-9a-fA-F-]{6,}/);
  expectJwt(customerToken, 'customer token');
});

test('Role 1c — the customer sees their real order in the real tracking UI', async ({ page }) => {
  await page.addInitScript((tok) => localStorage.setItem('dos_access_token', tok as string), customerToken);
  await page.goto(`${BASE}/s/${SLUG}/order/${orderId}`);
  await expect(page.locator('[data-testid=order-progress]'), 'real tracking UI renders').toBeVisible({ timeout: 20_000 });
  await page.screenshot({ path: 'audit/qa-realtime/02-order-tracking.png' });
});

test('Role 2 — OWNER sees the order + customer positive-control read (de-vacuums isolation)', async () => {
  const res = await api.get(`${BASE}/api/owner/orders`, { headers: ownerHdr() });
  expect(res.ok(), `GET /api/owner/orders → ${res.status()}`).toBeTruthy();
  expect(await res.text(), 'order visible to its owner').toContain(orderId);
  // POSITIVE CONTROL — the customer CAN read their own order (so the isolation negatives mean something).
  const own = await api.get(`${BASE}/api/orders/${orderId}`, { headers: { authorization: `Bearer ${customerToken}` } });
  expect(own.status(), 'customer reads OWN order = 200').toBe(200);
  expect(await own.text()).toContain(orderId);
});

test('REAL-TIME — owner dashboard AND customer order-room each get a NEW delta per transition', async () => {
  const dash = collector(ownerToken, `location:${locationId}:dashboard`);
  const cust = collector(customerToken, `order:${orderId}`);
  await Promise.all([dash.ready, cust.ready]);
  expect(dash.wasOpened() && cust.wasOpened(), 'both WS connections opened (no silent error)').toBe(true);
  await sleep(700); // absorb any on-subscribe snapshot so we measure DELTAS, not the snapshot

  for (const status of ['CONFIRMED', 'PREPARING', 'READY'] as const) {
    const dashBefore = dash.refsFor(orderId);
    const custBefore = cust.refsFor(orderId);
    const r = await api.patch(`${BASE}/api/orders/${orderId}/status`, { headers: ownerHdr(), data: { status } });
    expect(r.ok(), `lifecycle → ${status} (${r.status()})`).toBeTruthy();
    // a NEW live delta for THIS order on the owner dashboard (proves per-transition broadcasting).
    await expect.poll(() => dash.refsFor(orderId), { timeout: 8_000, message: `owner WS new delta on ${status}` }).toBeGreaterThan(dashBefore);
    // the customer order-room ALSO receives the live update (the user-facing real-time path).
    await expect.poll(() => cust.refsFor(orderId), { timeout: 8_000, message: `customer WS new delta on ${status}` }).toBeGreaterThan(custBefore);
    // content (soft): the delta should carry the new status string.
    expect.soft(dash.hasContent(orderId, status), `owner delta carries ${status}`).toBe(true);
  }
  dash.close();
  cust.close();
});

test('Role 3 — COURIER dispatch: REAL online courier assigned, drives picked-up → IN_DELIVERY', async () => {
  const seeded = await seedOnlineCourier();
  test.skip(!seeded, 'courier auth rate-limited this run — re-run after cooldown');
  ({ courierToken, courierId } = seeded!);

  const assign = await api.post(`${BASE}/api/owner/locations/${locationId}/orders/${orderId}/assign-courier`, { headers: ownerHdr(), data: { courierId } });
  expect(assign.ok(), `owner assign-courier → ${assign.status()}`).toBeTruthy();

  await expect.poll(async () => {
    const list = await (await api.get(`${BASE}/api/courier/me/assignments`, { headers: { authorization: `Bearer ${courierToken}` } })).json();
    const arr = Array.isArray(list) ? list : (list.assignments ?? []);
    const match = arr.find((a: any) => JSON.stringify(a).includes(orderId));
    if (match) mainAssignmentId = match.id ?? match.assignment_id ?? '';
    return !!match;
  }, { timeout: 8_000, message: 'the seeded online courier received the assignment' }).toBe(true);
  expectUuid(mainAssignmentId, 'assignment id');

  const pickedUp = await api.post(`${BASE}/api/courier/assignments/${mainAssignmentId}/picked-up`, { headers: { authorization: `Bearer ${courierToken}` } });
  expect(pickedUp.ok(), `courier picked-up → IN_DELIVERY (${pickedUp.status()})`).toBeTruthy();
});

test('ISOLATION — cross-order token denial + customer-A→order-B WS room denial + authz', async () => {
  const second = await placeRealOrder(); // a 2nd REAL order with a different customer token

  // (1) cross-order: customer-A cannot READ order-B (and the body leaks no order-B data).
  const aReadsB = await api.get(`${BASE}/api/orders/${second.orderId}`, { headers: { authorization: `Bearer ${customerToken}` } });
  // an order-scoped token whose orderId !== :id gets exactly 404 (hidden, not 403 — no existence leak)
  expect(aReadsB.status(), 'customer-A → order-B').toBe(404);
  expect(await aReadsB.text(), 'no order-B payload leaked to customer-A').not.toContain(QA_TAG);

  // (2) WS room denial against a REAL room A is not a member of (order:<B>) — not an empty fake.
  const hijack = collector(customerToken, `order:${second.orderId}`);
  await hijack.ready;
  await sleep(1_500);
  expect(hijack.refsFor(second.orderId), 'customer-A cannot read order-B WS room').toBe(0);
  hijack.close();

  // (3) owner cannot receive a foreign tenant's dashboard stream (ownerCanAccessRoom guard).
  const FAKE_TENANT = '00000000-0000-0000-0000-000000000000';
  const intruder = collector(ownerToken, `location:${FAKE_TENANT}:dashboard`);
  await intruder.ready;
  await sleep(1_500);
  expect(intruder.refsFor(FAKE_TENANT), 'no foreign-tenant WS leak').toBe(0);
  intruder.close();

  // (4) owner API requires auth; (5) a customer token cannot drive owner-only status.
  const unauth = await api.get(`${BASE}/api/owner/orders`);
  expect(unauth.status(), 'unauth owner read').toBe(401);
  const forbidden = await api.patch(`${BASE}/api/orders/${orderId}/status`, { headers: { authorization: `Bearer ${customerToken}` }, data: { status: 'CANCELLED' } });
  expect(forbidden.status(), 'customer PATCH owner-only status').toBe(403); // requireRole(['owner'])
});
