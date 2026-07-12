/**
 * OWNER critical-path visual snapshot spec.
 *
 * Breakpoints are PROJECTS (mobile-390 / tablet-768 / desktop-1280) — we do NOT loop
 * breakpoints here; we loop LOCALES inside each test. Every snapshot masks the dynamic
 * zone (RelativeTime, live map, animated counters) via MASK(page).
 *
 * Auth: loginAs(request,'owner',{locationSlug}) → applyAuth(page, token) boots the SPA
 * authenticated. State that can't be seeded deterministically (empty list, dead WS) is
 * forced with page.route mocks on the relevant /api/owner/* (and the WS upgrade).
 *
 * Routes (READ apps/web/src/routes/AdminRoutes.tsx):
 *   /admin/orders → DashboardPage (renders directly, no AdminHome redirect gamble)
 *   /admin        → AdminHome (settings→activation gate) then DashboardPage
 * Testids (READ DashboardPage.tsx + packages/ui/.../OrderCard.tsx + ConfirmDialog.tsx):
 *   ws-status-dot, owner-alert-status, owner-alert-enable, owner-new-order-banner,
 *   order-card-<id>, order-confirm, order-prepare, order-ready, order-assign,
 *   reject button (Button variant=outline, text common.reject) → useConfirm role="alertdialog".
 */
import { test, expect } from '@playwright/test';
import { loginAs, applyAuth, setLocale, MASK, settle, LOCALES } from './harness.js';
import { readFixtures } from './global-setup.js';

// A deterministic owner-orders payload covering the kanban statuses the owner path needs.
// Shape mirrors AdminOrder (id/shortId/status/total/customerName/createdAt/items/...).
const FIXED_TS = '2026-06-24T10:00:00.000Z';
function mockOrders(statuses: string[]) {
  return statuses.map((status, i) => ({
    id: `vis-order-${i}`,
    shortId: `A${100 + i}`,
    status,
    total: 1800 + i * 350,
    customerName: `Customer ${i + 1}`,
    deliveryAddress: 'Rruga e Durrësit 12, Tiranë',
    paymentMethod: 'cash',
    itemsSummary: '2x Sushi roll, 1x Miso',
    createdAt: FIXED_TS,
    items: [
      { name: 'Sushi roll', qty: 2, price: 600 },
      { name: 'Miso soup', qty: 1, price: 600 },
    ],
  }));
}

/** Route owner background fetches so a screen is deterministic regardless of seed/clock. */
async function stubOwnerBackground(page: import('@playwright/test').Page, opts: { orders?: unknown } = {}) {
  // Orders list — the screen's primary payload (override per-test via opts.orders).
  if (opts.orders !== undefined) {
    await page.route('**/api/owner/orders', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(opts.orders) }),
    );
  }
  // Quiet the readiness/notification/courier-map side fetches so they don't introduce
  // run-to-run jitter or network-idle stalls.
  await page.route('**/api/owner/settings', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ id: 'vis-loc', name: 'Dubin & Sushi', phone: '+355691234567', address: 'Durrës, AL' }) }),
  );
  await page.route('**/api/owner/menu/categories', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify([{ id: 'c1', name: 'Sushi' }]) }),
  );
  await page.route('**/api/owner/couriers', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify([]) }),
  );
  await page.route('**/api/owner/locations/*/notifications/status', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ telegramConnected: true }) }),
  );
  await page.route('**/api/owner/locations/*/couriers/live', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ couriers: [] }) }),
  );
}

async function bootOwner(page: import('@playwright/test').Page, request: import('@playwright/test').APIRequestContext) {
  const { token } = await loginAs(request, 'owner', { locationSlug: readFixtures().open.slug });
  await applyAuth(page, token);
}

// ── Dashboard: success ──────────────────────────────────────────────────────
for (const loc of LOCALES) {
  test(`dashboard-success-${loc}`, async ({ page, request }) => {
    await bootOwner(page, request);
    await setLocale(page, loc);
    await stubOwnerBackground(page, { orders: mockOrders(['PENDING', 'CONFIRMED', 'PREPARING', 'READY']) });
    await page.goto('/admin/orders');
    await settle(page);
    await expect(page).toHaveScreenshot(`dashboard-success-${loc}.png`, { mask: MASK(page), fullPage: true });
  });
}

