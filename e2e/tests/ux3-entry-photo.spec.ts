import { test, expect } from '@playwright/test';

// UX-3 entry-anchor photo — courier side. Mocks the assignment so the entrance
// photo surfaces on the delivery screen; proves the thumbnail renders and taps
// open a fullscreen view. (Upload + R2 storage mirror the proven product-image
// path; R2 isn't available in-sandbox so the upload itself isn't E2E here.)
const PNG = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+M8AAAMBAQDJ/pLvAAAAAElFTkSuQmCC';

test('courier sees the entry-anchor photo and can open it fullscreen', async ({ page }) => {
  await page.route('**/public/**', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
  await page.route('**/api/**', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
  // Registered AFTER the catch-all on purpose: Playwright tries handlers last-added-first, so
  // this specific assignment route wins over the `**/api/**` stub. Keep this last.
  await page.route('**/api/courier/assignments/*', (r: any) => {
    // ignore sub-actions like /picked-up; only the bare GET returns the task
    if (/\/(picked-up|delivered|messages)/.test(r.request().url())) return r.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
    return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({
      id: 'o1', status: 'IN_DELIVERY', eta: '10 min', total: 1000, cashPayWith: null,
      restaurant: { name: 'Roma', address: 'Blloku', lat: 41.328, lng: 19.812 },
      customer: { address: 'Rruga e Elbasanit 12', phone: '+355691234567', instructions: 'Blue door', lat: 41.337, lng: 19.825, entryPhotoUrl: PNG },
    }) });
  });

  await page.goto('/courier/delivery/o1?dev=true');
  const thumb = page.getByTestId('entry-photo-thumb');
  await expect(thumb).toBeVisible();
  // not just "a button is visible" — the thumbnail must carry the real photo src,
  // so a broken/empty <img> can't pass.
  await expect(thumb.locator('img')).toHaveAttribute('src', PNG);
  await thumb.click();
  const modal = page.getByTestId('entry-photo-modal');
  await expect(modal).toBeVisible();
  // the fullscreen view must render the SAME photo, not an empty/wrong-src <img>.
  await expect(modal.locator('img')).toHaveAttribute('src', PNG);
});

test('courier with no entry photo sees no thumbnail (field absent)', async ({ page }) => {
  await page.route('**/public/**', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
  await page.route('**/api/**', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
  await page.route('**/api/courier/assignments/*', (r: any) => {
    if (/\/(picked-up|delivered|messages)/.test(r.request().url())) return r.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
    // identical task, but the customer never shared an entrance photo
    return r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({
      id: 'o1', status: 'IN_DELIVERY', eta: '10 min', total: 1000, cashPayWith: null,
      restaurant: { name: 'Roma', address: 'Blloku', lat: 41.328, lng: 19.812 },
      // messengerKind/Handle make the message button render — a stable, always-present
      // [data-testid] that proves the page actually rendered (not a 500/spinner).
      customer: { address: 'Rruga e Elbasanit 12', phone: '+355691234567', instructions: 'Blue door', lat: 41.337, lng: 19.825, messengerKind: 'telegram', messengerHandle: 'cust' },
    }) });
  });

  await page.goto('/courier/delivery/o1?dev=true');
  // positive render proof: the screen loaded with task content
  await expect(page.getByTestId('message-customer-btn')).toBeVisible();
  // negative: no photo field → the thumbnail must NOT exist
  await expect(page.getByTestId('entry-photo-thumb')).toHaveCount(0);
  await expect(page.getByTestId('entry-photo-modal')).toHaveCount(0);
});

// TODO(needs_staging): cross-tenant / cross-courier IDOR is NOT covered here — these specs
// mock the assignment, so they prove rendering only, not access control. A real proof needs a
// LIVE staging run with a REAL second courier (different tenant) hitting GET /courier/assignments/o1
// and asserting status 404 (route filters `WHERE ca.id=$1 AND ca.courier_id=$2`,
// apps/api/src/routes/courier/assignments.ts:112-115) so o1's customer PII (incl. entryPhotoUrl)
// never leaks. Requires a second-courier fixture token that does not exist in-sandbox; do not fake.
