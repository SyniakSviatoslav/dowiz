import { test, expect, type APIRequestContext } from '@playwright/test';
import crypto from 'node:crypto';
<<<<<<< Updated upstream
=======
import { expectJwt, expectUuid } from '../../helpers/assert-shape';
import { requireStaging } from '../../helpers/staging-guard';
>>>>>>> Stashed changes

// Order-status stepper proof (testplan §2a) against deployed staging.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test order-stepper --project=desktop --reporter=list
//
// Places a real delivery order on the demo storefront, opens the customer tracking link
// (self-authenticating via the ?t= grant — no login needed), asserts the delivery-branch
// stepper, then advances the order to CONFIRMED as owner and proves the step lights up.
const CREDS = { email: 'test@dowiz.com', password: 'test123456' };
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

// These tests MUTATE state (place real orders, drive the lifecycle). Fail fast against a
// prod/unknown target rather than writing to prod.
test.beforeAll(() => requireStaging(BASE));

// /api/orders is rate-limited per phone and login per IP; keep a unique phone per order
// and memoize one owner token per worker.
let cachedToken: string | null = null;
async function ownerToken(request: APIRequestContext): Promise<string> {
  if (cachedToken) return cachedToken;
  const res = await request.post('/api/auth/local/login', { data: CREDS });
  expect(res.ok(), 'owner login should succeed').toBeTruthy();
  cachedToken = (await res.json()).access_token as string;
  expect(cachedToken, 'login returns an access token').toBeTruthy();
  return cachedToken;
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
  // Pin the delivery at the venue's own coordinates so it is always within range.
  return { locationId: m.locationId ?? m.location_id, lat: loc.lat, lng: loc.lng, productId: products[0].id };
}

// Each order POST is rate-limited per-phone (max 5/min, apps/api/src/routes/orders.ts:67),
// so every order needs a genuinely distinct phone. A monotonic counter guarantees that even
// for back-to-back serial calls (Date.now() alone can collide at ms resolution).
let phoneSeq = 0;
function uniquePhone() {
  phoneSeq += 1;
  return `+35563${String(Date.now()).slice(-4)}${String(phoneSeq).padStart(2, '0')}`;
}

// Place a real order of the given type and return its id + self-authenticating track path.
// Pickup orders carry no delivery pin/address (apps/api/src/routes/orders.ts:91).
async function createOrder(
  request: APIRequestContext,
  type: 'delivery' | 'pickup',
  t: { locationId: string; lat: number; lng: number; productId: string },
) {
  const body: Record<string, unknown> = {
    locationId: t.locationId,
    type,
    items: [{ product_id: t.productId, quantity: 1 }],
    customer: { phone: uniquePhone(), name: 'E2E Stepper' },
    payment: { method: 'cash' },
    idempotency_key: crypto.randomUUID(),
    // Anti-fraud: several orders from one test-runner IP trip the soft `velocity` signal.
    // The real storefront resolves it by confirming the "please confirm" dialog, which
    // re-submits with acknowledged_codes — pre-acknowledge here so the proof models that
    // flow in a single POST (and stays within the per-phone rate budget).
    acknowledged_codes: ['velocity'],
  };
  if (type === 'delivery') body.delivery = { pin: { lat: t.lat, lng: t.lng }, address_text: 'Demo HQ' };

  const created = await request.post('/api/orders', { data: body });
  expect(created.status(), `create ${type} order: ${await created.text()}`).toBe(201);
  const order = await created.json();
  expectUuid(order.id, 'created order id');
  const trackUrl = new URL(order.trackUrl as string);
  return { id: order.id as string, trackPath: trackUrl.pathname + trackUrl.search };
}

