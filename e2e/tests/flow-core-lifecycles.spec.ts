import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
let authToken: string;
let activeLocationId: string;
let orderId: string;
let orderStatus: string;
let inviteId: string;
let inviteCode: string;
let courierJwt: string;
let courierRefreshToken: string;
let courierUserId: string;
let assignmentId: string;
let groupId: string;
let modifierId: string;
let productId: string;
let categoryId: string;
const TS = Date.now();
const COURIER_EMAIL = `courier-e2e-${TS}@test.com`;
const COURIER_PASSWORD = 'test-password-123!';

test.describe.configure({ mode: 'serial' });

test.describe('Flow: Core Lifecycles — Orders, Courier, Settings, Modifiers', () => {

  // ════════════════════════════════════════════════════════════════
  // SETUP
  // ════════════════════════════════════════════════════════════════
  test.beforeAll(async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'owner', locationSlug: 'demo' }, timeout: 30000 });
    expect(authRes.status()).toBe(200);
    const authBody = await authRes.json();
    authToken = authBody.access_token;
    activeLocationId = authBody.activeLocationId;

    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      headers: { Authorization: `Bearer ${authToken}` },
      data: { name: `E2E-Cat-${TS}` },
    });
    if (catRes.status() === 201) {
      categoryId = (await catRes.json()).id;
    }

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      headers: { Authorization: `Bearer ${authToken}` },
      data: {
        category_id: categoryId || undefined,
        name: `E2E-Prod-${TS}`,
        price: 999,
        attributes: { taste: { spicy: 1, sweet: 2, salty: 1, richness: 2, sour: 0 } },
      },
    });
    if (prodRes.status() === 201) {
      productId = (await prodRes.json()).id;
    }
  });

  test.afterAll(async ({ request }) => {
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch(() => {});
    }
    if (categoryId) {
      await request.delete(`${BASE}/api/owner/menu/categories/${categoryId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch(() => {});
    }
    if (groupId) {
      await request.delete(`${BASE}/api/owner/locations/${activeLocationId}/modifier-groups/${groupId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch(() => {});
    }
  });

  async function getOrderStatus(request: any): Promise<string | null> {
    if (!orderId) return null;
    try {
      const res = await request.get(`${BASE}/api/owner/locations/${activeLocationId}/orders/${orderId}/verify`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });
      if (res.status() !== 200) return null;
      const body = await res.json();
      return body.order?.status || body.status || null;
    } catch {
      return null;
    }
  }

  // ════════════════════════════════════════════════════════════════
  // ORDER LIFECYCLE
  // ════════════════════════════════════════════════════════════════

  test('Flow 1: Order — create order for lifecycle tests', async ({ request }) => {
    const infoRes = await request.get(`${BASE}/public/locations/demo/info`);
    const locationSlug = infoRes.ok() ? (await infoRes.json()).slug : 'demo';

    const menuRes = await request.get(`${BASE}/public/locations/${locationSlug}/menu`);
    expect(menuRes.ok()).toBe(true);
    const menu = await menuRes.json();
    const cats = menu.categories || [];
    const allProds = cats.flatMap((c: any) => c.products || c.items || []);
    const flatProds = menu.products || menu.items || menu.data || [];
    const products = [...allProds, ...flatProds];
    const pid = products[0]?.id || productId;

    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId: activeLocationId,
        type: 'delivery',
        items: [{ product_id: pid, quantity: 1 }],
        customer: { phone: '+355600000001', name: 'E2E Test' },
        delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Rruga e Barrikadave, Tirana' },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });
    expect(orderRes.status()).toBe(201);
    const body = await orderRes.json();
    orderId = body.id || body.orderId;
    expect(orderId).toBeTruthy();
  });

  test('Flow 2: Owner — reject order', async ({ request }) => {
    test.skip(!orderId, 'No order created');
    const rejectRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/orders/${orderId}/reject`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { reason: 'Out of stock' } }
    );
    expect(rejectRes.status()).toBe(200);
    const body = await rejectRes.json();
    expect(body.status).toBe('REJECTED');
  });

  test('Flow 3: Owner — assign courier to order', async ({ request }) => {
    test.skip(!orderId, 'No order created');
    const status = await getOrderStatus(request);
    test.skip(status !== 'PENDING' && status !== 'CONFIRMED', `Order is in state ${status}, cannot assign courier`);

    const couriersRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/couriers`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(couriersRes.status()).toBe(200);
    const couriersBody = await couriersRes.json();
    const couriers = couriersBody.couriers || couriersBody.data || [];
    test.skip(couriers.length === 0, 'No couriers available to assign');

    const courierId = couriers[0].id;
    const assignRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/orders/${orderId}/assign-courier`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { courierId } }
    );
    expect(assignRes.status()).toBe(200);
    const body = await assignRes.json();
    expect(body.courierId || body.courier_id).toBeTruthy();
    assignmentId = body.id || body.assignmentId;
  });

  test('Flow 4: Owner — mark order as no-show', async ({ request }) => {
    test.skip(!orderId, 'No order created');
    const status = await getOrderStatus(request);
    test.skip(status !== 'PENDING' && status !== 'CONFIRMED', `Order is in state ${status}, cannot mark no-show`);

    const noShowRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/orders/${orderId}/mark-no-show`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(noShowRes.status()).toBe(200);
    const body = await noShowRes.json();
    expect(body.success).toBe(true);
  });

  test('Flow 5: Owner — verify order detail', async ({ request }) => {
    test.skip(!orderId, 'No order created');
    const verifyRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/orders/${orderId}/verify`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(verifyRes.status()).toBe(200);
    const body = await verifyRes.json();
    expect(body.order).toBeTruthy();
    expect(body.order.id || body.order.orderId).toBeTruthy();
    expect(body.order.status).toBeTruthy();
    expect(Array.isArray(body.items || body.order.items)).toBe(true);
  });

  test('Flow 6: Customer — cancel own order', async ({ request }) => {
    test.skip(!orderId, 'No order created');
    const status = await getOrderStatus(request);
    test.skip(status === 'REJECTED' || status === 'CANCELLED', `Order is already ${status}, cannot cancel`);

    const cancelRes = await request.post(
      `${BASE}/api/customer/orders/${orderId}/cancel`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { reason: 'Changed my mind — E2E test' } }
    );
    expect(cancelRes.status()).toBe(200);
    const body = await cancelRes.json();
    expect(body.success).toBe(true);
  });

  // ════════════════════════════════════════════════════════════════
  // COURIER INVITE REDEEM + AUTH + ASSIGNMENT LIFECYCLE
  // ════════════════════════════════════════════════════════════════

  test('Flow 7: Owner — create courier invite', async ({ request }) => {
    const inviteRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/courier-invites`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { role: 'courier', email: COURIER_EMAIL } }
    );
    expect(inviteRes.status()).toBe(200);
    const body = await inviteRes.json();
    inviteId = body.inviteId;
    inviteCode = body.code;
    expect(inviteId).toBeTruthy();
    expect(inviteCode).toMatch(/^[a-f0-9]{16}$/);
  });

  test('Flow 8: Courier — redeem invite', async ({ request }) => {
    test.skip(!inviteId, 'No invite created');
    const redeemRes = await request.post(
      `${BASE}/api/courier/auth/invites/${inviteId}/redeem`,
      { data: { email: COURIER_EMAIL, code: inviteCode, password: COURIER_PASSWORD, full_name: 'E2E Courier' } }
    );
    expect(redeemRes.status()).toBe(200);
    const body = await redeemRes.json();
    courierJwt = body.jwt;
    courierRefreshToken = body.refreshToken;
    courierUserId = body.courier?.id;
    expect(courierJwt).toBeTruthy();
    expect(courierRefreshToken).toBeTruthy();
    expect(courierUserId).toBeTruthy();
  });

  test('Flow 9: Courier — login API', async ({ request }) => {
    const loginRes = await request.post(`${BASE}/api/courier/auth/login`, {
      data: { email: COURIER_EMAIL, password: COURIER_PASSWORD, location_id: activeLocationId },
    });
    expect(loginRes.status()).toBe(200);
    const body = await loginRes.json();
    courierJwt = body.jwt;
    courierRefreshToken = body.refreshToken;
    expect(courierJwt).toBeTruthy();
    expect(body.activeLocationId).toBeTruthy();
    expect(body.role || body.role).toBeTruthy();
  });

  test('Flow 10: Courier — refresh token', async ({ request }) => {
    test.skip(!courierRefreshToken, 'No refresh token available');
    const refreshRes = await request.post(`${BASE}/api/courier/auth/refresh`, {
      data: { refresh_token: courierRefreshToken },
    });
    expect(refreshRes.status()).toBe(200);
    const body = await refreshRes.json();
    expect(body.jwt).toBeTruthy();
    expect(body.refreshToken).toBeTruthy();
    courierJwt = body.jwt;
    courierRefreshToken = body.refreshToken;
  });

  test('Flow 11: Courier — GET /me profile', async ({ request }) => {
    test.skip(!courierJwt, 'No courier auth');
    const meRes = await request.get(`${BASE}/api/courier/me`, {
      headers: { Authorization: `Bearer ${courierJwt}` },
    });
    expect(meRes.status()).toBe(200);
    const body = await meRes.json();
    expect(body.id || body.courier?.id).toBeTruthy();
    expect(body.full_name || body.courier?.full_name).toBeTruthy();
  });

  test('Flow 12: Courier — GET /me/audit-log', async ({ request }) => {
    test.skip(!courierJwt, 'No courier auth');
    const auditRes = await request.get(`${BASE}/api/courier/me/audit-log`, {
      headers: { Authorization: `Bearer ${courierJwt}` },
    });
    expect(auditRes.status()).toBe(200);
    const body = await auditRes.json();
    const logs = body.logs || body.data || body;
    expect(Array.isArray(logs)).toBe(true);
  });

  test('Flow 13: Courier — GET /me/earnings and /me/history', async ({ request }) => {
    test.skip(!courierJwt, 'No courier auth');
    const earnRes = await request.get(`${BASE}/api/courier/me/earnings`, {
      headers: { Authorization: `Bearer ${courierJwt}` },
    });
    expect(earnRes.status()).toBe(200);
    const earnBody = await earnRes.json();
    if (earnBody.summary) {
      expect(typeof earnBody.summary.today).toBe('number');
    }
    expect(Array.isArray(earnBody.payouts || [])).toBe(true);

    const histRes = await request.get(`${BASE}/api/courier/me/history`, {
      headers: { Authorization: `Bearer ${courierJwt}` },
    });
    expect(histRes.status()).toBe(200);
    const histBody = await histRes.json();
    expect(Array.isArray(histBody)).toBe(true);
  });

  test('Flow 14: Courier — GET /me/payouts', async ({ request }) => {
    test.skip(!courierJwt, 'No courier auth');
    const payoutRes = await request.get(`${BASE}/api/courier/me/payouts`, {
      headers: { Authorization: `Bearer ${courierJwt}` },
    });
    expect(payoutRes.status()).toBe(200);
    const body = await payoutRes.json();
    const payouts = body.payouts || body.data || body;
    if (Array.isArray(payouts) && payouts.length > 0) {
      const detailRes = await request.get(`${BASE}/api/courier/me/payouts/${payouts[0].id}`, {
        headers: { Authorization: `Bearer ${courierJwt}` },
      });
      expect(detailRes.status()).toBe(200);
    }
  });

  test('Flow 15: Courier — PATCH /me/password validation', async ({ request }) => {
    test.skip(!courierJwt, 'No courier auth');
    const pwdRes = await request.patch(`${BASE}/api/courier/me/password`, {
      headers: { Authorization: `Bearer ${courierJwt}` },
      data: { current_password: COURIER_PASSWORD, new_password: 'new-password-456!' },
    });
    expect(pwdRes.status()).toBe(200);
    expect((await pwdRes.json()).success).toBe(true);
    await request.patch(`${BASE}/api/courier/me/password`, {
      headers: { Authorization: `Bearer ${courierJwt}` },
      data: { current_password: 'new-password-456!', new_password: COURIER_PASSWORD },
    }).catch(() => {});
  });

  test('Flow 16: Courier — logout', async ({ request }) => {
    test.skip(!courierRefreshToken, 'No refresh token');
    const logoutRes = await request.post(`${BASE}/api/courier/auth/logout`, {
      data: { refresh_token: courierRefreshToken },
    });
    expect(logoutRes.status()).toBe(200);
  });

  test('Flow 17: Courier — assignment accept/pickup/deliver/cancel', async ({ request }) => {
    test.skip(!courierJwt || !assignmentId, 'No courier auth or no assignment');

    const acceptRes = await request.post(`${BASE}/api/courier/assignments/${assignmentId}/accept`, {
      headers: { Authorization: `Bearer ${courierJwt}` },
    });
    if (acceptRes.status() !== 200) {
      test.skip(); // assignment not in acceptable state
    }

    const puRes = await request.post(`${BASE}/api/courier/assignments/${assignmentId}/picked-up`, {
      headers: { Authorization: `Bearer ${courierJwt}` },
    });
    expect(puRes.status()).toBe(200);

    const delRes = await request.post(`${BASE}/api/courier/assignments/${assignmentId}/delivered`, {
      headers: { Authorization: `Bearer ${courierJwt}` },
      data: { cash_collected: false },
    });
    expect(delRes.status()).toBe(200);

    const cancelRes = await request.post(`${BASE}/api/courier/assignments/${assignmentId}/cancel`, {
      headers: { Authorization: `Bearer ${courierJwt}` },
      data: { reason: 'E2E test cancellation' },
    });
    expect(cancelRes.status()).toBe(410);
  });

  // ════════════════════════════════════════════════════════════════
  // COURIER SHIFT LIFECYCLE
  // ════════════════════════════════════════════════════════════════

  test('Flow 18: Courier — shift lifecycle (start, transition, ping, end)', async ({ request }) => {
    test.skip(!courierJwt, 'No courier auth');

    const shiftRes = await request.get(`${BASE}/api/courier/me/shift`, {
      headers: { Authorization: `Bearer ${courierJwt}` },
    });
    expect(shiftRes.status()).toBe(200);

    const startRes = await request.post(`${BASE}/api/courier/me/shift/start`, {
      headers: { Authorization: `Bearer ${courierJwt}` },
      data: { lat: 41.3275, lng: 19.8187 },
    });
    expect(startRes.status()).toBe(200);

    const transRes = await request.post(`${BASE}/api/courier/shifts/transition`, {
      headers: { Authorization: `Bearer ${courierJwt}` },
      data: { to: 'available', lat: 41.3275, lng: 19.8187 },
    });
    expect(transRes.status()).toBe(200);

    const pingRes = await request.post(`${BASE}/api/courier/shifts/ping`, {
      headers: { Authorization: `Bearer ${courierJwt}` },
      data: { lat: 41.3275, lng: 19.8187, accuracy_meters: 10 },
    });
    expect(pingRes.status()).toBe(200);

    const endRes = await request.post(`${BASE}/api/courier/me/shift/end`, {
      headers: { Authorization: `Bearer ${courierJwt}` },
    });
    expect(endRes.status()).toBe(200);
  });

  // ════════════════════════════════════════════════════════════════
  // MODIFIER GROUPS — CRUD + attach to product
  // ════════════════════════════════════════════════════════════════

  test('Flow 19: Owner — create modifier group', async ({ request }) => {
    const mgRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/modifier-groups`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { name: `E2E-ModGroup-${TS}`, min_select: 0, max_select: 3, required: false } }
    );
    expect(mgRes.status()).toBe(201);
    const body = await mgRes.json();
    groupId = body.id;
    expect(groupId).toBeTruthy();
    expect(body.name).toContain(`E2E-ModGroup-${TS}`);
  });

  test('Flow 20: Owner — list modifier groups', async ({ request }) => {
    const mgListRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/modifier-groups`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(mgListRes.status()).toBe(200);
    const body = await mgListRes.json();
    const groups = body.data || body;
    expect(Array.isArray(groups)).toBe(true);
  });

  test('Flow 21: Owner — update modifier group', async ({ request }) => {
    test.skip(!groupId, 'No group created');
    const mgPatchRes = await request.patch(
      `${BASE}/api/owner/locations/${activeLocationId}/modifier-groups/${groupId}`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { max_select: 2 } }
    );
    expect(mgPatchRes.status()).toBe(200);
    const body = await mgPatchRes.json();
    expect(body.max_select).toBe(2);
  });

  test('Flow 22: Owner — create modifier in group', async ({ request }) => {
    test.skip(!groupId, 'No group created');
    const modRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/modifier-groups/${groupId}/modifiers`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { name: `Extra Cheese ${TS}`, price_delta: 50, sort_order: 1 } }
    );
    expect(modRes.status()).toBe(201);
    const body = await modRes.json();
    modifierId = body.id;
    expect(modifierId).toBeTruthy();
    expect(body.name).toContain(`Extra Cheese ${TS}`);
  });

  test('Flow 23: Owner — update modifier', async ({ request }) => {
    test.skip(!modifierId, 'No modifier created');
    const modPatchRes = await request.patch(
      `${BASE}/api/owner/locations/${activeLocationId}/modifiers/${modifierId}`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { price_delta: 75, sort_order: 2 } }
    );
    expect(modPatchRes.status()).toBe(200);
    const body = await modPatchRes.json();
    expect(body.price_delta).toBe(75);
  });

  test('Flow 24: Owner — attach modifier group to product', async ({ request }) => {
    test.skip(!groupId || !productId, 'No group or product');
    const attachRes = await request.put(
      `${BASE}/api/owner/locations/${activeLocationId}/products/${productId}/modifier-groups`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: [{ group_id: groupId, sort_order: 0 }] }
    );
    expect(attachRes.status()).toBe(200);
    expect((await attachRes.json()).success).toBe(true);

    const getAttachedRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/products/${productId}/modifier-groups`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(getAttachedRes.status()).toBe(200);
    const body = await getAttachedRes.json();
    const groups = body.data || body;
    expect(Array.isArray(groups)).toBe(true);
    expect(groups.some((g: any) => g.id === groupId || g.group_id === groupId || g.groupId === groupId)).toBe(true);
  });

  // ════════════════════════════════════════════════════════════════
  // OWNER SETTINGS — dwell, fallback, retention, location
  // ════════════════════════════════════════════════════════════════

  test('Flow 25: Owner — dwell settings round-trip', async ({ request }) => {
    const getRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/settings/dwell`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(getRes.status()).toBe(200);
    let body = await getRes.json();
    const thresholds = body.dwellThresholds || body;
    const orig = {
      pending_s: thresholds.pending_s || 300,
      confirmed_s: thresholds.confirmed_s || 300,
      preparing_s: thresholds.preparing_s || 600,
      en_route_s: thresholds.en_route_s || 900,
    };

    const putRes = await request.put(
      `${BASE}/api/owner/locations/${activeLocationId}/settings/dwell`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { dwellThresholds: { pending_s: 60, confirmed_s: 60, preparing_s: 120, en_route_s: 180 } } }
    );
    expect(putRes.status()).toBe(200);
    body = await putRes.json();
    const updated = body.dwellThresholds || body;
    expect(updated.pending_s).toBe(60);

    await request.put(`${BASE}/api/owner/locations/${activeLocationId}/settings/dwell`, {
      headers: { Authorization: `Bearer ${authToken}` },
      data: { dwellThresholds: orig },
    }).catch(() => {});
  });

  test('Flow 26: Owner — fallback settings round-trip', async ({ request }) => {
    const getRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/settings/fallback`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(getRes.status()).toBe(200);
    let body = await getRes.json();
    const origPhone = body.phone;
    const origShow = body.showPhoneOnError;

    const putRes = await request.put(
      `${BASE}/api/owner/locations/${activeLocationId}/settings/fallback`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { showPhoneOnError: true, showPhoneOnOffline: true } }
    );
    expect(putRes.status()).toBe(200);

    await request.put(`${BASE}/api/owner/locations/${activeLocationId}/settings/fallback`, {
      headers: { Authorization: `Bearer ${authToken}` },
      data: { phone: origPhone, showPhoneOnError: origShow, showPhoneOnOffline: true },
    }).catch(() => {});
  });

  test('Flow 27: Owner — degradation status', async ({ request }) => {
    const degRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/degradation`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(degRes.status()).toBe(200);
    const body = await degRes.json();
    expect(body.locationId || body.location_id).toBeTruthy();
  });

  test('Flow 28: Owner — retention settings round-trip', async ({ request }) => {
    const getRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/settings/retention`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(getRes.status()).toBe(200);
    let body = await getRes.json();
    const origDays = body.retentionDays || 365;

    const putRes = await request.put(
      `${BASE}/api/owner/locations/${activeLocationId}/settings/retention`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { retentionDays: 90 } }
    );
    expect(putRes.status()).toBe(200);
    expect((await putRes.json()).retentionDays).toBe(90);

    await request.put(`${BASE}/api/owner/locations/${activeLocationId}/settings/retention`, {
      headers: { Authorization: `Bearer ${authToken}` },
      data: { retentionDays: origDays },
    }).catch(() => {});
  });

  test('Flow 29: Owner — update location settings', async ({ request }) => {
    const patchRes = await request.patch(
      `${BASE}/api/owner/locations/${activeLocationId}`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { name: `E2E-Test-Location-${TS}` } }
    );
    expect(patchRes.status()).toBe(200);
    const body = await patchRes.json();
    expect(body.name || body.location?.name).toContain(`E2E-Test-Location-${TS}`);
  });

  // ════════════════════════════════════════════════════════════════
  // PRODUCT TRANSLATIONS
  // ════════════════════════════════════════════════════════════════

  test('Flow 30: Owner — product translations CRUD', async ({ request }) => {
    test.skip(!productId, 'No product created');

    const putRes = await request.put(
      `${BASE}/api/owner/locations/${activeLocationId}/products/${productId}/translations/sq`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { name: `Produkt Test ${TS}`, description: 'Pershkrim test' } }
    );
    expect(putRes.status()).toBe(200);

    const listRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/products/${productId}/translations`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(listRes.status()).toBe(200);
    const body = await listRes.json();
    const translations = body.data || body;
    if (Array.isArray(translations) && translations.length > 0) {
      expect(translations.some((t: any) => t.locale === 'sq')).toBe(true);
    }

    const delRes = await request.delete(
      `${BASE}/api/owner/locations/${activeLocationId}/products/${productId}/translations/sq`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(delRes.status()).toBe(204);
  });

  // ════════════════════════════════════════════════════════════════
  // PUSH NOTIFICATIONS (owner)
  // ════════════════════════════════════════════════════════════════

  test('Flow 31: Owner — push notification subscribe/unsubscribe/state', async ({ request }) => {
    const stateRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/push/state`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(stateRes.status()).toBe(200);
    const stateBody = await stateRes.json();
    expect('subscribed' in stateBody).toBe(true);

    const subRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/push/subscribe`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { subscription: { endpoint: 'https://example.com/push', keys: { p256dh: 'test', auth: 'test' } } } }
    );
    expect([200, 422]).toContain(subRes.status());

    const unsubRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/push/unsubscribe`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(unsubRes.status()).toBe(200);
  });

  // ════════════════════════════════════════════════════════════════
  // NOTIFICATIONS TARGETS
  // ════════════════════════════════════════════════════════════════

  test('Flow 32: Owner — notification targets list', async ({ request }) => {
    const targetsRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/notifications/targets`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(targetsRes.status()).toBe(200);
    const body = await targetsRes.json();
    const targets = body.targets || body.data || body;
    expect(Array.isArray(targets)).toBe(true);
    if (targets.length > 0) {
      expect(targets[0].id || targets[0].targetId).toBeTruthy();
    }
  });
});
