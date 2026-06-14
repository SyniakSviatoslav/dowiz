import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe('WS courier assignment notification (bugfix: wrong channel + wrapping)', () => {

  test('courier receives task_assigned via WS after owner assigns to order', async ({ page, request }) => {
    // 1. Auth as owner
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, {
      data: { role: 'owner', locationSlug: 'demo' },
      timeout: 30000,
    });
    expect(authRes.status()).toBe(200);
    const authBody = await authRes.json();
    const ownerToken = authBody.access_token;
    const locationId = authBody.activeLocationId;
    expect(ownerToken).toBeTruthy();
    expect(locationId).toBeTruthy();

    // 2. Fetch menu for a product
    const menuRes = await request.get(`${BASE}/public/locations/demo/menu`, {
      headers: { 'Cache-Control': 'no-cache' },
    });
    expect(menuRes.ok()).toBe(true);
    const menu = await menuRes.json();
    const cats = menu.categories || [];
    const allProds = cats.flatMap((c: any) => c.products || c.items || []);
    const topLevelProds = menu.products || menu.items || menu.data || [];
    const products = [...allProds, ...topLevelProds];
    test.skip(products.length === 0, 'No products in demo menu');
    const productId = products[0].id;

    // 3. Create order
    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1 }],
        customer: { phone: '+355600000001', name: 'E2E WS Test' },
        delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Rruga e Barrikadave, Tirana' },
        payment: { method: 'cash' },
        idempotency_key: `ws-test-${Date.now()}`,
      },
    });
    const orderData = await orderRes.json();
    console.log('Order create:', orderRes.status(), JSON.stringify(orderData));
    expect(orderRes.status()).toBe(201);
    const orderId = orderData.id || orderData.orderId;
    expect(orderId).toBeTruthy();

    // 4. Confirm the order (so it can be assigned)
    const confirmRes = await request.post(
      `${BASE}/api/owner/locations/${locationId}/orders/${orderId}/confirm`,
      { headers: { Authorization: `Bearer ${ownerToken}` } }
    );
    expect([200, 409]).toContain(confirmRes.status());

    // 5. Create courier via mock-auth
    const courierAuthRes = await request.post(`${BASE}/api/dev/mock-auth`, {
      data: { role: 'courier', locationId },
    });
    expect(courierAuthRes.status()).toBe(200);
    const courierBody = await courierAuthRes.json();
    const courierToken = courierBody.access_token;
    const courierId = courierBody.userId;
    expect(courierToken).toBeTruthy();
    expect(courierId).toBeTruthy();

    // 6. Open WebSocket as courier and subscribe to personal channel
    const wsMessages = await page.evaluate(async ({ token, cId, base }) => {
      const ws = new WebSocket(`${base.startsWith('https') ? 'wss' : 'ws'}://${new URL(base).host}/ws?token=${token}`);

      return new Promise<string[]>((resolve) => {
        const messages: string[] = [];
        const timeout = setTimeout(() => {
          ws.close();
          resolve([...messages, 'timeout']);
        }, 15000);

        ws.onopen = () => messages.push('open');

        ws.onmessage = (e) => {
          try {
            const data = JSON.parse(e.data);
            messages.push(`msg:${data.type}`);

            if (data.type === 'auth_success') {
              ws.send(JSON.stringify({ type: 'subscribe', room: `courier:${cId}` }));
            }

            if (data.type === 'task_assigned') {
              messages.push(`payload:${data.payload?.orderId}`);
              clearTimeout(timeout);
              ws.close();
              resolve(messages);
            }
          } catch {
            messages.push('parse_error');
          }
        };

        ws.onerror = () => messages.push('error');
        ws.onclose = (e) => {
          messages.push(`close:${e.code}`);
          clearTimeout(timeout);
          resolve(messages);
        };
      });
    }, { token: courierToken, cId: courierId, base: BASE });

    // Verify WS connection
    expect(wsMessages).toContain('open');
    expect(wsMessages).toContain('msg:auth_success');

    // If we haven't received task_assigned yet (test might be fast), wait for it
    // by assigning courier now and re-checking (the WS is still open in evaluate)
    // Actually the evaluate above already started — we need to assign AFTER WS is ready
    // Let's use a simpler approach: do it in two page.evaluate phases

    // Phase 2: Open a new WS and assign courier while listening
    const wsResult = await page.evaluate(async ({ token, cId, oId, locId, ownerTok, base }) => {
      const ws = new WebSocket(`${base.startsWith('https') ? 'wss' : 'ws'}://${new URL(base).host}/ws?token=${token}`);

      return new Promise<string[]>((resolve) => {
        const messages: string[] = [];
        let subscribed = false;
        const timeout = setTimeout(() => {
          ws.close();
          resolve([...messages, 'timeout']);
        }, 20000);

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
              // Now trigger the assignment via owner API
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
              messages.push(`task_ok:${data.payload?.orderId}`);
              clearTimeout(timeout);
              ws.close();
              resolve(messages);
            }
          } catch {
            messages.push('parse_error');
          }
        };

        ws.onerror = () => messages.push('error');
        ws.onclose = (e) => {
          messages.push(`close:${e.code}`);
          clearTimeout(timeout);
          resolve(messages);
        };
      });
    }, {
      token: courierToken,
      cId: courierId,
      oId: orderId,
      locId: locationId,
      ownerTok: ownerToken,
      base: BASE,
    });

    // 7. Assertions — verify the WS received the task_assigned event
    console.log(`WS courier test result: [${wsResult.join(', ')}]`);

    expect(wsResult).toContain('msg:auth_success');
    expect(wsResult).toContain('subscribed_ok');
    expect(wsResult).toContain('msg:task_assigned');
    expect(wsResult.some(m => m.startsWith('task_ok:'))).toBe(true);
    expect(wsResult).not.toContain('timeout');
    expect(wsResult).not.toContain('assign_error');
  });
});
