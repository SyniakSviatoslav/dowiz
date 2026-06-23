import { test, expect, type APIRequestContext } from '@playwright/test';
import crypto from 'node:crypto';

// Order-status stepper proof (testplan §2a) against deployed staging.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test order-stepper --project=desktop --reporter=list
//
// Places a real delivery order on the demo storefront, opens the customer tracking link
// (self-authenticating via the ?t= grant — no login needed), asserts the delivery-branch
// stepper, then advances the order to CONFIRMED as owner and proves the step lights up.
const CREDS = { email: 'test@dowiz.com', password: 'test123456' };

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

  // The ?t= grant is single-use and already stripped; the stored session re-fetches on reload.
  await page.reload();
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
