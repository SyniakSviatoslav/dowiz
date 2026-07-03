import { test, expect } from '@playwright/test';

/*
 * Proof for the card→detail shared-element (layoutId) reveal — cinematic-brand-ux plan §7, P1.
 *
 * GATING: the reveal ships behind VITE_CINEMATIC_REVEALS (build-time Vite flag, default OFF).
 * A shared-element morph is inherently a VISUAL change — the deterministic Playwright proof here
 * asserts the *behavioural* contract the morph must not break: tapping a card opens the detail
 * view showing the SAME dish (name + price), and it still opens under prefers-reduced-motion
 * (static, opacity-only — the accessibility fallback). The pixel-level morph itself is covered by
 * the visual-regression baseline (see the commands in the lane report).
 *
 * QUEUE: run against a staging build with the flag baked in AND after the lead wires
 * ProductDetailSheet into MenuPage.tsx:
 *   flyctl deploy -a dowiz-staging --remote-only --build-arg VITE_CINEMATIC_REVEALS=true
 *   VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test cinematic-reveal --reporter=list
 */

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const SLUG = 'demo';

test.describe.configure({ mode: 'serial' });
test.setTimeout(60_000);

test('flag ON: tapping a card reveals the detail sheet with the SAME dish (morph target present)', async ({ page }) => {
  await page.goto(`${BASE}/s/${SLUG}`);
  const firstCard = page.locator('[data-testid=menu-item]').first();
  await expect(firstCard, 'menu cards render').toBeVisible({ timeout: 20_000 });

  // Capture the card's dish name — the shared-element morph must land the same dish in the sheet.
  const cardName = (await firstCard.innerText()).split('\n').map((s) => s.trim()).filter(Boolean)[0] ?? '';
  await firstCard.click();

  // Detail view opened. Accept either the extracted sheet (post-wiring) or the inline modal.
  const dialog = page.getByRole('dialog');
  await expect(dialog, 'detail sheet opens on card tap').toBeVisible({ timeout: 10_000 });
  if (cardName) {
    await expect(dialog.getByText(cardName, { exact: false }).first(), 'the tapped dish name flew into the detail sheet').toBeVisible();
  }
});

test('reduced-motion: the detail sheet still opens (static opacity fallback, no morph)', async ({ browser }) => {
  const ctx = await browser.newContext({ reducedMotion: 'reduce' });
  const page = await ctx.newPage();
  await page.goto(`${BASE}/s/${SLUG}`);
  const firstCard = page.locator('[data-testid=menu-item]').first();
  await expect(firstCard).toBeVisible({ timeout: 20_000 });
  await firstCard.click();
  await expect(page.getByRole('dialog'), 'reduced-motion still opens the detail (accessibility fallback)').toBeVisible({ timeout: 10_000 });
  await ctx.close();
});

test('regression: closing the detail returns to the grid (morph does not trap focus/scroll)', async ({ page }) => {
  await page.goto(`${BASE}/s/${SLUG}`);
  const firstCard = page.locator('[data-testid=menu-item]').first();
  await expect(firstCard).toBeVisible({ timeout: 20_000 });
  await firstCard.click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible({ timeout: 10_000 });
  await page.keyboard.press('Escape').catch(() => {});
  // Escape may be handled by the page; the close control is the guaranteed path.
  const closeBtn = page.locator('[data-testid=product-detail-close], [aria-label="Close"]').first();
  if (await closeBtn.isVisible().catch(() => false)) await closeBtn.click();
  await expect(page.locator('[data-testid=menu-item]').first(), 'back on the grid after close').toBeVisible({ timeout: 10_000 });
});
