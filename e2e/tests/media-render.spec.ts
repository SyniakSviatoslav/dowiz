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

  // Capture the rendered slide image's HTTP response so visibility alone (which Playwright
  // grants to a 404'd <img> tag) cannot satisfy the test — the byte payload must be a real 200.
  const imageResponse = page.waitForResponse(
    (r) => r.request().resourceType() === 'image' && /wikimedia/.test(r.url()),
    { timeout: 20000 },
  );
  await card.click();

  // The gallery orchestrator container — keyed off a semantic data-testid (only mounted for a
  // >1 media set: 0 → null, 1 → dz-media-hero), so a class rename/misapply can't fake presence.
  const gallery = page.getByTestId('media-gallery');
  await expect(gallery).toBeVisible({ timeout: 15000 });

  // Proves the gallery surfaced the full 3-item seeded set, not a degraded single-image layout:
  // one dot ("tab") is rendered per media item. (The slide <img> count can't be used — the gallery
  // mounts exactly ONE heavy decode at a time, so only the active slide has an <img> in the DOM.)
  await expect(gallery.getByRole('tab')).toHaveCount(3);

  // It shows a seeded media image (proves the http-passthrough resolver + actual render)…
  await expect(gallery.locator('img[src*="wikimedia"]').first()).toBeVisible();
  // …and that image actually loaded over the wire (status 200, not a broken-but-visible <img>).
  const imageStatus = (await imageResponse).status();
  expect(imageStatus).toBe(200);
});

// NOTE: the `spin` (SpinViewer) path shares the exact render pipeline the gallery test proves
// above — lazy media fetch on modal open → code-split renderer chunk → render. It is verified
// at the data level (the lazy endpoint returns the spin with a resolved posterUrl + 4 frameUrls
// for the demo) and unit level (SpinViewer renders the poster <img>). A browser E2E for it was
// dropped: the demo has two products named "Red Pearl" (only the Chef's-Picks one is seeded),
// which makes a name-based storefront click ambiguous — a test-data artifact, not a feature bug.
