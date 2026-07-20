import { test, expect } from '@playwright/test';

// UX-4 tips — display surfaces. Mocks the order/assignment so the tip renders on
// the client order breakdown and the courier delivery screen.
const SLUG = 'test-roma';

async function mockCommon(page: any) {
  await page.route('**/public/**', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
  await page.route('**/api/**', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
  await page.route('**/public/locations/*/info', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
}

test('client order breakdown shows the courier tip', async ({ page }) => {
  await mockCommon(page);
  await page.route('**/customer/orders/*/status', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({
    id: 'o1', status: 'IN_DELIVERY', total: 120000, tipAmount: 50000, createdAt: '2026-06-20T10:00:00Z', elapsedSeconds: 0, items: [], rating: null,
  }) }));
  await page.goto(`/s/${SLUG}/order/o1`);
  await expect(page.getByTestId('order-tip')).toBeVisible();
});

test('courier delivery screen shows the tip to collect', async ({ page }) => {
  await mockCommon(page);
  await page.route('**/api/courier/assignments/*', (r: any) => {
    if (/\/(picked-up|delivered|messages)/.test(r.request().url())) return r.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
    return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({
      id: 'o1', status: 'IN_DELIVERY', eta: '10 min', total: 120000, tipAmount: 50000, cashPayWith: null,
      restaurant: { name: 'Roma', address: 'Blloku', lat: 41.328, lng: 19.812 },
      customer: { address: 'Rruga e Elbasanit 12', phone: '+355691234567', lat: 41.337, lng: 19.825 },
    }) });
  });
  await page.goto('/courier/delivery/o1?dev=true');
  await expect(page.getByTestId('task-tip')).toBeVisible();
});
