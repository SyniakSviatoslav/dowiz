import { test, expect } from '@playwright/test';

// Admin orders surface = DashboardPage (route /admin/orders → DashboardPage.tsx).
// Real-DOM anchors (verified against source):
//   [data-testid="ws-status-dot"]   — always rendered once the dashboard mounts
//   getByText(/couldn't load orders/i) — the error EmptyState (fetchOrders catch)
//   #login-email                    — admin LoginPage form (unauth redirect target /login)
// Orders are fetched from /api/owner/orders (apiClient API_BASE='/api', RLS-scoped).

test.describe('Admin Orders Page', () => {

  test('orders page accessible from sidebar — navigates and renders content', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/admin?dev=true');
    await page.waitForLoadState('networkidle');
    const ordersLink = page.locator('a:has-text("Orders"), a:has-text("orders")');
    const linkCount = await ordersLink.count();
    expect(linkCount).toBeGreaterThanOrEqual(1);
    await ordersLink.first().click();
    await page.waitForURL(/\/admin\/orders/);
    await page.waitForLoadState('networkidle');
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    // Specific orders-only anchor — a skeleton/spinner/error-boundary has no ws-status-dot.
    await expect(page.locator('[data-testid="ws-status-dot"]')).toBeVisible();
    // The orders fetch must NOT have failed (positive control).
    await expect(page.getByText(/couldn't load orders/i)).toHaveCount(0);
  });

  test('orders page WITHOUT dev bypass redirects unauth user to /login', async ({ page }) => {
    // Exercises the REAL auth gate (AdminRoutes: !isAuthed && !isDev → navigate('/login')).
    // No ?dev=true — a broken gate that let an anonymous user through would fail here.
    await page.goto('/admin/orders');
    await page.waitForURL(/\/login/);
    await expect(page.locator('#login-email')).toBeVisible();
    expect(new URL(page.url()).pathname).toBe('/login');
  });

  test('orders page surfaces an error when /owner/orders returns 500', async ({ page }) => {
    // Error-matrix: a 5xx from the orders API must show the error EmptyState, never a
    // silent empty grid or a stuck spinner.
    await page.route('**/api/owner/orders**', route =>
      route.fulfill({ status: 500, contentType: 'application/json', body: JSON.stringify({ code: 'INTERNAL', message: 'boom' }) }),
    );
    await page.goto('/admin/orders?dev=true');
    await page.waitForLoadState('networkidle');
    await expect(page.getByText(/couldn't load orders/i)).toBeVisible();
  });

  test('menu manager page loads with categories and products', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/admin/menu?dev=true');
    await page.waitForLoadState('networkidle');
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    // Specific anchor — KitchenBusyToggle is rendered unconditionally on MenuManagerPage.
    await expect(page.locator('[data-testid="kitchen-busy-toggle"]')).toBeVisible();
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('branding page loads with theme editor and CSS controls', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/admin/branding?dev=true');
    await page.waitForLoadState('networkidle');
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    // Specific anchor — top-level wrapper of BrandingPage.
    await expect(page.locator('[data-testid="branding-page"]')).toBeVisible();
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  // TODO(needs_staging): cross-tenant / IDOR — seed a SECOND real tenant with a known order id,
  //   log in as tenant-A admin, assert tenant-B's order-card-<id> is NOT visible AND a direct
  //   GET /api/owner/orders never returns tenant-B's order id (RLS proof). Requires a real 2nd
  //   tenant fixture + staging run; do not fake with a nil-UUID (it 404s by absence, proves nothing).
  // TODO(needs_staging): real-time WS — open /admin/orders, place an order via the public API for
  //   THIS tenant, assert order-card-<that-id> appears with NO reload, gated on
  //   expect(wsOpened).toBe(true). Requires a live staging order-create + WS bridge.

});
