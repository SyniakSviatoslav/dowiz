import { test, expect } from '@playwright/test';
import { requireStaging } from '../helpers/staging-guard';

// Go-live remediation — live checkout proofs against the demo tenant (staging).
// #5 privacy notice (softened, decision-b copy) and #4 failure fallback + no
// fake-success. Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec
// playwright test golive-remediation --reporter=list

const CARD = '[data-testid="menu-item"]';
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

async function addItemAndGoToCheckout(page: any, slug = 'demo') {
  // Clear BEFORE the document scripts run, else a prior test's persisted cart is
  // hydrated before the clear and leaks in (finding #1). SPA nav to /checkout is
  // client-side (no document reload), so this init script fires only on this goto.
  await page.addInitScript(() => localStorage.clear());
  await page.goto(`/s/${slug}`);
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
  // Mutating spec (drives a live order POST against the demo tenant). Fail fast on
  // an unknown/prod target so a run can never write to prod.
  test.beforeAll(() => requireStaging(BASE));

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
    // Snapshot the cart total BEFORE the failure — it is derived from item count +
    // identity + qty, so a byte-identical total after the 5xx proves the cart was
    // preserved (not silently emptied/mutated), beyond merely "a total is visible".
    const totalBefore = (await page.locator('[data-testid="checkout-total"]').textContent())?.trim();
    expect(totalBefore, 'cart total must be present before submit').toBeTruthy();
    // Force the order POST to 5xx (the pool-wedge failure mode #2 turns into a fast 5xx).
    // Count + fulfill EVERY matching request so no silent auto-retry can escape the mock
    // and POST a real order to staging (finding #3); assert exactly one POST was attempted.
    let orderPostCount = 0;
    await page.route('**/api/orders', (route: any) => {
      orderPostCount += 1;
      return route.fulfill({ status: 500, contentType: 'application/json', body: JSON.stringify({ error: 'forced' }) });
    });
    await page.locator('[data-testid="order-confirm-button"]').click();
    // Phone CTA appears (cached on mount, so it does not depend on a fetch under load).
    const cta = page.locator('[data-testid="checkout-call-restaurant"]');
    await expect(cta).toBeVisible({ timeout: 8000 });
    await expect(cta).toHaveAttribute('href', /^tel:/);
    // NO fake success: the o_mock_123 dev-mock is compile-time dead-stripped in the prod build.
    await expect(page).not.toHaveURL(/o_mock_123/);
    await expect(page).toHaveURL(/\/checkout/);
    // Cart preserved — the order summary still renders (clearCart only runs on success;
    // an emptied cart would swap the checkout for its empty-state view, no total) AND the
    // total is byte-identical to the pre-failure snapshot → item count/identity unchanged.
    const totalEl = page.locator('[data-testid="checkout-total"]');
    await expect(totalEl).toBeVisible();
    await expect(totalEl).toHaveText(totalBefore as string);
    // Exactly one order POST was attempted — no silent auto-retry escaped to staging.
    expect(orderPostCount).toBe(1);
  });
});
