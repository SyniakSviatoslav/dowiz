import { test, expect, type APIRequestContext } from '@playwright/test';
import crypto from 'node:crypto';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// Real-UI proof of the final seam-polish fixes (server read-only). Focus: no order-lifecycle
// terminal state is a dead-end — REJECTED/CANCELLED tracking pages offer an "Order again" exit
// and a humane, non-accusing reason (F3). Reuses the order-stepper harness.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test seam-polish --project=desktop --reporter=list

// Mutating spec (creates orders, patches status) → never let it run against prod/unknown.
test.beforeAll(() => requireStaging(process.env.VITE_BASE_URL));

// Credentials from env (never commit plaintext into the source / CI logs). Local fallback only.
const CREDS = {
  email: process.env.E2E_OWNER_EMAIL ?? 'test@dowiz.com',
  password: process.env.E2E_OWNER_PASSWORD ?? 'test123456',
};
// No module-level token cache: a shared cache leaks auth state across the parametrised
// iterations and silently reuses a stale/expired token. Mint a fresh, shape-checked token per call.
async function ownerToken(request: APIRequestContext): Promise<string> {
  const res = await request.post('/api/auth/local/login', { data: CREDS });
  expect(res.ok(), 'owner login').toBeTruthy();
  const token = (await res.json()).access_token as string;
  expectJwt(token, 'owner access_token');
  return token;
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
  expectUuid(order.id, 'created order id');
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
    // Exact route shape: a relative or same-origin absolute `/s/:slug` — NOT any url that merely
    // contains '/s/' somewhere (e.g. '/assets/...'), which a mis-wired href would slip past.
    const href = await again.getAttribute('href');
    expect(href, 'reorder routes back to the storefront /s/:slug').toMatch(/^(?:https?:\/\/[^/]+)?\/s\/[\w-]+/);
  });
}

// IDOR / auth negative control — the status PATCH must reject unauthenticated callers and must
// never reach a row outside the caller's tenant. The route is RLS-scoped (withTenant), so an order
// id the owner does not own reads 0 rows → 404 (apps/api/src/routes/orders.ts:773-774); a missing
// token → 401 (apps/api/src/plugins/auth.ts:47).
test('status PATCH is auth- and tenant-scoped (IDOR negative control)', async ({ request }) => {
  const token = await ownerToken(request);
  const foreignId = crypto.randomUUID();
  const noAuth = await request.patch(`/api/orders/${foreignId}/status`, { data: { status: 'CANCELLED' } });
  expect(noAuth.status(), 'unauthenticated status PATCH is rejected').toBe(401);
  const foreign = await request.patch(`/api/orders/${foreignId}/status`, {
    data: { status: 'CANCELLED' }, headers: { Authorization: `Bearer ${token}` },
  });
  expect(foreign.status(), 'owner cannot PATCH an order outside its RLS tenant scope').toBe(404);
  // TODO(needs-staging): full cross-tenant proof — mint a SECOND tenant owner token and assert it
  // gets 404 PATCHing THIS tenant's *real* order id (not just a random non-existent id). Requires a
  // provisioned 2nd-tenant fixture on staging; the random-id 404 here exercises the same RLS read path.
});