test('order-status stepper renders the delivery branch and lights up on CONFIRMED', async ({ page, request }) => {
  const target = await demoTarget(request);
  const { id, trackPath } = await createOrder(request, 'delivery', target);

  // Customer opens the tracking link; the page exchanges ?t= for a session and renders.
  await page.goto(trackPath);
  const progress = page.locator('[data-testid="order-progress"]');
  await expect(progress).toBeVisible({ timeout: 25000 });
  await expect(progress).toHaveAttribute('data-order-type', 'delivery');

  // PENDING is reached; CONFIRMED is not yet active.
  await expect(page.locator('[data-testid="order-step-pending"]')).toHaveAttribute('data-active', 'true');
  await expect(page.locator('[data-testid="order-step-confirmed"]')).toHaveAttribute('data-active', 'false');

  // Delivery branch is rendered; the pickup-only step is absent.
  await expect(page.locator('[data-testid="order-step-in_delivery"]')).toBeVisible();
  await expect(page.locator('[data-testid="order-step-delivered"]')).toBeVisible();
  await expect(page.locator('[data-testid="order-step-picked_up"]')).toHaveCount(0);

  // Owner advances the order to CONFIRMED via the state machine.
  const token = await ownerToken(request);
  const patch = await request.patch(`/api/orders/${id}/status`, {
    data: { status: 'CONFIRMED' },
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(patch.status(), `confirm: ${await patch.text()}`).toBe(200);

  // Real-time proof (Test Integrity §8): the still-open tracking page must light up the
  // CONFIRMED step from the live WS push alone — NO reload. A reload here would mask a dead
  // realtime channel by re-fetching over HTTP. The page is anchored to THIS order id.
  // TODO(needs_staging): only a live staging run exercises the WS push; if the realtime
  // channel is down this assertion (correctly) goes red instead of being papered over by reload.
  await expect(page.locator('[data-testid="order-step-confirmed"]')).toHaveAttribute('data-active', 'true', { timeout: 25000 });
  await expect(page.locator('[data-testid="order-step-confirmed-time"]')).toBeVisible();
});

// Testplan §2b — a pickup order renders the READY→PICKED_UP branch (no delivery tail).
test('order-status stepper renders the pickup branch (picked_up, no delivery steps)', async ({ page, request }) => {
  const target = await demoTarget(request);
  const { trackPath } = await createOrder(request, 'pickup', target);

  await page.goto(trackPath);
  const progress = page.locator('[data-testid="order-progress"]');
  await expect(progress).toBeVisible({ timeout: 25000 });
  await expect(progress).toHaveAttribute('data-order-type', 'pickup');

  // Pickup tail present; delivery-only steps absent.
  await expect(page.locator('[data-testid="order-step-picked_up"]')).toBeVisible();
  await expect(page.locator('[data-testid="order-step-in_delivery"]')).toHaveCount(0);
  await expect(page.locator('[data-testid="order-step-delivered"]')).toHaveCount(0);
});

// Testplan §2c — a REJECTED order short-circuits to the terminal rejected node (active).
test('order-status stepper shows the terminal REJECTED node', async ({ page, request }) => {
  const target = await demoTarget(request);
  const { id, trackPath } = await createOrder(request, 'delivery', target);

  const token = await ownerToken(request);
  const patch = await request.patch(`/api/orders/${id}/status`, {
    data: { status: 'REJECTED' },
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(patch.status(), `reject: ${await patch.text()}`).toBe(200);

  await page.goto(trackPath);
  const rejected = page.locator('[data-testid="order-step-rejected"]');
  await expect(rejected).toBeVisible({ timeout: 25000 });
  await expect(rejected).toHaveAttribute('data-active', 'true');
});

// Testplan §2d — a CANCELLED order short-circuits to the terminal cancelled node (active).
test('order-status stepper shows the terminal CANCELLED node', async ({ page, request }) => {
  const target = await demoTarget(request);
  const { id, trackPath } = await createOrder(request, 'delivery', target);

  const token = await ownerToken(request);
  const patch = await request.patch(`/api/orders/${id}/status`, {
    data: { status: 'CANCELLED' },
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(patch.status(), `cancel: ${await patch.text()}`).toBe(200);

  await page.goto(trackPath);
  const cancelled = page.locator('[data-testid="order-step-cancelled"]');
  await expect(cancelled).toBeVisible({ timeout: 25000 });
  await expect(cancelled).toHaveAttribute('data-active', 'true');
});

// Negative auth controls (Test Integrity §4) on PATCH /api/orders/:id/status. The positive
// control (valid owner → 200) is proven by the §2a/§2c/§2d tests above; here we prove the
// gate actually rejects unauthenticated and bad-token callers rather than letting everyone in.
// Statuses verified against apps/api/src/plugins/auth.ts verifyAuth (401) and the route at
// apps/api/src/routes/orders.ts:749 (withTenant RLS → 404 for a non-owning tenant).
test('PATCH order status rejects unauthenticated and bad-token callers', async ({ request }) => {
  const target = await demoTarget(request);
  const { id } = await createOrder(request, 'delivery', target);

  // (1) No Authorization header → 401.
  const noAuth = await request.patch(`/api/orders/${id}/status`, { data: { status: 'CONFIRMED' } });
  expect(noAuth.status(), 'bare PATCH must be 401').toBe(401);

  // (2) Malformed bearer token → 401 (token verification fails).
  const badToken = await request.patch(`/api/orders/${id}/status`, {
    data: { status: 'CONFIRMED' },
    headers: { Authorization: 'Bearer not.a.real.jwt' },
  });
  expect(badToken.status(), 'garbage-token PATCH must be 401').toBe(401);

  // TODO(needs_staging): a wrong-tenant owner (real second tenant's owner JWT) must get 404
  // (withTenant RLS hides the order, orders.ts:773-775) — NOT a nil-UUID (§5). Needs a second
  // seeded owner fixture; add once staging exposes one.
});

// Self-authenticating ?t= track grant — negative control (Test Integrity §10). An unknown/forged
// code must be refused with the single opaque 410 (apps/api/src/routes/customer/track.ts:55-62);
// the grant is reusable-by-design until expiry (track.ts:66-67) so single-use is NOT asserted.
test('track-exchange refuses an unknown ?t= code with 410', async ({ request }) => {
  const forged = crypto.randomBytes(32).toString('base64url'); // valid shape, no matching grant
  const res = await request.post('/api/customer/track/exchange', { data: { code: forged } });
  expect(res.status(), 'unknown track code must be 410').toBe(410);

  // TODO(needs_staging): cross-order IDOR — a grant minted for order A must not read order B's
  // status. Needs a real second order from a DIFFERENT customer (distinct customer_id) on a live
  // run; the issued customer JWT is order/customer-scoped (track.ts:75-79). Do not fake with nil-UUID.
});