// ── Dashboard: new-order (a PENDING order present + unacknowledged banner surface) ─
for (const loc of LOCALES) {
  test(`dashboard-new-order-${loc}`, async ({ page, request }) => {
    await bootOwner(page, request);
    await setLocale(page, loc);
    await stubOwnerBackground(page, { orders: mockOrders(['PENDING', 'PENDING']) });
    // The persistent new-order banner is WS-driven (DashboardPage useWebSocket → on
    // `order.created` → unackOrders → banner). Mock the /ws upgrade: ack `auth`, then push
    // a synthetic `order.created` frame so the banner actually renders. A bare PENDING-card
    // check never exercised this path → it could go green with the banner wiring broken.
    await page.routeWebSocket(/\/ws(\?|$)/, (ws) => {
      ws.onMessage((message) => {
        let parsed: { type?: string } | null = null;
        try { parsed = JSON.parse(String(message)) as { type?: string }; } catch { parsed = null; }
        if (parsed?.type === 'auth' || parsed?.type === 'subscribe') {
          ws.send(JSON.stringify({ type: 'auth_success' }));
          ws.send(JSON.stringify({ type: 'order.created', data: { id: 'vis-order-0', status: 'PENDING', shortId: 'A100' } }));
        }
      });
    });
    await page.goto('/admin/orders');
    await settle(page);
    // Proof the WS-driven banner rendered (not merely that PENDING cards exist).
    await expect(page.locator('[data-testid="owner-new-order-banner"]')).toBeVisible();
    await expect(page.locator('[data-testid^="order-card-"][data-status="PENDING"]').first()).toBeVisible();
    await expect(page).toHaveScreenshot(`dashboard-new-order-${loc}.png`, { mask: MASK(page), fullPage: true });
  });
}

// ── Dashboard: empty (no orders) ──────────────────────────────────────────────
for (const loc of LOCALES) {
  test(`dashboard-empty-${loc}`, async ({ page, request }) => {
    await bootOwner(page, request);
    await setLocale(page, loc);
    await stubOwnerBackground(page, { orders: [] });
    await page.goto('/admin/orders');
    await settle(page);
    await expect(page).toHaveScreenshot(`dashboard-empty-${loc}.png`, { mask: MASK(page), fullPage: true });
  });
}

// ── Dashboard: dead-channel (WS disconnected → ws-status-dot reflects it) ─────
for (const loc of LOCALES) {
  test(`dashboard-dead-channel-${loc}`, async ({ page, request }) => {
    await bootOwner(page, request);
    await setLocale(page, loc);
    await stubOwnerBackground(page, { orders: mockOrders(['PENDING', 'CONFIRMED']) });
<<<<<<< Updated upstream
    // Force the realtime socket to fail its upgrade so connectionStatus is not 'connected'.
    await page.route('**/socket.io/**', (route) => route.abort());
    await page.route('ws://**', (route) => route.abort()).catch(() => {});
    await page.route('wss://**', (route) => route.abort()).catch(() => {});
=======
    // Force the realtime socket dead: the app connects a raw WebSocket to `/ws` (NOT
    // socket.io). Mock the upgrade and immediately close it so connectionStatus settles
    // on 'disconnected' (close code 1000 → no reconnect, per useWebSocket.onclose).
    await page.routeWebSocket(/\/ws(\?|$)/, (ws) => ws.close());
>>>>>>> Stashed changes
    await page.goto('/admin/orders');
    await settle(page);
    // Assert the dot actually reflects the dead channel — a bare toBeVisible() passed even
    // when connected, so a broken status→data-connected mapping would not have failed here.
    await expect(page.locator('[data-testid="ws-status-dot"][data-connected="false"]')).toBeVisible();
    await expect(page).toHaveScreenshot(`dashboard-dead-channel-${loc}.png`, { mask: MASK(page), fullPage: true });
  });
}

// ── Dashboard: orders fetch error (500 → honest error UI, not a silent empty board) ─
// Every status branch was previously mocked 200; a 500/spinner/redirect would have slipped
// through. Here the orders endpoint 500s → DashboardPage.fetchOrders catch → error EmptyState.
for (const loc of LOCALES) {
  test(`dashboard-orders-error-${loc}`, async ({ page, request }) => {
    await bootOwner(page, request);
    await setLocale(page, loc);
    await stubOwnerBackground(page); // side fetches only; orders handled below
    await page.route('**/api/owner/orders', (route) =>
      route.fulfill({ status: 500, contentType: 'application/json', body: JSON.stringify({ error: 'internal' }) }),
    );
    await page.goto('/admin/orders');
    await settle(page);
    // The error surface must render AND no order cards must be present (a 500 cannot look
    // like a normal/empty board). A bare list check would never have caught a swallowed 500.
    await expect(page.locator('[data-testid="orders-load-error"]')).toBeVisible();
    await expect(page.locator('[data-testid^="order-card-"]')).toHaveCount(0);
    await expect(page).toHaveScreenshot(`dashboard-orders-error-${loc}.png`, { mask: MASK(page), fullPage: true });
  });
}

