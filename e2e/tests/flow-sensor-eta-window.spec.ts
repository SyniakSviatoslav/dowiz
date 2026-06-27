import { test, expect, type APIRequestContext } from '@playwright/test';
import crypto from 'node:crypto';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

/**
 * SENSOR-BUS §1.1 runtime proof (ADR-0009 v4) — the ETA-window synthesis.
 * Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test flow-sensor-eta-window --reporter=list
 *
 * Proves the two customer-facing DoD micro-assertions of the §1.1 ship-blocker:
 *   1. range-never-point — promisedWindow + liveEta are always [lo,hi] with lo<hi AND hi<=eta_cap (90).
 *   2. set-once frozen promise — promisedWindow is written ONCE at CONFIRMED and never changes as the
 *      order advances (the customer's live truth channel, liveEta, may still move).
 *
 * API-level against deployed staging: owner drives the state machine; the customer reads the order
 * via the ?t= track grant exchanged for a real customer JWT (mirrors the storefront tracking link).
 */

const CREDS = { email: 'test@dowiz.com', password: 'test123456' };
const ETA_CAP_MIN = 90; // locations.eta_cap_min default (migration 066)

test.describe.configure({ mode: 'serial' });

// MUTATING spec (places real orders + drives the state machine) — fail fast unless the
// target is an explicit staging/local host. Never write to prod from a test.
test.beforeAll(() => requireStaging(process.env.VITE_BASE_URL));

let ownerTok: string | null = null;
async function ownerToken(request: APIRequestContext): Promise<string> {
  if (ownerTok) return ownerTok;
  const res = await request.post('/api/auth/local/login', { data: CREDS });
  expect(res.ok(), `owner login: ${await res.text()}`).toBeTruthy();
  ownerTok = (await res.json()).access_token as string;
  expectJwt(ownerTok, 'owner access_token');
  return ownerTok;
}

async function demoTarget(request: APIRequestContext) {
  const [info, menu] = await Promise.all([
    request.get('/public/locations/demo/info', { headers: { accept: 'application/json' } }),
    request.get('/public/locations/demo/menu', { headers: { accept: 'application/json' } }),
  ]);
  expect(info.ok(), 'demo info loads').toBeTruthy();
  expect(menu.ok(), 'demo menu loads').toBeTruthy();
  const loc = await info.json();
  const m = await menu.json();
  const products: any[] = (m.categories ?? []).flatMap((c: any) => c.products ?? []);
  expect(products.length, 'demo has products').toBeGreaterThan(0);
  return { locationId: m.locationId ?? m.location_id, lat: loc.lat, lng: loc.lng, productId: products[0].id };
}

let phoneSeq = 0;
function uniquePhone() {
  phoneSeq += 1;
  return `+35564${String(Date.now()).slice(-4)}${String(phoneSeq).padStart(2, '0')}`;
}

// Place a real delivery order; return its id + the customer JWT (via track-grant exchange).
async function placeOrderAndAuthCustomer(request: APIRequestContext, t: { locationId: string; lat: number; lng: number; productId: string }) {
  const created = await request.post('/api/orders', {
    data: {
      locationId: t.locationId,
      type: 'delivery',
      items: [{ product_id: t.productId, quantity: 1 }],
      customer: { phone: uniquePhone(), name: 'E2E Sensor' },
      delivery: { pin: { lat: t.lat, lng: t.lng }, address_text: 'Demo HQ' },
      payment: { method: 'cash' },
      idempotency_key: crypto.randomUUID(),
      acknowledged_codes: ['velocity'],
    },
  });
  expect(created.status(), `create order: ${await created.text()}`).toBe(201);
  const order = await created.json();
  const code = new URL(order.trackUrl as string).searchParams.get('t');
  expect(code, 'track grant code present in trackUrl').toBeTruthy();

  const ex = await request.post('/api/customer/track/exchange', { data: { code } });
  expect(ex.ok(), `track exchange: ${await ex.text()}`).toBeTruthy();
  const customerTok = (await ex.json()).token as string;
  expectJwt(customerTok, 'customer token');
  expectUuid(order.id, 'created order id');
  return { id: order.id as string, customerTok };
}

async function readCustomerStatus(request: APIRequestContext, id: string, tok: string) {
  const res = await request.get(`/api/customer/orders/${id}/status`, { headers: { Authorization: `Bearer ${tok}` } });
  expect(res.status(), `customer status: ${await res.text()}`).toBe(200);
  return res.json();
}

async function patchStatus(request: APIRequestContext, id: string, status: string, tok: string) {
  const res = await request.patch(`/api/orders/${id}/status`, { data: { status }, headers: { Authorization: `Bearer ${tok}` } });
  expect(res.status(), `${status}: ${await res.text()}`).toBe(200);
}

