import { test, expect, request, type APIRequestContext } from '@playwright/test';
import WebSocket from 'ws';
import crypto from 'node:crypto';
import { expectJwt, expectUuid } from '../helpers/assert-shape';

// ─────────────────────────────────────────────────────────────────────────────
// ADR-0013 — courier real-time authorization isolation net (B6 + C1 + N3).
// Runs against the REAL, NON-MOCKED staged service (real auth, DB, WS, courier).
//
//   POSITIVE CONTROL — a courier BOUND to an order receives its live `order:<id>` stream
//                      (and drives the REAL /courier/delivery/:id route — synthetic room BANNED).
//   B6 / location:   — a courier may NOT subscribe `location:*` (owner dashboard feed) → Forbidden.
//   N3 (WS)          — courier-2 (online, NOT bound to order-X) gets ZERO frames on `order:<X>`.
//   N3 (REST)        — courier-2 GET /api/orders/<X>/messages → 404 (assignment-scoped, not shop-wide).
//   C1 (eviction)    — after the owner REASSIGNS order-X from courier-1 → courier-2, courier-1 stops
//                      receiving within ≤TTL and gets `binding_revoked`; the post-expiry trigger frame
//                      itself is WITHHELD (zero frames incl. the trigger); courier-2 now receives.
//   DEFERRED here    — UNAVAILABLE-retryable + the ~60s ceiling are covered by the in-memory UNIT net
//                      (apps/api/tests/courier-relay-guard.test.ts); NOBYPASSRLS item-8 needs a DB-role
//                      harness (the predicate's BEGIN+set_config tenant tx is unit-asserted).
//
//   VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test \
//     e2e/tests/courier-room-authz-isolation.spec.ts --project=desktop --reporter=list
// ─────────────────────────────────────────────────────────────────────────────

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const WS_URL = BASE.replace(/^http/, 'ws');
const OWNER = { email: process.env.QA_OWNER_EMAIL || 'test@dowiz.com', password: process.env.QA_OWNER_PASSWORD || 'test123456' };
const SLUG = 'demo';
const QA_TAG = 'AUTOMATED-QA ADR-0013 — safe to purge';
const TTL_MS = 10_000; // courier-relay-guard fixed TTL — the honest eviction bound under DB availability.

test.describe.configure({ mode: 'serial' });
test.setTimeout(180_000);

let api: APIRequestContext;
let ownerToken = '';
let locationId = '';
let orderId = '';
let customerToken = '';
let c1Token = '', c1Id = '', c1Assignment = '';
let c2Token = '', c2Id = '';
const createdOrders: { id: string; token: string }[] = [];
const cleanupAssignments: { id: string; token: string }[] = [];

const ownerHdr = () => ({ authorization: `Bearer ${ownerToken}` });
const bearer = (t: string) => ({ authorization: `Bearer ${t}` });
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function collector(token: string, room: string) {
  const msgs: any[] = [];
  const ws = new WebSocket(`${WS_URL}/?token=${encodeURIComponent(token)}`);
  let opened = false;
  const ready = new Promise<void>((resolve) => {
    ws.on('open', () => { opened = true; ws.send(JSON.stringify({ type: 'subscribe', room })); resolve(); });
    ws.on('error', () => resolve());
  });
  ws.on('message', (d) => { try { msgs.push(JSON.parse(d.toString())); } catch { /* non-JSON */ } });
  const refsFor = (id: string) => msgs.filter((m) => JSON.stringify(m).includes(id)).length;
  const countType = (t: string) => msgs.filter((m) => m?.type === t || m?.data?.type === t).length;
  const errors = (needle: string) => msgs.filter((m) => m?.type === 'error' && JSON.stringify(m).includes(needle)).length;
  const sendClientLoc = () => { try { ws.send(JSON.stringify({ type: 'client_location', payload: { lat: 41.324, lng: 19.456 } })); } catch { /* closed */ } };
  return { ws, ready, msgs, refsFor, countType, errors, sendClientLoc, wasOpened: () => opened, close: () => { try { ws.close(); } catch { /* closed */ } } };
}

