import { test, expect } from '@playwright/test';

// Phase-2 cinematic-media render proof against the live (flag-on) staging demo storefront.
// MediaGallery only mounts when the lazy media endpoint returns >1 item, so its presence is
// unambiguous proof: read_public_menu surfaced primary_media_id → the modal lazy-fetched the
// media set → the code-split renderer chunk loaded → rich media rendered (not the
// image_key/gradient fallback). Seed 057 gave "Sweet Chili Tiger Premium" a 3-image gallery.

test('rich media gallery renders in the product modal on /s/demo', async ({ page }) => {
  await page.goto('/s/demo');

  // Menu renders client-side; wait for the seeded product to appear, then open it.
  const card = page.getByText('Sweet Chili Tiger', { exact: false }).first();
  await expect(card).toBeVisible({ timeout: 25000 });
  await card.click();

  // The gallery orchestrator container — only present for a >1 media set.
  const gallery = page.locator('.dz-media-gallery');
  await expect(gallery).toBeVisible({ timeout: 15000 });

  // It shows a seeded media image (proves the http-passthrough resolver + actual render).
  await expect(gallery.locator('img[src*="wikimedia"]').first()).toBeVisible();
});

test('spin product opens with a rich-media renderer on /s/demo', async ({ page }) => {
  await page.goto('/s/demo');

  const card = page.getByText('Red Pearl', { exact: false }).first();
  await expect(card).toBeVisible({ timeout: 25000 });
  await card.click();

  // Red Pearl has a single `spin` media (primary set by 058) → a seeded media image renders
  // (SpinViewer's poster/frame), proving the single-renderer path, not the gradient fallback.
  await expect(page.locator('img[src*="wikimedia"]').first()).toBeVisible({ timeout: 15000 });
});
