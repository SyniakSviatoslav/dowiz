/* eslint-disable @typescript-eslint/no-explicit-any, local/no-empty-catch -- test/spec/spike/helper code -- flagged strings are test data, selectors, logs, error codes and SQL, not user-facing UI copy; any/raw-any are deliberate test/integration seams */
import { test, expect } from '@playwright/test';

// Go-live remediation — live checkout proofs against the demo tenant (staging).
// #5 privacy notice (softened, decision-b copy) and #4 failure fallback + no
// fake-success. Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec
// playwright test golive-remediation --reporter=list

const CARD = '[data-testid="menu-item"]';

async function addItemAndGoToCheckout(page: any, slug = 'demo') {
  await page.goto(`/s/${slug}`);
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.waitForSelector(CARD, { timeout: 20000 });
  await page.locator('[data-testid="menu-item-add"]').first().click();
  // demo (sushi-durres mirror) has no modifiers → quick-add; tolerate a detail modal.
  const confirm = page.locator('[data-testid="product-detail-confirm"]');
  try { await confirm.waitFor({ state: 'visible', timeout: 2500 }); await confirm.click(); } catch { /* quick-added */ }
  await expect(page.locator('[data-testid="cart-open"]')).toBeVisible({ timeout: 8000 });
  await page.locator('[data-testid="cart-open"]').click();
  await page.locator('[data-testid="cart-checkout"]').click();
  await expect(page).toHaveURL(/\/checkout/);
}

test.describe('Go-live remediation — checkout', () => {
  test('#5 privacy notice visible near submit, anonymize-not-delete copy, no hard number', async ({ page }) => {
    await addItemAndGoToCheckout(page);
    const notice = page.locator('[data-testid="checkout-privacy-notice"]');
    await expect(notice).toBeVisible();
    // "remove the details that identify you" — anonymize-not-delete (sq/en/uk variants).
    await expect(notice).toContainText(/identifik|identify|ідентиф/i);
    // Decision (b): never "delete everything"; never a hard retention day-count promise.
    await expect(notice).not.toContainText(/fshijmë gjithçka|delete everything/i);
    await expect(notice).not.toContainText(/\d+\s*(ditë|days|дн)/i);
  });

  test('#4 forced order failure → phone CTA, no fake success, cart preserved', async ({ page }) => {
    await addItemAndGoToCheckout(page);
    // pickup avoids the delivery map-pin requirement so we reach the order POST.
    await page.getByRole('tab', { name: /Marr|Pick up|Забрати/ }).click();
    await page.locator('[data-testid="checkout-phone"]').fill('+355691112233');
    await page.locator('input[autocomplete="name"]').fill('Test Customer');
    // Force the order POST to 5xx (the pool-wedge failure mode #2 turns into a fast 5xx).
    await page.route('**/api/orders', (route: any) =>
      route.fulfill({ status: 500, contentType: 'application/json', body: JSON.stringify({ error: 'forced' }) }));
    await page.locator('[data-testid="order-confirm-button"]').click();
    // Phone CTA appears (cached on mount, so it does not depend on a fetch under load).
    const cta = page.locator('[data-testid="checkout-call-restaurant"]');
    await expect(cta).toBeVisible({ timeout: 8000 });
    await expect(cta).toHaveAttribute('href', /^tel:/);
    // NO fake success: the o_mock_123 dev-mock is compile-time dead-stripped in the prod build.
    await expect(page).not.toHaveURL(/o_mock_123/);
    await expect(page).toHaveURL(/\/checkout/);
    // Cart preserved — the order summary still renders (clearCart only runs on success;
    // an emptied cart would swap the checkout for its empty-state view, no total).
    await expect(page.locator('[data-testid="checkout-total"]')).toBeVisible();
  });
});