async function placeRealOrder(): Promise<{ orderId: string; customerToken: string }> {
  const menu = await (await api.get(`${BASE}/public/locations/${SLUG}/menu`)).json();
  const avail = (menu.categories ?? []).flatMap((c: any) => c.products ?? [])
    .filter((p: any) => p.available !== false && p.is_available !== false);
  // Priciest available product + enough quantity to clear the location's min-order floor (else 422
  // MIN_ORDER_NOT_MET); the cheapest demo item is 200, below the 500 minimum.
  const product = avail.sort((a: any, b: any) => (b.price ?? 0) - (a.price ?? 0))[0];
  expect(product, 'a real available product').toBeTruthy();
  const quantity = Math.max(1, Math.ceil(1000 / (product.price || 200)));
  // A UNIQUE phone per run — phone-velocity is per-number, so a reused QA phone eventually crosses
  // soft-confirm into a hard 429 throttle. Random keeps each run on the clean path.
  const phone = `+35569${crypto.randomInt(1_000_000, 9_999_999)}`;
  const post = (key: string, acknowledged_codes: string[]) => api.post(`${BASE}/api/orders`, {
    data: {
      locationId, type: 'delivery', items: [{ product_id: product.id, quantity }],
      customer: { phone, name: 'QA-ADR13' },
      delivery: { pin: { lat: 41.324, lng: 19.456 } },
      delivery_instructions: QA_TAG,
      payment: { method: 'cash' }, idempotency_key: key, acknowledged_codes,
    },
  });
  const track = (j: any) => { createdOrders.push({ id: j.id, token: j.authToken }); return { orderId: j.id, customerToken: j.authToken }; };
  const probe = await post(crypto.randomUUID(), []);
  if (probe.status() === 201) return track(await probe.json());
  const codes: string[] = ((await probe.json()).reasons ?? []).map((r: any) => r.code);
  const res = await post(crypto.randomUUID(), codes);
  expect(res.status(), `order create (soft-ack ${codes.join(',') || 'none'})`).toBe(201);
  return track(await res.json());
}

// Seed a REAL online courier (invite → redeem → shift). Returns null on rate-limit (429).
async function seedOnlineCourier(tag: string): Promise<{ token: string; id: string } | null> {
  const email = `qa-c13-${tag}-${Date.now()}@dowiz.dev`;
  const password = `Qa1!${crypto.randomUUID()}`;
  const inv = await (await api.post(`${BASE}/api/owner/locations/${locationId}/courier-invites`, { headers: ownerHdr(), data: { role: 'courier', email } })).json();
  expect(inv.code, `courier invite (${tag})`).toBeTruthy();
  const redRes = await api.post(`${BASE}/api/courier/auth/invites/${inv.inviteId}/redeem`, {
    data: { email, code: inv.code, password, full_name: `QA Courier ${tag}`, phone: '+355690000111' },
  });
  if (redRes.status() === 429) { console.log(`[courier ${tag}] seed rate-limited (429) — skipped`); return null; }
  const red = await redRes.json();
  expectJwt(red.jwt, `courier ${tag} redeem jwt (HTTP ${redRes.status()})`);
  const shift = await api.post(`${BASE}/api/courier/me/shift/start`, { headers: bearer(red.jwt), data: { lat: 41.324, lng: 19.456 } });
  expect(shift.ok(), `courier ${tag} shift start → ${shift.status()}`).toBeTruthy();
  return { token: red.jwt, id: red.courier.id };
}

async function assignmentIdFor(token: string, oid: string): Promise<string> {
  let found = '';
  await expect.poll(async () => {
    const list = await (await api.get(`${BASE}/api/courier/me/assignments`, { headers: bearer(token) })).json();
    const arr = Array.isArray(list) ? list : (list.assignments ?? []);
    const m = arr.find((a: any) => JSON.stringify(a).includes(oid));
    if (m) found = m.id ?? m.assignment_id ?? '';
    return !!m;
  }, { timeout: 10_000, message: 'courier received the assignment' }).toBe(true);
  return found;
}

test.beforeAll(async () => {
  api = await request.newContext();
  const login = await api.post(`${BASE}/api/auth/local/login`, { data: OWNER });
  expect(login.ok(), 'owner REAL login').toBeTruthy();
  const j = await login.json();
  ownerToken = j.access_token; locationId = j.activeLocationId;
  expect(ownerToken && locationId, 'owner token + tenant').toBeTruthy();

  ({ orderId, customerToken } = await placeRealOrder());
  expectUuid(orderId, 'order id');

  const c1 = await seedOnlineCourier('c1');
  test.skip(!c1, 'courier auth rate-limited this run — re-run after cooldown');
  ({ token: c1Token, id: c1Id } = c1!);
  const c2 = await seedOnlineCourier('c2');
  test.skip(!c2, 'second courier rate-limited — re-run after cooldown');
  ({ token: c2Token, id: c2Id } = c2!);

  // assign-courier requires CONFIRMED/PREPARING/READY. Best-effort confirm (a fresh order may already
  // be CONFIRMED → a same-status 409 is fine; we only need it OUT of PENDING).
  const conf = await api.patch(`${BASE}/api/orders/${orderId}/status`, { headers: ownerHdr(), data: { status: 'CONFIRMED' } });
  console.log(`[setup] confirm order → ${conf.status()}`);

  // Bind order-X to courier-1.
  const assign = await api.post(`${BASE}/api/owner/locations/${locationId}/orders/${orderId}/assign-courier`, { headers: ownerHdr(), data: { courierId: c1Id } });
  expect(assign.ok(), `assign order→c1 → ${assign.status()} ${assign.ok() ? '' : await assign.text()}`).toBeTruthy();
  c1Assignment = await assignmentIdFor(c1Token, orderId);
  expectUuid(c1Assignment, 'c1 assignment id');
  cleanupAssignments.push({ id: c1Assignment, token: c1Token });
});