function assertRangeNeverPoint(w: { loMin: number; hiMin: number } | null, label: string) {
  expect(w, `${label} should be present`).toBeTruthy();
  expect(Number.isInteger(w!.loMin) && Number.isInteger(w!.hiMin), `${label} bounds are integers`).toBeTruthy();
  expect(w!.loMin, `${label} lo >= 1`).toBeGreaterThanOrEqual(1);
  expect(w!.hiMin, `${label} is a real band (hi>lo)`).toBeGreaterThan(w!.loMin);
  expect(w!.hiMin, `${label} hi within absolute eta_cap`).toBeLessThanOrEqual(ETA_CAP_MIN);
}

test('promised_window + live_eta are set at CONFIRMED, both honest ranges within the cap', async ({ request }) => {
  const t = await demoTarget(request);
  const { id, customerTok } = await placeOrderAndAuthCustomer(request, t);

  // Before confirm: no frozen promise yet (PENDING). The compute-on-read etaRange may exist; the
  // persisted window is null until the order is confirmed.
  const pending = await readCustomerStatus(request, id, customerTok);
  expect(pending.promisedWindow, 'no frozen promise before confirm').toBeNull();
  // Synthesis is gated on CONFIRMED: the live channel must also be null while PENDING — the
  // feature must not synthesise a window before the order is confirmed.
  expect(pending.liveEta, 'no live window before confirm (synthesis gated on CONFIRMED)').toBeNull();

  const owner = await ownerToken(request);
  await patchStatus(request, id, 'CONFIRMED', owner);

  const confirmed = await readCustomerStatus(request, id, customerTok);
  assertRangeNeverPoint(confirmed.promisedWindow, 'promisedWindow@CONFIRMED');
  assertRangeNeverPoint(confirmed.liveEta, 'liveEta@CONFIRMED');
});

test('promised_window is FROZEN (set-once) while live_eta stays a valid range across stages', async ({ request }) => {
  const t = await demoTarget(request);
  const { id, customerTok } = await placeOrderAndAuthCustomer(request, t);
  const owner = await ownerToken(request);

  await patchStatus(request, id, 'CONFIRMED', owner);
  const s1 = await readCustomerStatus(request, id, customerTok);
  const frozen = s1.promisedWindow;
  assertRangeNeverPoint(frozen, 'promisedWindow@CONFIRMED');

  // Advance through prep + ready — the frozen promise must NOT move; the live channel stays valid.
  for (const stage of ['PREPARING', 'READY']) {
    await patchStatus(request, id, stage, owner);
    const s = await readCustomerStatus(request, id, customerTok);
    expect(s.promisedWindow, `promisedWindow immutable @${stage}`).toEqual(frozen);
    assertRangeNeverPoint(s.liveEta, `liveEta@${stage}`);
  }
});

// Access controls on the customer-facing tracking surface — a customer token is a narrow grant,
// not a skeleton key. Negative controls (no-token 401, cross-order IDOR 404, role-escalation 403)
// alongside the positive control already proven above (valid token → 200 in readCustomerStatus).
test('customer status + order-status mutation enforce auth, ownership, and role', async ({ request }) => {
  const t = await demoTarget(request);
  const a = await placeOrderAndAuthCustomer(request, t);
  const b = await placeOrderAndAuthCustomer(request, t);

  // (2) No Authorization header → the auth guard must reject (401), not leak the order.
  const noAuth = await request.get(`/api/customer/orders/${a.id}/status`);
  expect(noAuth.status(), `no-token status: ${await noAuth.text()}`).toBe(401);

  // (1) IDOR: customer A's token must NOT read customer B's order. The route scopes by
  //     customer_id, so a foreign order is absent → 404 (route: customer/orders.ts:48-52).
  const crossAB = await request.get(`/api/customer/orders/${b.id}/status`, {
    headers: { Authorization: `Bearer ${a.customerTok}` },
  });
  expect(crossAB.status(), `A reads B: ${await crossAB.text()}`).toBe(404);
  const crossBA = await request.get(`/api/customer/orders/${a.id}/status`, {
    headers: { Authorization: `Bearer ${b.customerTok}` },
  });
  expect(crossBA.status(), `B reads A: ${await crossBA.text()}`).toBe(404);

  // (3) Privilege escalation: a customer-role JWT must NOT drive the owner state machine.
  //     requireRole(['owner']) on PATCH /orders/:id/status → 403 (route: orders.ts:750,761).
  const escalate = await request.patch(`/api/orders/${a.id}/status`, {
    data: { status: 'CONFIRMED' },
    headers: { Authorization: `Bearer ${a.customerTok}` },
  });
  expect(escalate.status(), `customer PATCH status: ${await escalate.text()}`).toBe(403);
});
