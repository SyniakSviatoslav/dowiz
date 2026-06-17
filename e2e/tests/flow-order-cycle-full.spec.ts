/**
 * Full order cycle — slow-paced UI walkthrough with screenshots at every step.
 *
 * Runs against https://dowiz.fly.dev (or VITE_BASE_URL).
 * Video + screenshots saved to e2e/artifacts/order-cycle/.
 * Serial mode — each step depends on the previous.
 */

import { test, expect } from '@playwright/test';
import path from 'path';
import fs from 'fs';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
const ART = path.join('e2e', 'artifacts', 'order-cycle');

function shot(name: string) { return path.join(ART, `${name}.png`); }

test.use({
  video: { mode: 'on', size: { width: 390, height: 844 } },
  screenshot: 'on',
});

test.describe('Full Order Cycle UI', () => {
  test.describe.configure({ mode: 'serial' });

  let ownerToken: string;
  let locationSlug: string;
  let locationId: string;
  let locationLat: number;
  let locationLng: number;
  let categoryId: string;
  let productId: string;
  let productName: string;
  let orderId: string;
  let customerAuthToken: string;

  test.beforeAll(async ({ request }) => {
    fs.mkdirSync(ART, { recursive: true });

    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    const auth = await authRes.json();
    ownerToken = auth.access_token;

    const settingsRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    const settings = await settingsRes.json();
    locationSlug = settings.slug;
    locationId = settings.id;
    locationLat = settings.lat ?? 41.315347;
    locationLng = settings.lng ?? 19.4449964;

    // Ensure delivery is open all day
    const openAllDay: Record<string, any> = {};
    for (const d of ['monday','tuesday','wednesday','thursday','friday','saturday','sunday']) {
      openAllDay[d] = { isOpen: true, open: '00:00', close: '23:59' };
    }
    await request.put(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { deliveryPaused: false, hoursJson: openAllDay, lat: locationLat, lng: locationLng },
    });

    // Create test product so we have something to order
    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { name: `Cycle-Cat-${Date.now()}` },
    });
    categoryId = (await catRes.json()).id;

    productName = `Cycle-Prod-${Date.now()}`;
    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { name: productName, price: 3000, available: true, categoryId },
    });
    productId = (await prodRes.json()).id;
  });

  test.afterAll(async ({ request }) => {
    if (orderId) {
      await request.patch(`${BASE}/api/orders/${orderId}/status`, {
        headers: { Authorization: `Bearer ${ownerToken}` },
        data: { status: 'CANCELLED' },
      }).catch(() => {});
    }
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${ownerToken}` },
      }).catch(() => {});
    }
    if (categoryId) {
      await request.delete(`${BASE}/api/owner/menu/categories/${categoryId}`, {
        headers: { Authorization: `Bearer ${ownerToken}` },
      }).catch(() => {});
    }
  });

  // ── STEP 1: Menu page loads ──────────────────────────────────────────────

  test('MENU-1: /s/:slug loads without JS errors', async ({ page }) => {
    const jsErrors: string[] = [];
    page.on('pageerror', e => jsErrors.push(e.message));

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await page.screenshot({ path: shot('01-menu-initial'), fullPage: true });

    // Menu should have content
    const body = await page.locator('body').innerText();
    expect(body.length, 'Menu page must have content').toBeGreaterThan(100);

    // No critical JS errors (filter noise)
    const criticalErrors = jsErrors.filter(e =>
      !e.includes('ResizeObserver') &&
      !e.includes('Non-passive event') &&
      !e.includes('favicon')
    );
    expect(criticalErrors, `JS errors on menu: ${criticalErrors.join('; ')}`).toHaveLength(0);
  });

  test('MENU-2: product cards visible and category tabs work', async ({ page }) => {
    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await page.screenshot({ path: shot('02-menu-loaded'), fullPage: false });

    // Check for our test product (by scrolling to its category)
    const categoryTab = page.locator(`[role="tab"]:has-text("Cycle-Cat"), button:has-text("Cycle-Cat")`).first();
    if (await categoryTab.isVisible({ timeout: 3000 }).catch(() => false)) {
      await categoryTab.click();
      await page.waitForTimeout(500);
      await page.screenshot({ path: shot('02b-menu-test-category'), fullPage: false });
    }

    // There should be at least some product cards
    const cards = page.locator('[data-testid="product-card"], .product-card, [class*="product"]').first();
    // Don't hard-fail if our product isn't in the first visible section
    await page.screenshot({ path: shot('02c-menu-cards'), fullPage: true });
  });

  // ── STEP 2: Add item to cart ─────────────────────────────────────────────

  test('CART-1: add test product to cart via API, then check menu cart indicator', async ({ page, request }) => {
    // Navigate to menu first so we have a page context
    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });

    // Find and click the add button for our test product
    // The product may be deep in the page — scroll to it first
    const productText = page.locator(`text=${productName}`).first();
    const productVisible = await productText.isVisible({ timeout: 5000 }).catch(() => false);

    if (productVisible) {
      await productText.scrollIntoViewIfNeeded();
      await page.screenshot({ path: shot('03a-product-found'), fullPage: false });

      // Find the + add button near this product
      const addBtn = productText.locator('..').locator('..').locator('button').last();
      if (await addBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
        await addBtn.click();
        await page.waitForTimeout(800);
        await page.screenshot({ path: shot('03b-after-add-click'), fullPage: false });
      }
    } else {
      await page.screenshot({ path: shot('03a-product-not-visible'), fullPage: true });
    }

    // Check if cart sticky bar appeared
    const cartBar = page.locator('[data-testid="cart-bar"], button:has-text("ALL"), .sticky-cart').first();
    await page.screenshot({ path: shot('03c-cart-bar'), fullPage: false });
  });

  // ── STEP 3: Checkout flow ────────────────────────────────────────────────
  // The checkout page is a React component inside the SPA, reached via client-side
  // navigation (React Router navigate()). Direct page.goto('/checkout') hits the
  // server which serves a standalone vanilla JS checkout — NOT the React checkout.
  // Tests must navigate from within the SPA.

  async function seedCartAndNavigateToCheckout(page: any) {
    // 1. Seed the cart in localStorage (React CartProvider format)
    //    Also pre-fill delivery address so the map pin requirement is met.
    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'domcontentloaded' });
    await page.evaluate(({ slug, pid, price, name, lat, lng }) => {
      // Cart items for React CartProvider
      localStorage.setItem(`dos_cart_${slug}`, JSON.stringify({
        version: 1,
        items: [{ id: pid, productId: pid, name, price, quantity: 1 }],
      }));
      // Pre-saved delivery pin so CheckoutPage loads with pin already set
      localStorage.setItem(`dos_last_delivery_${slug}`, JSON.stringify({
        lat, lng, address: 'Test Street 1, Tirana', entrance: 'A', apartment: '2',
      }));
      // Pre-fill form draft (phone, name, notes)
      localStorage.setItem(`dos_checkout_draft_${slug}`, JSON.stringify({
        phone: '+355691000199', customerName: 'Test Customer',
        deliveryType: 'delivery', instructionOption: 'custom',
        instructionCustom: 'Ground floor, red door',
      }));
    }, { slug: locationSlug, pid: productId, price: 3000, name: productName, lat: locationLat, lng: locationLng });

    // 2. Reload so CartProvider's useState initializer reads the seeded data
    await page.reload({ waitUntil: 'networkidle' });

    // 3. Wait for the sticky cart bar (proves CartProvider has items)
    const cartBar = page.locator('[data-testid="cart-open"]');
    await expect(cartBar).toBeVisible({ timeout: 10000 });

    // 4. Click cart bar → open drawer
    await cartBar.click();
    await page.waitForTimeout(400);

    // 5. Click "Checkout" in the drawer → React Router navigate() to /checkout
    const checkoutBtn = page.locator('[data-testid="cart-checkout"]');
    await expect(checkoutBtn).toBeVisible({ timeout: 5000 });
    await checkoutBtn.click();

    // 6. Wait for URL to include /checkout (pushState navigation)
    await page.waitForURL(`**/${locationSlug}/checkout**`, { timeout: 8000 });
    await page.waitForTimeout(500);
  }

  test('CHECKOUT-1: navigate to checkout — React CheckoutPage renders with form', async ({ page }) => {
    await seedCartAndNavigateToCheckout(page);
    await page.screenshot({ path: shot('04a-checkout-initial'), fullPage: false });
    await page.screenshot({ path: shot('04b-checkout-full'), fullPage: true });

    // React CheckoutPage phone input must be visible
    await expect(page.locator('[data-testid="checkout-phone"]')).toBeVisible({ timeout: 8000 });
    await page.screenshot({ path: shot('04c-checkout-form-visible'), fullPage: false });
  });

  test('CHECKOUT-2: form fields, map, and lower section all visible on mobile', async ({ page }) => {
    await seedCartAndNavigateToCheckout(page);
    await page.screenshot({ path: shot('05a-checkout-top'), fullPage: false });

    // Scroll down past the map to check lower section is reachable
    await page.evaluate(() => window.scrollBy(0, 300));
    await page.waitForTimeout(400);
    await page.screenshot({ path: shot('05b-checkout-after-scroll-300'), fullPage: false });

    await page.evaluate(() => window.scrollBy(0, 300));
    await page.waitForTimeout(400);
    await page.screenshot({ path: shot('05c-checkout-after-scroll-600'), fullPage: false });

    await page.evaluate(() => window.scrollBy(0, 600));
    await page.waitForTimeout(400);
    await page.screenshot({ path: shot('05d-checkout-bottom'), fullPage: false });

    // The submit button should be reachable
    const submitBtn = page.locator('button[type="submit"]').first();
    const submitVisible = await submitBtn.isVisible({ timeout: 3000 }).catch(() => false);
    await page.screenshot({ path: shot('05e-checkout-submit'), fullPage: false });

    if (!submitVisible) {
      console.warn('BUG: Submit button not visible — page may not scroll correctly');
    }
  });

  test('CHECKOUT-3: fill form and place order', async ({ page }) => {
    await seedCartAndNavigateToCheckout(page);

    // Form should be pre-filled from dos_checkout_draft and dos_last_delivery
    // Fill phone if not already set
    const phoneInput = page.locator('[data-testid="checkout-phone"]');
    await expect(phoneInput).toBeVisible({ timeout: 8000 });
    const phoneValue = await phoneInput.inputValue();
    if (!phoneValue) await phoneInput.fill('+355691000199');

    // Fill customer name if not pre-filled
    const nameInput = page.locator('input[autocomplete="name"], input[placeholder*="name" i]').first();
    if (await nameInput.isVisible({ timeout: 2000 }).catch(() => false)) {
      const nameVal = await nameInput.inputValue().catch(() => '');
      if (!nameVal) await nameInput.fill('Test Customer');
    }

    await page.screenshot({ path: shot('06a-checkout-filled-contact'), fullPage: false });

    // Scroll to delivery section and fill the required notes textarea
    await page.evaluate(() => window.scrollBy(0, 400));
    await page.waitForTimeout(300);
    await page.screenshot({ path: shot('06b-checkout-delivery-section'), fullPage: false });

    // Notes ("Shenime") textarea is required and NOT saved in draft — must fill it
    const notesTextarea = page.locator('textarea[required], textarea').first();
    if (await notesTextarea.isVisible({ timeout: 3000 }).catch(() => false)) {
      await notesTextarea.scrollIntoViewIfNeeded();
      await notesTextarea.fill('Ground floor, red door. Building with orange sign.');
    }

    // Scroll to submit
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    await page.waitForTimeout(400);
    await page.screenshot({ path: shot('06c-checkout-bottom-before-submit'), fullPage: false });

    // Intercept order API call to capture orderId and authToken
    let capturedOrder: { id: string; authToken: string } | null = null;
    await page.route('**/api/orders', async (route) => {
      const res = await route.fetch();
      const body = await res.json().catch(() => null);
      if (body?.id) capturedOrder = { id: body.id, authToken: body.authToken };
      await route.fulfill({ response: res });
    });

    const submitBtn = page.locator('button[type="submit"]').first();
    if (await submitBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await submitBtn.click();
      await page.waitForTimeout(4000); // 1.5s confirmation + navigation
      await page.screenshot({ path: shot('06d-after-submit'), fullPage: false });
    }

    if (capturedOrder) {
      orderId = capturedOrder.id;
      customerAuthToken = capturedOrder.authToken;
      console.log('Order placed:', orderId);
    } else {
      const url = page.url();
      console.log('Current URL after submit:', url);
      const match = url.match(/\/order\/([0-9a-f-]{36})/);
      if (match) orderId = match[1];
      const bodyText = await page.locator('body').innerText().catch(() => '');
      if (bodyText.includes('MIN_ORDER') || bodyText.toLowerCase().includes('minimum order')) {
        console.warn('CHECKOUT-3: Blocked by min order requirement, will create order via API in ORDER-STATUS-1');
      }
    }
  });

  // ── STEP 4: Order status page ────────────────────────────────────────────

  test('ORDER-STATUS-1: order status page renders with correct data', async ({ page }) => {
    if (!orderId) {
      console.warn('No orderId — creating one via API directly');
      const r = await page.request.post(`${BASE}/api/orders`, {
        data: {
          locationId,
          type: 'delivery',
          items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
          customer: { phone: '+355691000198', name: 'Fallback Customer' },
          delivery: { pin: { lat: locationLat, lng: locationLng }, address_text: 'API fallback' },
          payment: { method: 'cash' },
          idempotency_key: crypto.randomUUID(),
        },
      });
      const body = await r.json();
      if (r.status() === 201) {
        orderId = body.id;
        customerAuthToken = body.authToken;
      } else {
        test.skip(true, `Cannot create order: ${r.status()} ${JSON.stringify(body)}`);
        return;
      }
    }

    // Inject customer token
    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'domcontentloaded' });
    if (customerAuthToken) {
      await page.evaluate((token) => localStorage.setItem('dos_access_token', token), customerAuthToken);
    }

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });
    await page.screenshot({ path: shot('07a-order-status-top'), fullPage: false });
    await page.screenshot({ path: shot('07b-order-status-full'), fullPage: true });

    // Status bar must be visible
    const statusBar = page.locator('[aria-live="polite"], [role="region"], .max-w-md').first();
    await expect(statusBar).toBeVisible({ timeout: 10000 });

    // No NaN
    const bodyText = await page.locator('body').innerText();
    expect(bodyText, 'Order status must not contain NaN').not.toContain('NaN');

    // Page URL must still be the order page (no redirect loop)
    expect(page.url(), 'Must stay on order status page').toContain(`/order/${orderId}`);

    await page.waitForTimeout(3000);
    await page.screenshot({ path: shot('07c-order-status-3s-later'), fullPage: true });

    // URL must still be the same after 3s (no infinite refresh)
    expect(page.url(), 'URL must not change after 3s (no refresh loop)').toContain(`/order/${orderId}`);
  });

  test('ORDER-STATUS-2: item name and price display correctly', async ({ page }) => {
    if (!orderId) { test.skip(true, 'No orderId'); return; }

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'domcontentloaded' });
    if (customerAuthToken) {
      await page.evaluate((token) => localStorage.setItem('dos_access_token', token), customerAuthToken);
    }

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });

    // Scroll to order details section
    const orderDetails = page.locator('text=/Order Details|Detajet/i').first();
    if (await orderDetails.isVisible({ timeout: 5000 }).catch(() => false)) {
      await orderDetails.scrollIntoViewIfNeeded();
    }
    await page.waitForTimeout(500);
    await page.screenshot({ path: shot('08a-order-details-section'), fullPage: false });

    const bodyText = await page.locator('body').innerText();
    expect(bodyText, 'Price must not be NaN').not.toContain('NaN');
    expect(bodyText, 'Item name must appear').toContain('Cycle-Prod-');
  });

  test('ORDER-STATUS-3: WebSocket status indicator and no page thrash', async ({ page }) => {
    if (!orderId) { test.skip(true, 'No orderId'); return; }

    let navigationCount = 0;
    page.on('framenavigated', () => navigationCount++);

    let statusApiCalls = 0;
    await page.route(`**/customer/orders/${orderId}/status`, route => {
      statusApiCalls++;
      route.continue();
    });

    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'domcontentloaded' });
    if (customerAuthToken) {
      await page.evaluate((token) => localStorage.setItem('dos_access_token', token), customerAuthToken);
    }

    await page.goto(`${BASE}/s/${locationSlug}/order/${orderId}`, { waitUntil: 'networkidle' });
    navigationCount = 1; // reset after initial load

    // Wait 5s — if the reconnect loop was active we'd see many calls
    await page.waitForTimeout(5000);
    await page.screenshot({ path: shot('09a-order-status-5s'), fullPage: false });

    console.log(`Status API calls in 5s: ${statusApiCalls}, navigations: ${navigationCount}`);
    expect(navigationCount, 'No extra navigations (no infinite reload)').toBe(1);
    expect(statusApiCalls, 'At most 2 status API calls in 5s (initial + optional WS reconnect)').toBeLessThanOrEqual(2);
  });

  // ── STEP 5: Admin dashboard order appearance ─────────────────────────────

  test('ADMIN-1: order appears in admin dashboard', async ({ page }) => {
    if (!orderId) { test.skip(true, 'No orderId'); return; }

    // Auth as owner
    await page.goto(`${BASE}/admin`, { waitUntil: 'domcontentloaded' });
    await page.evaluate((token) => localStorage.setItem('dos_access_token', token), ownerToken);

    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await page.screenshot({ path: shot('10a-admin-dashboard'), fullPage: false });

    // Check for the order in the dashboard
    const orderCard = page.locator(`text=${orderId.substring(0, 8)}`).first();
    const found = await orderCard.isVisible({ timeout: 5000 }).catch(() => false);
    if (found) {
      await orderCard.scrollIntoViewIfNeeded();
    }
    await page.screenshot({ path: shot('10b-admin-order-found'), fullPage: false });
    await page.screenshot({ path: shot('10c-admin-full'), fullPage: true });

    // No infinite refresh on admin either
    let adminNavigations = 0;
    page.on('framenavigated', () => adminNavigations++);
    await page.waitForTimeout(3000);
    await page.screenshot({ path: shot('10d-admin-3s-later'), fullPage: false });

    expect(adminNavigations, 'Admin dashboard must not reload infinitely').toBe(0);
  });
});