// TODO(needs-staging): two blind-spots can't be honestly covered with route-mocks here —
//   (1) 401/403 on /api/owner/orders → the real auth interceptor's logout/redirect behaviour
//       (mocking it would only re-assert the local catch branch, proving nothing about auth).
//   (2) cross-tenant isolation: a token bound to location A must NOT receive location B's
//       orders. This needs the REAL /api/owner/orders + a REAL second tenant's seeded id
//       (a nil/zero id 404s by absence and proves nothing — see AGENTS.md Test Integrity #5).
//   Both require a live staging run against a 2nd seeded tenant; do NOT fake them here.

// ── Dashboard: alert/sound-enable affordance (the dashboard's "toggle on" surface) ─
for (const loc of LOCALES) {
  test(`dashboard-alert-enable-${loc}`, async ({ page, request }) => {
    await bootOwner(page, request);
    await setLocale(page, loc);
    await stubOwnerBackground(page, { orders: mockOrders(['PENDING']) });
    await page.goto('/admin/orders');
    await settle(page);
    // Audio is blocked under headless → the honest "Enable sound" affordance is shown.
    await expect(page.locator('[data-testid="owner-alert-enable"]')).toBeVisible();
    await expect(page).toHaveScreenshot(`dashboard-alert-enable-${loc}.png`, { mask: MASK(page), fullPage: true });
  });
}

// ── Kanban (orders in various states) ─────────────────────────────────────────
for (const loc of LOCALES) {
  test(`kanban-mixed-${loc}`, async ({ page, request }) => {
    await bootOwner(page, request);
    await setLocale(page, loc);
    await stubOwnerBackground(page, { orders: mockOrders(['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY']) });
    await page.goto('/admin/orders');
    await settle(page);
    await expect(page.locator('[data-testid="order-confirm"]').first()).toBeVisible();
    await expect(page.locator('[data-testid="order-ready"]').first()).toBeVisible();
    await expect(page).toHaveScreenshot(`kanban-mixed-${loc}.png`, { mask: MASK(page), fullPage: true });
  });
}

// ── Order detail drawer (click a card → ResponsiveDialog) ─────────────────────
for (const loc of LOCALES) {
  test(`kanban-order-detail-${loc}`, async ({ page, request }) => {
    await bootOwner(page, request);
    await setLocale(page, loc);
    await stubOwnerBackground(page, { orders: mockOrders(['CONFIRMED', 'PREPARING']) });
    await page.goto('/admin/orders');
    await settle(page);
    // Click the card body (OrderCard root onClick → onViewDetail opens the detail dialog).
    await page.locator('[data-testid="order-card-vis-order-0"]').click();
    const detailDialog = page.getByRole('dialog');
    await expect(detailDialog).toBeVisible();
    // Assert the dialog rendered the order's own content (customer + a line item), not just
    // an empty shell — a visible-dialog-only check passed even if the body failed to bind.
    await expect(detailDialog.getByText('Customer 1')).toBeVisible();
    await expect(detailDialog.getByText(/Sushi roll/)).toBeVisible();
    await settle(page);
    await expect(page).toHaveScreenshot(`kanban-order-detail-${loc}.png`, { mask: MASK(page), fullPage: true });
  });
}

// ── Reject modal (PENDING order → Reject → useConfirm alertdialog) ────────────
for (const loc of LOCALES) {
  test(`kanban-reject-modal-${loc}`, async ({ page, request }) => {
    await bootOwner(page, request);
    await setLocale(page, loc);
    await stubOwnerBackground(page, { orders: mockOrders(['PENDING']) });
    await page.goto('/admin/orders');
    await settle(page);
    // PENDING card exposes Accept (order-confirm) + Reject (outline button). Reject →
    // handleUpdateStatus('CANCELLED') → confirm() opens the danger alertdialog.
    await page.locator('[data-testid="order-card-vis-order-0"] button').filter({ hasText: /reject|refuzo/i }).first().click();
    await expect(page.getByRole('alertdialog')).toBeVisible();
    await settle(page);
    await expect(page).toHaveScreenshot(`kanban-reject-modal-${loc}.png`, { mask: MASK(page), fullPage: true });
  });
}
