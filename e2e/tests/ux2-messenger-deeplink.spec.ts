import { test, expect } from '@playwright/test';

// UX-2 messenger deep-link — client "Message courier" button. Mocks the order
// status so the courier's messenger surfaces; proves the button renders with the
// correct deep link and hides when no messenger is set. (Deep-link formatting is
// unit-tested separately; courier-side button mirrors this wiring.)
const SLUG = 'test-roma';

async function mockCommon(page: any) {
  await page.route('**/api/**', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
  await page.route('**/public/**', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
  await page.route('**/public/theme/**', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ primaryColor: '#ea4f16', bgColor: '#121212', textColor: '#fff' }) }));
  await page.route('**/public/locations/*/info', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
}

function orderStatus(extra: any) {
  return { id: 'o1', status: 'IN_DELIVERY', total: 1000, createdAt: '2026-06-20T10:00:00Z', elapsedSeconds: 0, items: [], rating: null, courier_id: 'cid', ...extra };
}

test('Message-courier button appears with a Telegram deep link', async ({ page }) => {
  await mockCommon(page);
  await page.route('**/customer/orders/*/status', (r: any) => r.fulfill({ status: 200, contentType: 'application/json',
    body: JSON.stringify(orderStatus({ courierMessenger: { kind: 'telegram', handle: '@courier_bob' } })) }));

  await page.goto(`/s/${SLUG}/order/o1`);
  // Positive render control: the order context actually loaded (not a broken/skeleton
  // shell from the blanket {} mocks) — the status badge only renders with a real order.
  await expect(page.getByTestId('order-status-badge')).toBeVisible();
  const btn = page.getByTestId('message-courier-btn');
  await expect(btn).toBeVisible();
  await expect(btn).toHaveAttribute('href', 'https://t.me/courier_bob');
  // The leading '@' MUST be stripped (raw or url-encoded) — guards the implicit strip path.
  await expect(btn).not.toHaveAttribute('href', /@|%40/);
});

test('Message-courier button is absent when courier has no messenger', async ({ page }) => {
  await mockCommon(page);
  await page.route('**/customer/orders/*/status', (r: any) => r.fulfill({ status: 200, contentType: 'application/json',
    body: JSON.stringify(orderStatus({ courierMessenger: null })) }));

  await page.goto(`/s/${SLUG}/order/o1`);
  // Wait on a positive render anchor (status badge = order loaded), not a blind sleep —
  // so the absence assertion can't pass merely because the component hasn't mounted yet.
  await expect(page.getByTestId('order-status-badge')).toBeVisible();
  await expect(page.getByTestId('message-courier-btn')).toHaveCount(0);
});
