import { test, expect, type APIRequestContext, type Page } from '@playwright/test';
import crypto from 'node:crypto';
import { expectUuid } from '../../helpers/assert-shape';

// Courier offer timer + accept/decline proof (testplan §6) against deployed staging.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev DEV_AUTH_SECRET=stg-e2e-secret \
//        pnpm exec playwright test offer-timer --project=desktop --reporter=list
//
// HARNESS (no product code — reuses the flag-gated /api/dev test helpers, ADR-0003):
//   1. POST /api/dev/mock-auth {role:'courier'}  → a courier JWT for the demo location.
//   2. POST /api/orders (one real delivery order on the demo storefront).
//   3. POST /api/dev/create-assignment {orderId,courierId,locationId} → inserts a
//      courier_assignments row in 'assigned' status for that courier. The dispatcher has
//      no 'offered' state — an offer IS a row with status 'assigned' (the timer/decline
//      window is a CLIENT-SIDE countdown in TaskCard, packages/ui/.../TaskCard.tsx, shown
//      for any rendered card because TasksPage always supplies onReject + offerSeconds=60).
//   4. The SPA TasksPage (apps/web/src/pages/courier/TasksPage.tsx) reads the JWT from
//      localStorage('dos_access_token') and GETs /api/courier/me/assignments. No route
//      guard — injecting the token via addInitScript before navigation is sufficient.
//
// The /api/dev/* helpers 404 unless ALLOW_DEV_LOGIN=true AND the x-dev-auth-secret header
// matches DEV_AUTH_SECRET (sent automatically by playwright.config.ts when DEV_AUTH_SECRET
// is set in the env). On a prod-shaped deploy these tests will skip-fail at setup with a
// clear message rather than fake green.

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

// /api/orders is rate-limited ~5/min per IP, so we place exactly ONE real order for the
// whole file and reset its single assignment between tests (create-assignment is idempotent
// per order_id: ON CONFLICT DO UPDATE ... status='assigned'). Login/mock-auth are cheap.
let cachedOrderId: string | null = null;

// Staging occasionally returns a transient 5xx on the public read endpoints (cold pool).
// Retry a GET a few times so a blip doesn't fail the proof; a persistent failure still throws.
async function getJsonWithRetry(request: APIRequestContext, path: string, label: string) {
  let last = '';
  for (let i = 0; i < 8; i++) {
    const res = await request.get(path, { headers: { accept: 'application/json' } });
    if (res.ok()) return res.json();
    last = `${res.status()} ${await res.text()}`;
    await new Promise((r) => setTimeout(r, 2000));
  }
  throw new Error(`${label} did not load after retries: ${last}`);
}

async function demoTarget(request: APIRequestContext) {
  const loc = await getJsonWithRetry(request, '/public/locations/demo/info', 'demo info');
  const m = await getJsonWithRetry(request, '/public/locations/demo/menu', 'demo menu');
  const products: any[] = (m.categories ?? []).flatMap((c: any) => c.products ?? []);
  expect(products.length, 'demo has products').toBeGreaterThan(0);
  // Pin the delivery at the venue's own coordinates so it is always within range.
  return { locationId: m.locationId ?? m.location_id, lat: loc.lat, lng: loc.lng, productId: products[0].id };
}

let phoneSeq = 0;
function uniquePhone() {
  phoneSeq += 1;
  return `+35567${String(Date.now()).slice(-4)}${String(phoneSeq).padStart(2, '0')}`;
}

// POST that retries through transient 5xx (staging DB blips) but surfaces a real non-5xx
// failure (e.g. 404 when the dev flag/secret is off) immediately — never fakes green.
async function postWithRetry(request: APIRequestContext, path: string, data: unknown, label: string) {
  let last = '';
  for (let i = 0; i < 8; i++) {
    const res = await request.post(path, { data, timeout: 30000 });
    if (res.ok()) return res.json();
    last = `${res.status()} ${await res.text()}`;
    if (res.status() < 500) break; // a 4xx is a real config/contract error — stop, report it
    await new Promise((r) => setTimeout(r, 2000));
  }
  throw new Error(`${label} failed: ${last}`);
}

// Mint a fresh courier (random courierId + a JWT bound to the demo location).
async function mockCourier(request: APIRequestContext) {
  const j = await postWithRetry(
    request,
    '/api/dev/mock-auth',
    { role: 'courier' },
    'mock-auth courier (needs ALLOW_DEV_LOGIN=true + DEV_AUTH_SECRET env / x-dev-auth-secret header)',
  );
  return { token: j.access_token as string, courierId: j.userId as string, locationId: j.activeLocationId as string };
}

// One real order for the whole file (rate-limit budget). The order POST is the heaviest,
// flakiest call on staging — it can be slow (transient 5xx / latency) or 429 (rate-limited
// ~5/min per IP). Retry with a fresh phone each attempt; on 429 wait out the window (~75s).
// A genuine, persistent failure still throws with the real status — never faked green.
async function ensureOrder(request: APIRequestContext): Promise<string> {
  if (cachedOrderId) return cachedOrderId;
  const t = await demoTarget(request);
  let last = '';
  for (let attempt = 0; attempt < 3; attempt++) {
    const created = await request.post('/api/orders', {
      timeout: 30000,
      data: {
        locationId: t.locationId,
        type: 'delivery',
        items: [{ product_id: t.productId, quantity: 1 }],
        customer: { phone: uniquePhone(), name: 'E2E Offer Timer' },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
        acknowledged_codes: ['velocity'],
        delivery: { pin: { lat: t.lat, lng: t.lng }, address_text: 'Demo HQ' },
      },
    });
    if (created.status() === 201) {
      cachedOrderId = (await created.json()).id as string;
      return cachedOrderId;
    }
    last = `${created.status()} ${await created.text()}`;
    // 429 → rate-limited; wait out the per-IP window. 5xx → transient; short backoff.
    await new Promise((r) => setTimeout(r, created.status() === 429 ? 75000 : 3000));
  }
  throw new Error(`create order failed after retries: ${last}`);
}