test.afterAll(async () => {
  for (const a of cleanupAssignments) {
    const r = await api.post(`${BASE}/api/courier/assignments/${a.id}/cancel`, { headers: bearer(a.token), data: { reason: 'automated-qa-cleanup' } });
    console.log(`[cleanup] cancel assignment ${a.id} → ${r.status()}`);
  }
  for (const o of createdOrders) {
    const r = await api.post(`${BASE}/api/owner/locations/${locationId}/orders/${o.id}/reject`, { headers: ownerHdr(), data: { reason: 'automated-qa-cleanup' } });
    console.log(`[cleanup] reject ${o.id} → ${r.status()}`);
  }
});

test('POSITIVE CONTROL — courier-1 BOUND to order-X receives the guarded customer-GPS relay', async () => {
  const cust = collector(customerToken, `order:${orderId}`);
  const c1 = collector(c1Token, `order:${orderId}`);
  await Promise.all([cust.ready, c1.ready]);
  expect(c1.wasOpened() && cust.wasOpened(), 'both WS opened').toBe(true);
  await sleep(700);
  const before = c1.countType('client_location');
  // the customer streams GPS → the guarded relay forwards it to the BOUND courier member (fresh ALLOW).
  for (let i = 0; i < 4; i++) { cust.sendClientLoc(); await sleep(400); }
  await expect.poll(() => c1.countType('client_location'), { timeout: 10_000, message: 'bound courier receives the GPS relay' }).toBeGreaterThan(before);
  cust.close(); c1.close();
});

test('POSITIVE CONTROL (UI) — the REAL /courier/delivery/:id route renders + opens a WS (synthetic room banned)', async ({ page }) => {
  await page.addInitScript((tok) => localStorage.setItem('dos_access_token', tok as string), c1Token);
  const wsOpened: string[] = [];
  page.on('websocket', (ws) => wsOpened.push(ws.url()));
  await page.goto(`${BASE}/courier/delivery/${c1Assignment}`);
  // The page resolves the real order id from /courier/assignments/:id and renders the delivery surface.
  await expect(page.locator('[data-testid=courier-order-closed], [data-testid=courier-deliver-error], main, body').first()).toBeVisible({ timeout: 20_000 });
  await page.waitForTimeout(2_500);
  expect(wsOpened.some((u) => u.includes('/ws')), 'the real route opened a WS connection').toBe(true);
  await page.screenshot({ path: 'audit/adr13/positive-control-route.png' });
});

test('B6 / location: — a courier may NOT subscribe location:* (owner dashboard feed)', async () => {
  const intruder = collector(c1Token, `location:${locationId}`);
  await intruder.ready;
  await sleep(1_500);
  expect(intruder.refsFor(locationId), 'no location-room data leaked to a courier').toBe(0);
  expect(intruder.errors('Forbidden'), 'courier location subscribe is refused').toBeGreaterThan(0);
  intruder.close();
});

test('N3 (WS) — courier-2 (online, NOT bound to order-X) gets ZERO frames on order:<X>', async () => {
  const cust = collector(customerToken, `order:${orderId}`);
  const c2 = collector(c2Token, `order:${orderId}`);
  await Promise.all([cust.ready, c2.ready]);
  expect(c2.wasOpened(), 'courier-2 WS opened').toBe(true);
  await sleep(1_000);
  const before = c2.countType('client_location') + c2.refsFor(orderId);
  // the customer streams GPS + drive an order delta; the unbound colleague (subscribe DENIED) sees neither.
  for (let i = 0; i < 4; i++) { cust.sendClientLoc(); await sleep(300); }
  await api.patch(`${BASE}/api/orders/${orderId}/status`, { headers: ownerHdr(), data: { status: 'PREPARING' } });
  await sleep(3_000);
  expect(c2.countType('client_location') + c2.refsFor(orderId) - before, 'unbound colleague courier gets ZERO order-X frames').toBe(0);
  expect(c2.errors('Forbidden'), 'subscribe to a non-bound order is refused').toBeGreaterThan(0);
  cust.close(); c2.close();
});

