// spec: specs/client-checkout.md
// seed: e2e/tests/seed.spec.ts
//
// Planner→Generator→Healer output (Tooling Plan v2, Step 4), healed against the
// live SPA. Relative URLs resolve to playwright.config `use.baseURL` — the
// interactive client app served by `apps/web` vite on :5173 (run client browser
// tests with VITE_BASE_URL=http://localhost:5173 + `pnpm dev:all`). Prod
// /s/:slug is a static SSR menu only; the cart/checkout SPA does not exist there.
// `?dev=true` makes devBootstrap.ts mock the menu API client-side.
//
// Coverage: browse menu → add to cart → cart bar → cart drawer → proceed to
// checkout → fill order details → place order → order-status page.
//
// Order placement requires CheckoutPage's locationId, which it loads from
// GET /public/locations/:slug/info. That mock was missing from devBootstrap, so
// in ?dev=true mode locationId stayed null and place-order silently early-returned
// (CheckoutPage.tsx:269). Fixed by adding the mock in apps/web/src/api/mockData.ts;
// this test is the proof the full funnel now completes.

import { test, expect } from '@playwright/test';

test.describe('Client Checkout Flow', () => {
  test('Browse menu, add item, open cart, checkout, order placed', async ({ page, request }) => {
    // Resolve the seeded location slug from the owner settings API (relative →
    // baseURL; vite proxies /api → the dev API), so the test follows the active
    // location rather than a literal.
    const authRes = await request.post(`/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const { access_token } = await authRes.json();

    const settingsRes = await request.get(`/api/owner/settings`, {
      headers: { Authorization: `Bearer ${access_token}` },
    });
    expect(settingsRes.status()).toBe(200);
    const { slug } = await settingsRes.json();

    // 1. Navigate to the menu and wait for product cards to render
    await page.goto(`/s/${slug}?dev=true`);
    const firstCard = page.locator('main article').first();
    await expect(firstCard).toBeVisible({ timeout: 20000 });

    // Menu heading and the sticky category tab bar (≥2 categories)
    await expect(page.locator('h1').first()).toContainText(/Menu/i);
    expect(await page.getByRole('tab').count()).toBeGreaterThanOrEqual(2);

    // Cart bar is absent while the cart is empty
    await expect(page.getByRole('button', { name: /Shporta/i })).toHaveCount(0);

    // 2. Add the first available item to the cart (testid is locale-independent;
    //    the aria-label is translated, so a text selector would be locale-fragile).
    await firstCard.getByTestId('menu-item-add').click();

    // The cart bar appears showing one item and its line total
    const cartBar = page.getByRole('button', { name: /Shporta/i });
    await expect(cartBar).toBeVisible({ timeout: 5000 });
    await expect(cartBar).toContainText('1');
    await expect(cartBar).toContainText(/ALL/);

    // 3. Open the cart drawer and proceed to checkout
    await cartBar.click();
    const proceed = page.getByRole('button', { name: 'Porosit', exact: true });
    await expect(proceed).toBeVisible({ timeout: 5000 });
    await proceed.click();

    // 4. Checkout page renders with the contact/delivery form
    await expect(page).toHaveURL(/\/checkout/, { timeout: 8000 });
    await expect(page.getByText(/Informacioni i Kontaktit|Contact/i).first()).toBeVisible({ timeout: 8000 });

    // Delivery-type tabs (Delivery / Pickup / Scheduled)
    expect(await page.getByRole('tab').count()).toBeGreaterThanOrEqual(2);

    const nameInput = page.getByPlaceholder('Emri juaj');
    const phoneInput = page.getByTestId('checkout-phone');
    const addressInput = page.getByPlaceholder(/Adresa/i).first();
    const entranceInput = page.getByTestId('checkout-entrance');
    const apartmentInput = page.getByTestId('checkout-apartment');
    const notes = page.locator('textarea').first();

    await expect(nameInput).toBeVisible({ timeout: 5000 });
    await expect(phoneInput).toBeVisible();
    await expect(page.getByTestId('checkout-total')).toBeVisible();
    await expect(page.getByTestId('order-confirm-button')).toBeVisible();

    // 5. Fill the order details
    await nameInput.fill('Sara Mancini');
    await expect(nameInput).toHaveValue('Sara Mancini');

    await phoneInput.fill('+355691234567');
    await expect(phoneInput).toHaveValue('+355691234567');
    await expect(page.locator('[role="alert"]')).toHaveCount(0);

    await addressInput.fill('Rruga e Durrësit, Tirana');
    await expect(addressInput).toHaveValue('Rruga e Durrësit, Tirana');

    await entranceInput.fill('2');
    await expect(entranceInput).toHaveValue('2');

    await apartmentInput.fill('5');
    await expect(apartmentInput).toHaveValue('5');

    await notes.fill('Blue gate, third floor, ring the bell');
    await expect(notes).toHaveValue('Blue gate, third floor, ring the bell');

    // 6. Order summary reflects the cart (item 850 + delivery 200 = 1050 ALL)
    await expect(page.getByTestId('checkout-total')).toContainText(/1050\s*ALL/);

    // 7. Place the order → redirect to the order-status page. In ?dev=true mode
    //    the order completes via the intended dev fallback (CheckoutPage.tsx:352
    //    → /s/:slug/order/o_mock_123); reaching it requires the locationId fix above.
    const placeOrder = page.getByTestId('order-confirm-button');
    await expect(placeOrder).toBeEnabled();
    await placeOrder.click();

    await expect(page).toHaveURL(/\/order\/o_mock_123/, { timeout: 8000 });
    await expect(page.locator('body')).toBeVisible();
    await expect(page.locator('body')).toContainText(
      /Duke u pergatitur|preparing|pending|delivery|order|porosi/i,
      { timeout: 10000 },
    );
  });
});
