import { test, expect, type Page } from '@playwright/test';

const API_GLOB = '**/api/customer/orders/*/status';
const STATUS_RE = /\/api\/customer\/orders\/[^/]+\/status/;

// IN_DELIVERY delivery order with a courier + an HONEST eta RANGE (the page renders
// `etaRange`, never the raw `etaMinutes`). type is omitted → delivery branch (6 steps).
const STATUS_MOCK = {
  id: 'test-order-id',
  status: 'IN_DELIVERY',
  total: 650,
  items: [{ id: 'i1', productId: 'p1', name: 'Burger', price: 650, quantity: 1 }],
  createdAt: new Date().toISOString(),
  etaRange: { lowMin: 8, highMin: 12, phase: 'assigned', overdue: false },
  courierName: 'A***',
  courierPhoneMasked: '+*** *** 1234',
  courierPosition: { lat: 41.33, lng: 19.82 },
  deliveryLat: 41.335,
  deliveryLng: 19.825,
};

// Deterministic order data — never depend on what a live tenant happens to hold for
// this id. (The glob matches any id; we always drive the page to /test-order-id.)
function mockStatus(page: Page, body: Record<string, unknown>, status = 200): Promise<void> {
  return page.route(API_GLOB, route =>
    route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) }),
  );
}

// Wait for the REAL status fetch to land instead of a fixed sleep — a slow/never response
// then fails the test loudly instead of asserting against a half-rendered page.
const waitStatus = (page: Page) =>
  page.waitForResponse(r => STATUS_RE.test(r.url()), { timeout: 15000 });

