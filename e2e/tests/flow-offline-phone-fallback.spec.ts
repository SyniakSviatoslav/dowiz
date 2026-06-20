import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';

/**
 * Invariant §4 — "fallback-на-телефон живий": when the live channel drops, the
 * customer must still be able to reach the restaurant. Proves the order-status
 * offline banner surfaces a tel: link to the location's configured fallback phone
 * (owner-gated by show_phone_on_offline).
 *
 * Strategy: mint a real cash order on a public slug → open its self-authenticating
 * track URL → intercept the WebSocket and close it on every attempt (REST, incl.
 * fallback-config, stays up) → assert the banner + call affordance render.
 *
 * Bootstraps from PUBLIC endpoints only, so it runs against prod / any VITE_BASE_URL.
 */

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
const SLUG = process.env.TRACK_SLUG || 'sushi-durres'; // OTP off, min_order 0, real coords
const DELIVERY_LAT = Number(process.env.TRACK_LAT ?? 41.315347);
const DELIVERY_LNG = Number(process.env.TRACK_LNG ?? 19.4449964);

test.describe.configure({ mode: 'serial' });

test.describe('Flow: Offline phone fallback on order status', () => {
  let locationId: string;
  let productId: string;
  let expectedPhone: string;

  test.beforeAll(async ({ request }) => {
    const menuRes = await request.get(`${BASE}/public/locations/${SLUG}/menu`);
    expect(menuRes.status()).toBe(200);
    const menu = await menuRes.json();
    locationId = menu.locationId || menu.location_id;
    expect(locationId, 'public menu must expose a locationId').toBeTruthy();

    const products = (menu.categories || []).flatMap((c: any) => c.products || []);
    const product =
      products.find((p: any) => p.available !== false && (!p.modifier_groups || p.modifier_groups.length === 0)) ||
      products.find((p: any) => p.available !== false);
    expect(product, 'at least one available product').toBeTruthy();
    productId = product.id;

    const fcRes = await request.get(`${BASE}/api/public/locations/${SLUG}/fallback-config`);
    expect(fcRes.status()).toBe(200);
    const fc = await fcRes.json();
    expect(fc.showPhoneOnOffline, 'location opts into offline phone fallback').not.toBe(false);
    expect(fc.phone, 'location has a fallback phone configured').toBeTruthy();
    expectedPhone = fc.phone;
  });

  test('offline banner surfaces a tel: link to the restaurant phone', async ({ page, request }) => {
    // 1. Mint a real cash order → self-authenticating track URL (?t= grant).
    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
        customer: { phone: `+35569${String(Date.now()).slice(-7)}`, name: 'Offline Test' },
        delivery: { pin: { lat: DELIVERY_LAT, lng: DELIVERY_LNG }, address_text: 'Test Street 1, Durres' },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });
    if (orderRes.status() !== 201) {
      const body = await orderRes.json().catch(() => ({}));
      expect(orderRes.status(), `order not 201 (${orderRes.status()}: ${JSON.stringify(body)})`).not.toBe(500);
      expect(orderRes.status()).not.toBe(401);
      test.skip(true, `Order not created (status ${orderRes.status()}); cannot reach status page`);
      return;
    }
    const { trackUrl } = await orderRes.json();
    expect(typeof trackUrl, 'mint returns a trackUrl').toBe('string');
    expect(trackUrl).toContain('/order/');

    // The server mints an absolute trackUrl on its own origin; rebind it to the
    // origin under test (so the local FE build is exercised when BASE is local).
    const navUrl = new URL(trackUrl);
    const baseUrl = new URL(BASE);
    navUrl.protocol = baseUrl.protocol;
    navUrl.host = baseUrl.host;

    // 2. Force the live channel down: intercept the WS and close it on every attempt.
    //    REST stays up, so the order loads and fallback-config still resolves.
    await page.routeWebSocket(/\/ws(\?|$)/, (ws) => ws.close());

    // 3. Open the self-authenticating track URL.
    await page.goto(navUrl.toString());

    // Order must load → proves we're past the loading/error gates and on the status page.
    await expect(page.getByTestId('order-status-badge')).toBeVisible({ timeout: 20000 });

    // 4. Offline banner + restaurant call affordance must be present and correct.
    await expect(page.getByTestId('offline-banner')).toBeVisible({ timeout: 20000 });

    const callLink = page.getByTestId('offline-call-restaurant');
    await expect(callLink).toBeVisible();
    await expect(callLink).toHaveAttribute('href', `tel:${expectedPhone}`);
    await expect(callLink).toContainText(/call/i);
  });
});
