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

test('order-status stepper renders the delivery branch and lights up on CONFIRMED', async ({ page, request }) => {
  const { locationId, lat, lng, productId } = await demoTarget(request);

  const created = await request.post('/api/orders', {
    data: {
      locationId,
      type: 'delivery',
      items: [{ product_id: productId, quantity: 1 }],
      customer: { phone: `+35563${String(Date.now()).slice(-6)}`, name: 'E2E Stepper' },
      delivery: { pin: { lat, lng }, address_text: 'Demo HQ' },
      payment: { method: 'cash' },
      idempotency_key: crypto.randomUUID(),
    },
  });
  expect(created.status(), `create order: ${await created.text()}`).toBe(201);
  const order = await created.json();
  const trackUrl = new URL(order.trackUrl as string);
  const trackPath = trackUrl.pathname + trackUrl.search; // /s/demo/order/:id?t=<grant>

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
  const patch = await request.patch(`/api/orders/${order.id}/status`, {
    data: { status: 'CONFIRMED' },
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(patch.status(), `confirm: ${await patch.text()}`).toBe(200);

  // The ?t= grant is single-use and already stripped; the stored session re-fetches on reload.
  await page.reload();
  await expect(page.locator('[data-testid="order-step-confirmed"]')).toHaveAttribute('data-active', 'true', { timeout: 25000 });
  await expect(page.locator('[data-testid="order-step-confirmed-time"]')).toBeVisible();
});
