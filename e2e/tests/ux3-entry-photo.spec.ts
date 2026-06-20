import { test, expect } from '@playwright/test';

// UX-3 entry-anchor photo — courier side. Mocks the assignment so the entrance
// photo surfaces on the delivery screen; proves the thumbnail renders and taps
// open a fullscreen view. (Upload + R2 storage mirror the proven product-image
// path; R2 isn't available in-sandbox so the upload itself isn't E2E here.)
const PNG = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+M8AAAMBAQDJ/pLvAAAAAElFTkSuQmCC';

test('courier sees the entry-anchor photo and can open it fullscreen', async ({ page }) => {
  await page.route('**/public/**', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
  await page.route('**/api/**', (r: any) => r.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
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
  await thumb.click();
  await expect(page.getByTestId('entry-photo-modal')).toBeVisible();
});
