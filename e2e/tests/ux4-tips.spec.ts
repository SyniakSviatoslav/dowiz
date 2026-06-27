import { test, expect } from '@playwright/test';

// UX-4 tips — display surfaces. Mocks the order/assignment so the tip renders on
// the client order breakdown and the courier delivery screen.
//
// NOTE on route order: Playwright matches routes LIFO (last-registered wins), so the
// per-test specific routes below — registered AFTER mockCommon — take priority over the
// `**/api/**` catch-all. The exact-value assertions are the guardrail that would go RED if
// the catch-all ever shadowed a specific endpoint (the tip would render empty / not at all).
const SLUG = 'test-roma';

// tipAmount is integer minor units; ALL formats verbatim (no /100) → "50000 ALL".
const TIP = 50000;
const TIP_TEXT = '50000 ALL';

async function mockCommon(page: any) {
  await page.route('**/public/**', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
  await page.route('**/api/**', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
  await page.route('**/public/locations/*/info', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
}

function orderBody(tipAmount: number) {
  return JSON.stringify({
    id: 'o1', status: 'IN_DELIVERY', total: 120000, tipAmount, createdAt: '2026-06-20T10:00:00Z', elapsedSeconds: 0, items: [], rating: null,
  });
}

function taskBody(tipAmount: number) {
  return JSON.stringify({
    id: 'o1', status: 'IN_DELIVERY', eta: '10 min', total: 120000, tipAmount, cashPayWith: null,
    restaurant: { name: 'Roma', address: 'Blloku', lat: 41.328, lng: 19.812 },
    customer: { address: 'Rruga e Elbasanit 12', phone: '+355691234567', lat: 41.337, lng: 19.825 },
  });
}

async function routeOrderStatus(page: any, status: number, tipAmount = TIP) {
  await page.route('**/customer/orders/*/status', (r: any) => r.fulfill({
    status, contentType: 'application/json', body: status === 200 ? orderBody(tipAmount) : '{}',
  }));
}

async function routeAssignment(page: any, status: number, tipAmount = TIP) {
  await page.route('**/api/courier/assignments/*', (r: any) => {
    if (/\/(picked-up|delivered|messages)/.test(r.request().url())) return r.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
    return r.fulfill({ status, contentType: 'application/json', body: status === 200 ? taskBody(tipAmount) : '{}' });
  });
}

test('client order breakdown shows the courier tip with its value', async ({ page }) => {
  await mockCommon(page);
  await routeOrderStatus(page, 200);
  await page.goto(`/s/${SLUG}/order/o1`);
  const tip = page.getByTestId('order-tip');
  await expect(tip).toBeVisible();
  // Render proof: the actual tip value, not just element presence (an always-empty row must fail).
  await expect(tip).toContainText(TIP_TEXT);
});

test('client breakdown hides the tip row when tip is 0', async ({ page }) => {
  await mockCommon(page);
  await routeOrderStatus(page, 200, 0);
  await page.goto(`/s/${SLUG}/order/o1`);
  // Positive control: the order rendered (so this is a real absence, not a blank/hung page).
  await expect(page.getByTestId('order-status-badge')).toBeVisible();
  await expect(page.getByTestId('order-tip')).not.toBeVisible();
});

// Error matrix — a non-200 order status must surface the unavailable/not-found state and
// NEVER fabricate a tip row. Statuses are exact by construction (we author the mock).
for (const status of [401, 403, 404, 500]) {
  test(`client breakdown shows no tip on ${status} order-status`, async ({ page }) => {
    await mockCommon(page);
    await routeOrderStatus(page, status);
    await page.goto(`/s/${SLUG}/order/o1`);
    // Positive control: the empty/error surface (not a blank page).
    await expect(page.getByTestId('order-back-to-menu')).toBeVisible();
    await expect(page.getByTestId('order-tip')).not.toBeVisible();
  });
}

test('courier delivery screen shows the tip to collect with its value', async ({ page }) => {
  await mockCommon(page);
  await routeAssignment(page, 200);
  await page.goto('/courier/delivery/o1?dev=true');
  const tip = page.getByTestId('task-tip');
  await expect(tip).toBeVisible();
  await expect(tip).toContainText(TIP_TEXT);
});

test('courier delivery screen hides the tip when tip is 0', async ({ page }) => {
  await mockCommon(page);
  await routeAssignment(page, 200, 0);
  await page.goto('/courier/delivery/o1?dev=true');
  // Positive control: the active task rendered (advance action present) yet no tip row.
  await expect(page.getByTestId('courier-advance-action')).toBeVisible();
  await expect(page.getByTestId('task-tip')).not.toBeVisible();
});

// Error matrix — a non-200 assignment fetch must not render a tip the courier could try to collect.
for (const status of [401, 403, 404, 500]) {
  test(`courier screen shows no tip on ${status} assignment`, async ({ page }) => {
    await mockCommon(page);
    await routeAssignment(page, status);
    await page.goto('/courier/delivery/o1?dev=true');
    await expect(page.getByTestId('task-tip')).not.toBeVisible();
  });
}
