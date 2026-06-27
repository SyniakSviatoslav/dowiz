import { test, expect, type APIRequestContext } from '@playwright/test';
import crypto from 'node:crypto';
import { requireStaging } from '../helpers/staging-guard';

// These specs MUTATE state (they place real orders), so refuse to run against prod/unknown.
const BASE = process.env.VITE_BASE_URL ?? 'https://dowiz-staging.fly.dev';
test.beforeAll(() => requireStaging(BASE));

// Real-UI proof for the polish-debt round's browser-facing seams (server read-only):
//   F12 — a dedicated sr-only role="status" region announces the order status to screen readers.
//   F13 — a mid-journey refresh restores the live order (continuity), not a "Not Found" dead-end.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test polish-debt --project=desktop --reporter=list
const CREDS = { email: 'test@dowiz.com', password: 'test123456' };
let phoneSeq = 0;
const uniquePhone = () => `+35562${String(Date.now()).slice(-4)}${String(++phoneSeq).padStart(2, '0')}`;

async function createDeliveryOrder(request: APIRequestContext) {
  const info = await request.get('/public/locations/demo/info', { headers: { accept: 'application/json' } });
  const loc = await info.json();
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
  // Pick the priciest product so the order clears the venue's minimum-order value.
  const product = products.slice().sort((a, b) => (b.price ?? 0) - (a.price ?? 0))[0];
  // Order-create transiently 503s on staging under pool pressure (known) — retry a few times.
  let created!: Awaited<ReturnType<typeof request.post>>;
  for (let i = 0; i < 5; i++) {
    created = await request.post('/api/orders', {
      data: {
        locationId, type: 'delivery',
        items: [{ product_id: product.id, quantity: 1 }],
        customer: { phone: uniquePhone(), name: 'E2E Polish' }, payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(), acknowledged_codes: ['velocity'],
        delivery: { pin: { lat: loc.lat, lng: loc.lng }, address_text: 'Demo HQ' },
      },
    });
    if (created.status() !== 503) break;
    await new Promise((r) => setTimeout(r, 1500));
  }
  expect(created.status(), `create order: ${await created.text()}`).toBe(201);
  const order = await created.json();
  const u = new URL(order.trackUrl as string);
  return { id: order.id as string, trackPath: u.pathname + u.search };
}

test('F12: order status is announced via a dedicated sr-only role="status" region', async ({ page, request }) => {
  test.setTimeout(90_000);
  const { trackPath } = await createDeliveryOrder(request);
  await page.goto(trackPath);

  const announcer = page.locator('[data-testid="sr-status-announcer"]');
  await expect(announcer, 'a dedicated status announcer exists').toBeAttached({ timeout: 25000 });
  await expect(announcer).toHaveAttribute('role', 'status');
  await expect(announcer).toHaveAttribute('aria-live', 'polite');
  // Speaks the current status as explicit, localized text (prefix + label) — never empty.
  await expect(announcer, 'announcer carries the spoken status text').not.toBeEmpty();
});

test('F13: refreshing mid-journey restores the live order, not a "Not Found" dead-end', async ({ page, request }) => {
  test.setTimeout(90_000);
  const { id, trackPath } = await createDeliveryOrder(request);
  // The order-details heading renders `#<first-4-of-id>` in every locale, so it's a
  // stable, order-specific fingerprint — distinguishes "same order rehydrated" from a
  // status-string-only stub.
  const idFingerprint = page.getByRole('heading', { name: new RegExp(id.slice(0, 4), 'i') });
  const notFound = page.locator('[data-testid="order-back-to-menu"]');

  await page.goto(trackPath);
  const badge = page.locator('[data-testid="order-status-badge"]');
  await expect(badge, 'order status renders on first load').toBeVisible({ timeout: 25000 });
  await expect(idFingerprint, 'the created order (by id) renders on first load').toBeVisible({ timeout: 25000 });
  const statusBefore = await badge.getAttribute('data-status');
  expect(statusBefore, 'a real status is present before refresh').toMatch(/\w/);

  // The continuity test: a hard reload mid-journey must rehydrate THE SAME order — not any
  // order with a matching status string, and never the "Not Found" dead-end this seam guards.
  await page.reload();
  await expect(badge, 'order status survives a refresh (continuity)').toBeVisible({ timeout: 25000 });
  await expect(idFingerprint, 'the SAME order (by id) is re-fetched after refresh').toBeVisible({ timeout: 25000 });
  await expect(notFound, 'refresh must NOT dead-end into a Not Found state').toHaveCount(0);
  await expect(badge).toHaveAttribute('data-status', statusBefore!);
});
