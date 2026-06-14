import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe('WS courier assignment notification (bugfix: wrong channel + wrapping)', () => {
  let ownerToken: string;
  let locationId: string;
  let productId: string;
  let orderId: string;
  let courierToken: string;
  let courierId: string;
  const TS = Date.now();
  const COURIER_EMAIL = `ws-courier-${TS}@test.com`;
  const COURIER_PASSWORD = 'test-password-123!';

  test.beforeAll(async ({ request }) => {
    // Auth as owner
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, {
      data: { role: 'owner', locationSlug: 'demo' },
      timeout: 30000,
    });
    expect(authRes.status()).toBe(200);
    const authBody = await authRes.json();
    ownerToken = authBody.access_token;
    locationId = authBody.activeLocationId;

    // Create product
    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: `WS-Cat-${TS}` },
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    expect(catRes.status()).toBe(201);
    const categoryId = (await catRes.json()).id;

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: `WS-Prod-${TS}`, price: 500, available: true, categoryId, stockCount: 10 },
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    expect(prodRes.status()).toBe(201);
    productId = (await prodRes.json()).id;

    // Create courier via invite + redeem
    const invRes = await request.post(
      `${BASE}/api/owner/locations/${locationId}/courier-invites`,
      { headers: { Authorization: `Bearer ${ownerToken}` }, data: { email: COURIER_EMAIL, role: 'courier' } }
    );
    expect(invRes.status()).toBe(201);
    const inviteId = (await invRes.json()).id;

    const detailRes = await request.get(`${BASE}/api/courier/auth/invites/${inviteId}`);
    const inviteDetail = await detailRes.json();
    const code = inviteDetail.code || inviteDetail.inviteCode;

    await request.post(`${BASE}/api/courier/auth/invites/${inviteId}/redeem`, {
      data: { name: 'WS Courier', email: COURIER_EMAIL, password: COURIER_PASSWORD, code },
    });

    const loginRes = await request.post(`${BASE}/api/courier/auth/login`, {
      data: { email: COURIER_EMAIL, password: COURIER_PASSWORD },
    });
    expect(loginRes.status()).toBe(200);
    const loginBody = await loginRes.json();
    courierToken = loginBody.jwt;
    courierId = loginBody.courier?.id || loginBody.userId;
    expect(courierToken).toBeTruthy();
    expect(courierId).toBeTruthy();

    // Start shift
    const shiftRes = await request.post(`${BASE}/api/courier/me/shift/start`, {
      headers: { Authorization: `Bearer ${courierToken}` },
      data: { lat: 41.33, lng: 19.82 },
    });
    expect(shiftRes.status()).toBe(200);
  });

  test('courier receives task_assigned via WS after owner assigns to order', async ({ page }) => {
    test.skip(!ownerToken, 'Setup failed');

    // Create order via API
    const orderRes = await (await test.info()).request.post(`${BASE}/api/orders`, {
      data: {
        locationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1 }],
        customer: { phone: '+355600000001', name: 'WS Test' },
        delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Test Address, Tirana' },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });
    expect(orderRes.status()).toBe(201);
    orderId = (await orderRes.json()).id;
    console.log('Order created:', orderId);

    // Confirm the order
    const confirmRes = await (await test.info()).request.post(
      `${BASE}/api/owner/locations/${locationId}/orders/${orderId}/confirm`,
      { headers: { Authorization: `Bearer ${ownerToken}` } }
    );
    console.log('Order confirmed:', confirmRes.status());

    // Navigate to admin to set origin
    await page.goto(`${BASE}/admin`, { waitUntil: 'domcontentloaded', timeout: 15000 });

    // Open WS + assign courier + verify task_assigned
    const wsResult = await page.evaluate(async ({ token, cId, oId, locId, ownerTok, base }) => {
      const wsUrl = `${base.startsWith('https') ? 'wss' : 'ws'}://${new URL(base).host}/ws?token=${token}`;
      const ws = new WebSocket(wsUrl);

      return new Promise<string[]>((resolve) => {
        const messages: string[] = [];
        let subscribed = false;
        const timeout = setTimeout(() => { ws.close(); resolve([...messages, 'timeout']); }, 25000);

        ws.onopen = () => messages.push('open');

        ws.onmessage = async (e) => {
          try {
            const data = JSON.parse(e.data);
            messages.push(`msg:${data.type}`);

            if (data.type === 'auth_success') {
              ws.send(JSON.stringify({ type: 'subscribe', room: `courier:${cId}` }));
            }

            if (data.type === 'subscribed' && !subscribed) {
              subscribed = true;
              messages.push('subscribed_ok');
              try {
                const assignRes = await fetch(
                  `${base}/api/owner/locations/${locId}/orders/${oId}/assign-courier`,
                  {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${ownerTok}` },
                    body: JSON.stringify({ courierId: cId }),
                  }
                );
                messages.push(`assign_status:${assignRes.status}`);
                const assignBody = await assignRes.json();
                messages.push(`assign_body:${JSON.stringify(assignBody)}`);
              } catch (err: any) {
                messages.push(`assign_error:${err.message}`);
              }
            }

            if (data.type === 'task_assigned') {
              messages.push(`task_ok:${data.payload?.orderId || data.data?.payload?.orderId}`);
              clearTimeout(timeout);
              ws.close();
              resolve(messages);
            }
          } catch {
            messages.push('parse_error');
          }
        };

        ws.onerror = () => messages.push('error');
        ws.onclose = (e) => { messages.push(`close:${e.code}`); clearTimeout(timeout); resolve(messages); };
      });
    }, { token: courierToken, cId: courierId, oId: orderId, locId: locationId, ownerTok: ownerToken, base: BASE });

    console.log(`WS result: [${wsResult.join(', ')}]`);

    expect(wsResult).toContain('msg:auth_success');
    expect(wsResult).toContain('subscribed_ok');
    expect(wsResult).toContain('msg:task_assigned');
    const taskOk = wsResult.find(m => m.startsWith('task_ok:'));
    expect(taskOk).toBeTruthy();
    expect(wsResult).not.toContain('timeout');
  });
});