// Reset the order's single assignment to 'assigned' for a (fresh) courier, then return the
// assignment id. Idempotent — re-points the same order_id row at this courier.
async function offerTask(request: APIRequestContext, orderId: string, courierId: string, locationId: string) {
  const j = await postWithRetry(request, '/api/dev/create-assignment', { orderId, courierId, locationId }, 'create-assignment');
  return j.assignmentId as string;
}

// Seed the courier JWT into localStorage before any app code runs, then open /courier.
async function openTasksAs(page: Page, token: string) {
  await page.addInitScript((tok) => {
    try { window.localStorage.setItem('dos_access_token', tok); } catch { /* private mode */ }
  }, token);
  await page.goto('/courier');
}

// §6a — the offer countdown bar is visible and its data-remaining integer ticks DOWN.
test('6a: courier-offer-timer renders and ticks down', async ({ page, request }) => {
  const orderId = await ensureOrder(request);
  const courier = await mockCourier(request);
  const assignmentId = await offerTask(request, orderId, courier.courierId, courier.locationId);
  expectUuid(assignmentId, 'assignment id');

  await openTasksAs(page, courier.token);

  const timer = page.locator('[data-testid="courier-offer-timer"]');
  await expect(timer, 'offer timer is visible').toBeVisible({ timeout: 25000 });

  // data-remaining is an integer in (0, 60].
  const first = Number(await timer.getAttribute('data-remaining'));
  expect(Number.isInteger(first), `data-remaining is an integer (got "${first}")`).toBe(true);
  expect(first).toBeGreaterThan(0);
  expect(first).toBeLessThanOrEqual(60);

  // It counts DOWN: wait for the attribute to drop below the first reading.
  await expect
    .poll(async () => Number(await timer.getAttribute('data-remaining')), {
      timeout: 8000,
      message: 'data-remaining should decrease over time',
    })
    .toBeLessThan(first);
});

// §6b — accept is visible; clicking it navigates to /courier/delivery/:id.
test('6b: task-accept navigates to the delivery view', async ({ page, request }) => {
  const orderId = await ensureOrder(request);
  const courier = await mockCourier(request);
  const assignmentId = await offerTask(request, orderId, courier.courierId, courier.locationId);

  await openTasksAs(page, courier.token);

  const accept = page.locator('[data-testid="task-accept"]');
  await expect(accept, 'accept button is visible').toBeVisible({ timeout: 25000 });
  await accept.click();

  // TasksPage.handleAccept POSTs /courier/assignments/:id/accept then navigate(`/courier/delivery/${id}`).
  await expect(page).toHaveURL(new RegExp(`/courier/delivery/${assignmentId}`), { timeout: 20000 });
});

// §6c — decline removes the task card from the list.
test('6c: courier-offer-decline removes the task card', async ({ page, request }) => {
  const orderId = await ensureOrder(request);
  const courier = await mockCourier(request);
  await offerTask(request, orderId, courier.courierId, courier.locationId);

  await openTasksAs(page, courier.token);

  const card = page.locator(`[data-testid^="task-card-"]`);
  await expect(card, 'a task card is shown').toBeVisible({ timeout: 25000 });

  const decline = page.locator('[data-testid="courier-offer-decline"]');
  await expect(decline, 'decline button is visible').toBeVisible();
  await decline.click();

  // handleReject optimistically removes the card, then releases the assignment server-side.
  await expect(card, 'card is removed after decline').toHaveCount(0, { timeout: 20000 });
});

// §6e — RE-VERIFY: the courier-advance-action testid. The §6 note said it was previously
// found NOT to exist. It DOES exist (apps/web/src/pages/courier/DeliveryPage.tsx:407), but
// is rendered ONLY after the courier marks the order picked up in-page (gated by the local
// `pickedUp` state, set in handlePickup → setPickedUp(true) at line 168; it does NOT derive
// from task.status). So: accept (API) → open delivery view (status 'accepted' → "Mark as
// Picked Up" button) → click it → the advance action (SwipeToComplete) appears. We force
// locale=en so the pickup button label is stable.
test('6e: courier-advance-action exists on the delivery view (after pickup)', async ({ page, request }) => {
  const orderId = await ensureOrder(request);
  const courier = await mockCourier(request);
  const assignmentId = await offerTask(request, orderId, courier.courierId, courier.locationId);

  // Accept so the assignment is 'accepted' (precondition for /picked-up to succeed).
  const acc = await request.post(`/api/courier/assignments/${assignmentId}/accept`, {
    headers: { Authorization: `Bearer ${courier.token}` },
  });
  expect(acc.status(), `accept: ${await acc.text()}`).toBe(200);

  await page.addInitScript((tok) => {
    try {
      window.localStorage.setItem('dos_access_token', tok);
      window.localStorage.setItem('dos_locale', 'en');
    } catch { /* private mode */ }
  }, courier.token);
  await page.goto(`/courier/delivery/${assignmentId}`);

  // Before pickup the advance action is not shown; the pickup button is.
  const pickup = page.getByRole('button', { name: /picked up/i });
  await expect(pickup, '"Mark as Picked Up" button is shown for an accepted task').toBeVisible({ timeout: 25000 });
  await expect(page.locator('[data-testid="courier-advance-action"]')).toHaveCount(0);

  await pickup.click();

  await expect(
    page.locator('[data-testid="courier-advance-action"]'),
    'advance action appears after marking picked up',
  ).toBeVisible({ timeout: 20000 });
});
