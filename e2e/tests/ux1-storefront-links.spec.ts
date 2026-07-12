import { test, expect } from '@playwright/test';

// UX-1 storefront links — client surfaces. Mocks the public endpoints so the
// footer + Google review link render deterministically (no backend needed).
const SLUG = 'test-roma';

// Endpoints that per-test specific mocks own. The generic catch-alls below DEFER
// (route.fallback) for these so the test is robust to route-registration order —
// Playwright runs handlers LIFO, so a future reorder must not let `{}` shadow them.
const SPECIFIC = /\/(menu\b|info\b|status\b|theme\/)/;

async function mockCommon(page: any) {
  await page.route('**/api/**', (r: any) => (SPECIFIC.test(r.request().url()) ? r.fallback() : r.fulfill({ status: 200, contentType: 'application/json', body: '{}' })));
  await page.route('**/public/**', (r: any) => (SPECIFIC.test(r.request().url()) ? r.fallback() : r.fulfill({ status: 200, contentType: 'application/json', body: '{}' })));
  await page.route('**/public/theme/**', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ primaryColor: '#ea4f16', bgColor: '#121212', textColor: '#ffffff' }) }));
}

test('storefront footer renders Maps + social links, hidden in embed', async ({ page }) => {
  await mockCommon(page);
  await page.route('**/public/locations/*/menu*', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({
    categories: [{ id: 'c1', name: 'Pizza', products: [{ id: 'p1', name: 'Margherita', price: 500, available: true, attributes: {} }] }],
  }) }));
  await page.route('**/public/locations/*/info', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({
    lat: 41.3, lng: 19.8, isOpen: true,
    googleMapsUrl: 'https://maps.app.goo.gl/abc', socialInstagram: 'https://instagram.com/roma', socialFacebook: 'https://facebook.com/roma',
  }) }));

  await page.goto(`/s/${SLUG}`);
  await expect(page.locator('footer a[href="https://maps.app.goo.gl/abc"]')).toBeVisible();
  await expect(page.locator('footer a[href="https://instagram.com/roma"]')).toBeVisible();
  await expect(page.locator('footer a[href="https://facebook.com/roma"]')).toBeVisible();

  // Embed/activation-preview context hides the footer
  await page.goto(`/s/${SLUG}?embed=true`);
  await expect(page.getByText('Margherita').first()).toBeVisible(); // menu loaded in embed too
  await expect(page.locator('footer a[href="https://instagram.com/roma"]')).toHaveCount(0);
});

test('post-delivery Google review link shows to all (anti-gating) with correct place id', async ({ page }) => {
  await mockCommon(page);
  await page.route('**/customer/orders/*/status', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({
    id: 'o1', status: 'DELIVERED', total: 1000, createdAt: '2026-06-20T10:00:00Z', elapsedSeconds: 0, items: [], rating: null, canRate: true,
  }) }));
  await page.route('**/public/locations/*/info', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ googlePlaceId: 'ChIJtest123' }) }));

  await page.goto(`/s/${SLUG}/order/o1`);
  const review = page.getByTestId('google-review-link');
  await expect(review).toBeVisible(); // visible with rating===null → not gated on a positive rating
  // Exact href (base domain + path), not a partial regex — an evil.com?placeid=… must NOT pass.
  await expect(review).toHaveAttribute('href', 'https://search.google.com/local/writereview?placeid=ChIJtest123');
});

test('Google review link still shows after a rating exists (anti-gating, not a thank-you upsell)', async ({ page }) => {
  await mockCommon(page);
  await page.route('**/customer/orders/*/status', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({
    id: 'o2', status: 'DELIVERED', total: 1000, createdAt: '2026-06-20T10:00:00Z', elapsedSeconds: 0, items: [], rating: 4, feedback: 'great', canRate: false,
  }) }));
  await page.route('**/public/locations/*/info', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ googlePlaceId: 'ChIJtest123' }) }));

  await page.goto(`/s/${SLUG}/order/o2`);
  await expect(page.getByTestId('rating-submitted')).toBeVisible(); // already rated → submitted state, not the star picker
  const review = page.getByTestId('google-review-link');
  await expect(review).toBeVisible(); // link is NOT gated on rating===null
  await expect(review).toHaveAttribute('href', 'https://search.google.com/local/writereview?placeid=ChIJtest123');
});
