import { test, expect } from '@playwright/test';

// Undo/redo (client-draft history) proof for MenuManagerPage's Add/Edit product form.
// Wiring guardrail (source-content checks) lives in
// apps/web/src/pages/admin/menu-manager-undo-redo.test.ts — this spec is the DOM-level
// proof that referenced file promises: pressing Undo/Redo (buttons AND keyboard) actually
// reverts/restores a field, and a fresh form starts with both controls disabled.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test undo-redo --project=desktop --reporter=list

test.describe('Menu Manager — Undo/Redo', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/admin/menu?dev=true', { waitUntil: 'networkidle' });
    await page.waitForTimeout(2000);
    const tabBar = page.locator('.overflow-x-auto.hide-scrollbar').first();
    await expect(tabBar).toBeVisible({ timeout: 10000 });
    const tabs = tabBar.locator('button');
    if ((await tabs.count()) > 0) await tabs.first().click();
    await page.waitForTimeout(500);
  });

  test('a fresh Add Item form shows the undo/redo toolbar, both controls disabled', async ({ page }) => {
    await page.getByRole('button', { name: 'Add', exact: true }).first().click();
    const toolbar = page.locator('[data-testid="undo-redo-toolbar"]');
    await expect(toolbar).toBeVisible({ timeout: 10000 });
    // No edits yet → nothing to undo/redo.
    await expect(page.locator('[data-testid="undo-button"]')).toBeDisabled();
    await expect(page.locator('[data-testid="redo-button"]')).toBeDisabled();
  });

  test('Undo reverts and Redo restores an edited product name', async ({ page }) => {
    await page.getByRole('button', { name: 'Add', exact: true }).first().click();
    const nameInput = page.getByPlaceholder('e.g. Margherita Pizza');
    await expect(nameInput).toBeVisible({ timeout: 10000 });

    await nameInput.fill('First Draft');
    await page.waitForTimeout(200); // let the snapshot effect record the history entry
    await nameInput.fill('Second Draft');
    await page.waitForTimeout(200);
    await expect(nameInput).toHaveValue('Second Draft');

    const undoBtn = page.locator('[data-testid="undo-button"]');
    await expect(undoBtn).toBeEnabled();
    await undoBtn.click();
    await expect(nameInput).toHaveValue('First Draft');

    const redoBtn = page.locator('[data-testid="redo-button"]');
    await expect(redoBtn).toBeEnabled();
    await redoBtn.click();
    await expect(nameInput).toHaveValue('Second Draft');
  });

  test('Cmd/Ctrl+Z and Cmd/Ctrl+Shift+Z drive the same history as the buttons', async ({ page }) => {
    await page.getByRole('button', { name: 'Add', exact: true }).first().click();
    const nameInput = page.getByPlaceholder('e.g. Margherita Pizza');
    await expect(nameInput).toBeVisible({ timeout: 10000 });

    await nameInput.fill('Keyboard Draft A');
    await page.waitForTimeout(200);
    await nameInput.fill('Keyboard Draft B');
    await page.waitForTimeout(200);

    const isMac = process.platform === 'darwin';
    await nameInput.focus();
    await page.keyboard.press(isMac ? 'Meta+Z' : 'Control+Z');
    await expect(nameInput).toHaveValue('Keyboard Draft A');

    await page.keyboard.press(isMac ? 'Meta+Shift+Z' : 'Control+Shift+Z');
    await expect(nameInput).toHaveValue('Keyboard Draft B');
  });
});

// Flag-off behavior (VITE_UNDO_REDO_ENABLED=false hides the toolbar and disables the
// shortcuts) is a build-time constant baked in at compile time — it cannot be toggled from
// a running page, so it is proven at the source level instead:
// apps/web/src/pages/admin/menu-manager-undo-redo.test.ts
// ("the <UndoRedoButtons> render must be inside a {UNDO_REDO_ENABLED && ...} gate").
