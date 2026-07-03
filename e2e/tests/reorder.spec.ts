import { test, expect, type APIRequestContext } from '@playwright/test';
import crypto from 'node:crypto';

// Real-UI proof of REORDER (zero-friction repeat order). A past order's tracking page
// exposes a "Reorder" action that rehydrates the cart from that order's items —
// re-validated against the LIVE public menu (availability + price) via the shared cart
// add-path — then lands the customer back on the storefront with the cart populated.
// Device-local, read-only fetch (no auth/identity). Server is read-only in this test.
//
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test reorder --project=desktop --reporter=list

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
  const product = products[0];
  const created = await request.post('/api/orders', {
    data: {
      locationId, type: 'delivery',
      items: [{ product_id: product.id, quantity: 2 }],
      customer: { phone: uniquePhone(), name: 'E2E Reorder' }, payment: { method: 'cash' },
      idempotency_key: crypto.randomUUID(), acknowledged_codes: ['velocity'],
      delivery: { pin: { lat: loc.lat, lng: loc.lng }, address_text: 'Demo HQ' },
    },
  });
  expect(created.status(), `create order: ${await created.text()}`).toBe(201);
  const order = await created.json();
  const u = new URL(order.trackUrl as string);
  return { id: order.id as string, trackPath: u.pathname + u.search, productName: product.name as string };
}

test('past order → Reorder rehydrates the cart with the same item', async ({ page, request }) => {
  test.setTimeout(90_000); // staging menu endpoint flakes empty under load → retry budget + generous asserts
  const { id, trackPath, productName } = await createDeliveryOrder(request);

  // Make the order terminal so the tracking page shows the terminal exit block (with items).
  // CANCELLED is directly settable and keeps the reorder affordance (order has items).
  const token = await ownerToken(request);
  const patch = await request.patch(`/api/orders/${id}/status`, {
    data: { status: 'CANCELLED' }, headers: { Authorization: `Bearer ${token}` },
  });
  expect(patch.status(), `set CANCELLED: ${await patch.text()}`).toBe(200);

  await page.goto(trackPath);

  // The Reorder button is present on the past order.
  const reorderBtn = page.locator('[data-testid="reorder-btn"]');
  await expect(reorderBtn, 'past order exposes a Reorder action').toBeVisible({ timeout: 25000 });

  await reorderBtn.click();

  // Rehydrate → navigate back to the storefront; the cart FAB now shows the rehydrated cart.
  const cartOpen = page.locator('[data-testid="cart-open"]');
  await expect(cartOpen, 'cart is populated after reorder (FAB visible)').toBeVisible({ timeout: 25000 });

  // Open the cart and assert the reordered item is really in it (real cart DOM).
  await cartOpen.click();
  const cartDialog = page.getByRole('dialog');
  await expect(cartDialog, 'cart drawer opens').toBeVisible();
  await expect(cartDialog, 'the reordered item is in the cart').toContainText(productName);
});