test.describe('Client Order Status', () => {

  test('order status page loads with order content', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await mockStatus(page, STATUS_MOCK);
    const status = waitStatus(page);
    await page.goto('/s/test-slug/order/test-order-id?dev=true');
    await status;
    // Real render proof: the status badge for the mocked order renders (not a 500/spinner).
    await expect(page.locator('[data-testid=order-status-badge]')).toBeVisible({ timeout: 8000 });
    await expect(page.locator('[data-testid=order-eta-headline]')).toBeVisible();
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('status timeline shows progression steps', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await mockStatus(page, STATUS_MOCK);
    const status = waitStatus(page);
    await page.goto('/s/test-slug/order/test-order-id?dev=true');
    await status;
    await expect(page.locator('[data-testid=order-status-badge]')).toBeVisible({ timeout: 8000 });
    // A delivery order has EXACTLY 6 lifecycle steps (PENDING→DELIVERED). The `-time`
    // sub-nodes share the prefix, so exclude them.
    const steps = page.locator('[data-testid^="order-step-"]:not([data-testid$="-time"])');
    await expect(steps).toHaveCount(6);
    await expect(page.locator('[data-testid=order-step-in_delivery]')).toBeVisible();
    await expect(page.locator('[data-testid=order-step-delivered]')).toBeVisible();
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('order error shows the humane way-back state', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    // A 404 is intentionally soft-handled (fabricated PENDING order, never a dead-end);
    // a 500 is the genuine error path → the EmptyState with a route back to the menu.
    await mockStatus(page, { error: 'INTERNAL' }, 500);
    const status = waitStatus(page);
    await page.goto('/s/test-slug/order/nonexistent?dev=true');
    await status;
    await expect(page.locator('[data-testid=order-back-to-menu]')).toBeVisible({ timeout: 8000 });
    // and it is the error state, not a half-rendered order.
    await expect(page.locator('[data-testid=order-status-badge]')).toHaveCount(0);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('no cookies are set on status page', async ({ page }) => {
    await mockStatus(page, STATUS_MOCK);
    const status = waitStatus(page);
    await page.goto('/s/test-slug/order/test-order-id?dev=true');
    await status;
    // Assert AFTER the fetch path ran — that is when a cookie would have been set.
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  // CR-5: ETA display appears
  test('order status shows estimated arrival time', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await mockStatus(page, STATUS_MOCK);
    const status = waitStatus(page);
    await page.goto('/s/test-slug/order/test-order-id?dev=true');
    await status;
    await expect(page.locator('text=Estimated arrival')).toBeVisible({ timeout: 8000 });
    // Scope the ETA assertion to the headline locator — not the whole body.
    const eta = page.locator('[data-testid=order-eta-headline]');
    await expect(eta).toBeVisible();
    await expect(eta).toContainText('min');
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  // CR-5: Server-computed ETA range renders in the headline
  test('server eta range displays when available', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await mockStatus(page, STATUS_MOCK);
    const status = waitStatus(page);
    await page.goto('/s/test-slug/order/test-order-id?dev=true');
    await status;
    // Scoped to the visible headline — the mocked range is 8–12 min.
    const eta = page.locator('[data-testid=order-eta-headline]');
    await expect(eta).toBeVisible({ timeout: 8000 });
    await expect(eta).toContainText('8');
    await expect(eta).toContainText('12');
    await expect(eta).toContainText('min');
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  // CR-5: Courier info renders when a courier is assigned
  test('courier info shows on status page with courier assigned', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await mockStatus(page, STATUS_MOCK);
    const status = waitStatus(page);
    await page.goto('/s/test-slug/order/test-order-id?dev=true');
    await status;
    await expect(page.locator('[data-testid=order-status-badge]')).toBeVisible({ timeout: 8000 });
    // Scope the masked-courier-name assertion to the courier status element, not the
    // whole body (a debug overlay / tooltip must not satisfy it).
    const courier = page.getByText(/is delivering your order/);
    await expect(courier).toContainText('A***');
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  // CR-6: Share location button NOT visible before IN_DELIVERY
  test('share location button hidden before in_delivery', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    // PREPARING → the share-location affordance must not appear yet.
    await mockStatus(page, { ...STATUS_MOCK, status: 'PREPARING' });
    const status = waitStatus(page);
    await page.goto('/s/test-slug/order/test-order-id?dev=true');
    await status;
    // Positive control: the page actually rendered the (non-IN_DELIVERY) order.
    await expect(page.locator('[data-testid=order-status-badge]')).toBeVisible({ timeout: 8000 });
    const shareBtn = page.locator('text=Share my location with courier');
    await expect(shareBtn).toHaveCount(0);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  // CR-6: Share location button visible during IN_DELIVERY
  test('share location button visible during in_delivery', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await mockStatus(page, STATUS_MOCK);
    const status = waitStatus(page);
    await page.goto('/s/test-slug/order/test-order-id?dev=true');
    await status;
    const shareBtn = page.locator('text=Share my location with courier');
    await expect(shareBtn).toBeVisible({ timeout: 8000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  // CR-6: Share location flow — click share shows banner, stop hides it
  test('share location toggle shows and hides sharing banner', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await mockStatus(page, STATUS_MOCK);
    const status = waitStatus(page);
    await page.goto('/s/test-slug/order/test-order-id?dev=true');
    await status;

    const ctx = page.context();
    await ctx.grantPermissions(['geolocation']);
    await ctx.setGeolocation({ latitude: 41.33, longitude: 19.82 });

    // Click share → sharing banner appears (auto-retried, no fixed sleep).
    await page.locator('text=Share my location with courier').click();
    await expect(page.locator('text=Sharing your location')).toBeVisible({ timeout: 8000 });

    // Click stop → share button is back.
    await page.locator('text=Stop').click();
    await expect(page.locator('text=Share my location with courier')).toBeVisible({ timeout: 8000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  // API degradation after the page rendered → humane error state, never a dead-end.
  test('api 5xx renders the way-back error state instead of a stuck page', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await mockStatus(page, STATUS_MOCK);
    let status = waitStatus(page);
    await page.goto('/s/test-slug/order/test-order-id?dev=true');
    await status;
    await expect(page.locator('[data-testid=order-status-badge]')).toBeVisible({ timeout: 8000 });

    // Server degrades — the re-registered route wins (Playwright runs handlers LIFO).
    await mockStatus(page, { error: 'unavailable' }, 503);
    status = waitStatus(page);
    await page.reload();
    await status;
    await expect(page.locator('[data-testid=order-back-to-menu]')).toBeVisible({ timeout: 8000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    // TODO(staging): true MID-SESSION degradation — a 5xx on a WS-triggered refetch of an
    // OPEN page (no reload) — needs a live order + WS stream; see needs_staging.
  });

});