test('N3 (REST) — courier-2 cannot read order-X message thread (assignment-scoped, not shop-wide)', async () => {
  const c1Read = await api.get(`${BASE}/api/orders/${orderId}/messages`, { headers: bearer(c1Token) });
  expect(c1Read.status(), 'bound courier-1 reads the thread = 200').toBe(200);
  const c2Read = await api.get(`${BASE}/api/orders/${orderId}/messages`, { headers: bearer(c2Token) });
  expect(c2Read.status(), 'unbound colleague courier-2 → 404 (hidden)').toBe(404);
  expect(await c2Read.text(), 'no order-X payload leaked').not.toContain(QA_TAG);
});

test('C1 — owner reassign order-X (c1→c2): courier-1 is evicted within ≤TTL (binding_revoked, zero post-expiry frames)', async () => {
  const cust = collector(customerToken, `order:${orderId}`);
  const c1 = collector(c1Token, `order:${orderId}`);
  await Promise.all([cust.ready, c1.ready]);
  await sleep(700);
  // prove c1 is receiving the guarded GPS relay (a fresh ALLOW is cached).
  for (let i = 0; i < 4; i++) { cust.sendClientLoc(); await sleep(300); }
  await expect.poll(() => c1.countType('client_location'), { timeout: 10_000, message: 'c1 receiving pre-reassign' }).toBeGreaterThan(0);

  // Reassign c1→c2. With the offer-handshake dark on staging the first assign drove the order to
  // IN_DELIVERY, which the owner reassign status-guard rejects; so terminalize c1's binding via the
  // courier-decline/cancel path (an ADR C1 variant — reverts the order to READY), THEN owner assigns c2.
  const cancel = await api.post(`${BASE}/api/courier/assignments/${c1Assignment}/cancel`, { headers: bearer(c1Token), data: { reason: 'qa-reassign-c1-to-c2' } });
  expect(cancel.ok(), `c1 binding cancel → ${cancel.status()} ${cancel.ok() ? '' : await cancel.text()}`).toBeTruthy();
  const re = await api.post(`${BASE}/api/owner/locations/${locationId}/orders/${orderId}/assign-courier`, { headers: ownerHdr(), data: { courierId: c2Id } });
  expect(re.ok(), `reassign order→c2 → ${re.status()} ${re.ok() ? '' : await re.text()}`).toBeTruthy();
  const c2Assignment = await assignmentIdFor(c2Token, orderId);
  cleanupAssignments.push({ id: c2Assignment, token: c2Token });

  // Wait past the fixed TTL (no frames) so c1's cached ALLOW expires (honest bound: ≤TTL under DB availability).
  await sleep(TTL_MS + 2_000);
  const c2 = collector(c2Token, `order:${orderId}`);
  await c2.ready; await sleep(700);

  const c1After = c1.countType('client_location');
  const c2Before = c2.countType('client_location');
  // Post-expiry trigger frames. The guard withholds each from c1 (stale cache → revalidate → DENY →
  // binding_revoked) and relays to the now-bound c2.
  for (let i = 0; i < 5; i++) { cust.sendClientLoc(); await sleep(500); }
  // c1 is evicted: receives a binding_revoked error and NO new GPS frame (incl. the trigger frames).
  await expect.poll(() => c1.errors('binding_revoked'), { timeout: 12_000, message: 'c1 gets binding_revoked' }).toBeGreaterThan(0);
  await sleep(1_500);
  expect(c1.countType('client_location') - c1After, 'zero post-expiry frames reach the reassigned courier (incl. the trigger)').toBe(0);
  // c2 (now bound) DOES receive the relay — proves the room still broadcasts; only c1 was revoked.
  await expect.poll(() => c2.countType('client_location'), { timeout: 12_000, message: 'c2 receives after reassign' }).toBeGreaterThan(c2Before);
  cust.close(); c1.close(); c2.close();
});

// NOTE — two ADR-0013 dimensions are not staging-inducible and are covered off-staging:
//  - UNAVAILABLE→retryable + the in-memory ~60s wall ceiling: apps/api/tests/courier-relay-guard.test.ts
//    (mutant drop-ceiling → 3 fail) + courier-room-authz.test.ts (connect/begin/select → UNAVAILABLE).
//  - NOBYPASSRLS deny-all-couriers regression (item-8): the predicate's BEGIN+set_config tenant tx shape
//    is unit-asserted; a forced-NOBYPASSRLS DB harness is tracked as a follow-up.
