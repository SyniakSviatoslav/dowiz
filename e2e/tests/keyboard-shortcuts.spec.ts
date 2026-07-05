import { test, expect } from '@playwright/test';

// Admin power-user layer (AdminCommandCenter): ⌘K/Ctrl+K command palette, "g"-sequence
// nav, and the "?" shortcuts help sheet. The keychord matcher (parse/match/sequence) is
// unit-tested in packages/ui/src/hooks/__tests__/use-keyboard-shortcuts.test.ts; this spec
// proves the DOM wiring: the shortcuts actually open/close the right overlay from the
// admin shell.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test keyboard-shortcuts --project=desktop --reporter=list

test.describe('Admin — Keyboard Shortcuts', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/admin?dev=true', { waitUntil: 'networkidle' });
    await page.waitForTimeout(1500);
  });

  test('Mod+K opens the command palette; Escape closes it', async ({ page }) => {
    const isMac = process.platform === 'darwin';
    await page.keyboard.press(isMac ? 'Meta+K' : 'Control+K');
    const palette = page.locator('[data-testid="command-palette"]');
    await expect(palette).toBeVisible({ timeout: 5000 });

    await page.keyboard.press('Escape');
    await expect(palette).not.toBeVisible({ timeout: 5000 });
  });

  test('typing in the palette filters commands, and Enter navigates', async ({ page }) => {
    const isMac = process.platform === 'darwin';
    await page.keyboard.press(isMac ? 'Meta+K' : 'Control+K');
    const palette = page.locator('[data-testid="command-palette"]');
    await expect(palette).toBeVisible({ timeout: 5000 });

    const input = palette.locator('input[role="combobox"]');
    await input.fill('menu');
    await expect(page.locator('#command-palette-list li')).toHaveCount(1, { timeout: 5000 });

    await input.press('Enter');
    await expect(palette).not.toBeVisible({ timeout: 5000 });
    await expect(page).toHaveURL(/\/admin\/menu/);
  });

  test('"?" opens the shortcuts help sheet listing the active shortcuts', async ({ page }) => {
    // Focus something non-editable first so "?" is not swallowed as text entry.
    await page.locator('body').click();
    await page.keyboard.press('?');
    const help = page.locator('[data-testid="shortcuts-help"]');
    await expect(help).toBeVisible({ timeout: 5000 });
    await expect(help).toContainText('K'); // the Mod+K entry's <kbd>

    await page.keyboard.press('Escape');
    await expect(help).not.toBeVisible({ timeout: 5000 });
  });

  test('"g m" sequence navigates to the Menu section', async ({ page }) => {
    await page.locator('body').click();
    await page.keyboard.press('g');
    await page.keyboard.press('m');
    await expect(page).toHaveURL(/\/admin\/menu/, { timeout: 5000 });
  });

  test('shortcuts do not fire while typing in a text field (except Mod+K)', async ({ page }) => {
    const search = page.locator('input[type="text"], input[type="search"]').first();
    if (await search.count()) {
      await search.click();
      await search.type('g');
      await search.type('m');
      // Plain "g m" while typing must not navigate away from the current admin route.
      await expect(page).not.toHaveURL(/\/admin\/menu$/);
    }
  });
});
