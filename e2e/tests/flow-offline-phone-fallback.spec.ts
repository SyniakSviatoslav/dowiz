import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';
<<<<<<< Updated upstream
=======
import { expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';
>>>>>>> Stashed changes

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
 * Bootstraps from PUBLIC endpoints, but MINTS a real order (mutating) — so it is
 * guarded to staging/local only (never prod) via requireStaging() in beforeAll.
 */

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const SLUG = process.env.TRACK_SLUG || 'sushi-durres'; // OTP off, min_order 0, real coords
const DELIVERY_LAT = Number(process.env.TRACK_LAT ?? 41.315347);
const DELIVERY_LNG = Number(process.env.TRACK_LNG ?? 19.4449964);

test.describe.configure({ mode: 'serial' });

test.describe('Flow: Offline phone fallback on order status', () => {
  let locationId: string;
  let productId: string;
  let expectedPhone: string;

  test.beforeAll(async ({ request }) => {
    requireStaging(BASE); // mutating spec (mints an order) — never run against prod

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
    // Strict: route returns a real boolean (config.show_phone_on_offline !== false);
    // undefined/null here would mean a contract break, so demand exactly `true`.
    expect(fc.showPhoneOnOffline, 'location opts into offline phone fallback').toBe(true);
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
    // orders.ts mints a fresh order at exactly 201 (200 is the idempotency-replay
    // branch, which a unique idempotency_key never hits). Any non-201 — 403/422/409/
    // 429/500/401 — is a hard failure, not a silent skip.
    const orderBody = await orderRes.text();
    expect(orderRes.status(), `order create failed (${orderRes.status()}: ${orderBody})`).toBe(201);
    const { trackUrl } = JSON.parse(orderBody);
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

  // TODO(needs_staging): negative control — on a location with show_phone_on_offline=false,
  // the offline banner's call affordance must be ABSENT even when the WS is forced down.
  // Requires a real second staging location configured with the flag disabled (e.g.
  // process.env.TRACK_SLUG_NO_FALLBACK). Asserting fallback-config.showPhoneOnOffline===false
  // then `await expect(page.getByTestId('offline-call-restaurant')).toHaveCount(0)`.
  // Not added as a runtime-skipping stub (would be vacuous); add once the fixture exists.
});
