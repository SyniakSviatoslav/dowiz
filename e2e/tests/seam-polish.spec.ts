import { test, expect, type APIRequestContext } from '@playwright/test';
import crypto from 'node:crypto';

// Real-UI proof of the final seam-polish fixes (server read-only). Focus: no order-lifecycle
// terminal state is a dead-end — REJECTED/CANCELLED tracking pages offer an "Order again" exit
// and a humane, non-accusing reason (F3). Reuses the order-stepper harness.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test seam-polish --project=desktop --reporter=list
const CREDS = { email: 'test@dowiz.com', password: 'test123456' };
let cachedToken: string | null = null;
async function ownerToken(request: APIRequestContext): Promise<string> {
  if (cachedToken) return cachedToken;
  const res = await request.post('/api/auth/local/login', { data: CREDS });
  expect(res.ok(), 'owner login').toBeTruthy();
  cachedToken = (await res.json()).access_token as string;
  return cachedToken;
}
let phoneSeq = 0;
const uniquePhone = () => `+35562${String(Date.now()).slice(-4)}${String(++phoneSeq).padStart(2, '0')}`;

async function createDeliveryOrder(request: APIRequestContext) {
  const info = await request.get('/public/locations/demo/info', { headers: { accept: 'application/json' } });
  const loc = await info.json();
  // The public menu endpoint intermittently returns empty/5xx under a request burst on staging
  // (known flake; curl is consistently 200) — retry until it has products.
  let products: any[] = [];
  let locationId: string | undefined;
  for (let i = 0; i < 6 && products.length === 0; i++) {
    const menu = await request.get('/public/locations/demo/menu', { headers: { accept: 'application/json' } });
    if (menu.ok()) {
      const m = await menu.json();
      products = (m.categories ?? []).flatMap((c: any) => c.products ?? []);
      if (products.length) { locationId = m.locationId ?? m.location_id; break; }
    }
    await new Promise((r) => setTimeout(r, 1500));
  }
  expect(products.length, 'demo menu has products').toBeGreaterThan(0);
  const productId = products[0].id;
  const created = await request.post('/api/orders', {
    data: {
      locationId, type: 'delivery',
      items: [{ product_id: productId, quantity: 1 }],
      customer: { phone: uniquePhone(), name: 'E2E Seam' }, payment: { method: 'cash' },
      idempotency_key: crypto.randomUUID(), acknowledged_codes: ['velocity'],
      delivery: { pin: { lat: loc.lat, lng: loc.lng }, address_text: 'Demo HQ' },
    },
  });
  expect(created.status(), `create order: ${await created.text()}`).toBe(201);
  const order = await created.json();
  const u = new URL(order.trackUrl as string);
  return { id: order.id as string, trackPath: u.pathname + u.search };
}

for (const terminal of ['CANCELLED', 'REJECTED'] as const) {
  test(`terminal ${terminal} order is not a dead-end — offers "Order again" (F3)`, async ({ page, request }) => {
    test.setTimeout(90_000); // staging menu endpoint flakes empty under load → retry budget + 25s assert
    const { id, trackPath } = await createDeliveryOrder(request);
    const token = await ownerToken(request);
    const patch = await request.patch(`/api/orders/${id}/status`, {
      data: { status: terminal }, headers: { Authorization: `Bearer ${token}` },
    });
    expect(patch.status(), `set ${terminal}: ${await patch.text()}`).toBe(200);

    await page.goto(trackPath);
    const exit = page.locator('[data-testid="order-terminal-exit"]');
    await expect(exit, 'terminal state shows an exit block, not a dead-end').toBeVisible({ timeout: 25000 });
    const again = page.locator('[data-testid="order-again"]');
    await expect(again, '"Order again" CTA is present').toBeVisible();
    expect(await again.getAttribute('href'), 'reorder routes back to the storefront').toContain('/s/');
  });
}
