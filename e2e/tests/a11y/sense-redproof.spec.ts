/**
 * Red-proof for the Non-Pixel Verification Net (Senses 1 & 2).
 *
 * A gate only counts if it fails when the code is wrong. These tests construct a
 * deliberately-broken DOM and assert that each sense DETECTS the defect — so the
 * gate is proven non-vacuous. They pass BY catching the planted bug (no network).
 */
import { test, expect } from '@playwright/test';
import { attachConsoleGuard } from '../../fixtures/console-guard';
import { checkAxe } from '../../helpers/a11y';

test('Sense 2 detects a console.error', async ({ page }) => {
  const guard = attachConsoleGuard(page);
  await page.setContent('<!doctype html><html lang="en"><body><main>x</main></body></html>');
  await page.evaluate(() => console.error('planted runtime error'));
  await page.waitForTimeout(50);
  expect(guard.errors.some((e) => e.includes('planted runtime error'))).toBe(true);
});

test('Sense 2 never allowlists a hydration mismatch', async ({ page }) => {
  const guard = attachConsoleGuard(page);
  await page.setContent('<!doctype html><html lang="en"><body><main>x</main></body></html>');
  await page.evaluate(() => console.warn('Hydration failed because the initial UI did not match'));
  await page.waitForTimeout(50);
  expect(guard.errors.some((e) => e.includes('Hydration failed'))).toBe(true);
});

test('Sense 1 (axe) detects an unlabeled input + low contrast', async ({ page }) => {
  await page.setContent(`<!doctype html><html lang="en"><body><main>
    <input type="text" name="q" />
    <p style="color:#bbb;background:#fff;font-size:12px">low contrast text</p>
  </main></body></html>`);
  const issues = await checkAxe(page);
  expect(issues.length, JSON.stringify(issues)).toBeGreaterThan(0);
  const ids = issues.map((i) => i.id);
  expect(ids, JSON.stringify(issues)).toContain('label');
  expect(ids, JSON.stringify(issues)).toContain('color-contrast');
});
