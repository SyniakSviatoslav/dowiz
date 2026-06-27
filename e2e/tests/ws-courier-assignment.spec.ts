import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

test.describe('WS courier assignment notification (bugfix: wrong channel + wrapping)', () => {
  let ownerToken: string;
  let locationId: string;
  let productId: string;
  let orderId: string;
  let courierToken: string;
  let courierId: string;

  test.beforeAll(async ({ request }) => {
    // This spec MUTATES state (creates products/orders/assignments) — never let it touch prod.
    requireStaging(BASE);

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
      data: { name: `WS2-Cat-${Date.now()}` },
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    expect(catRes.status()).toBe(201);
    const categoryId = (await catRes.json()).id;

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: `WS2-Prod-${Date.now()}`, price: 500, available: true, categoryId, stockCount: 10 },
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    expect(prodRes.status()).toBe(201);
    productId = (await prodRes.json()).id;

    // Create courier JWT via mock-auth
    const courierAuth = await request.post(`${BASE}/api/dev/mock-auth`, {
      data: { role: 'courier', locationId },
    });
    expect(courierAuth.status()).toBe(200);
    const courierBody = await courierAuth.json();
    courierToken = courierBody.access_token;
    // The mock-auth returns a random UUID userId, but the actual
    // assign-courier endpoint needs the courier to exist in DB.
    // We'll use dev/create-assignment which handles DB records.
    courierId = courierBody.userId;
    expectJwt(courierToken, 'courierToken');
    expectUuid(courierId, 'courierId');
  });

  test('courier receives task_assigned via WS after dev creates assignment', async ({ page, request }) => {
    test.skip(!ownerToken, 'Setup failed');

    // Create order via API
    const orderRes = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId,
        type: 'delivery',
        items: [{ product_id: productId, quantity: 1 }],
        customer: { phone: `+3556000${String(Date.now()).slice(-6)}`, name: 'WS Test' },
        delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Test Address, Tirana' },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });
    expect(orderRes.status()).toBe(201);
    orderId = (await orderRes.json()).id;
    console.log('Order created:', orderId);

    // Confirm the order
    const confirmRes = await request.post(
      `${BASE}/api/owner/locations/${locationId}/orders/${orderId}/confirm`,
      { headers: { Authorization: `Bearer ${ownerToken}` } }
    );
    // owner confirm route returns exactly 200 (apps/api/src/routes/owner/dashboard.ts:196)
    expect(confirmRes.status()).toBe(200);

    // Navigate to admin to set origin for WS
    await page.goto(`${BASE}/admin`, { waitUntil: 'domcontentloaded', timeout: 15000 });

    // Open WS as courier + create assignment via dev endpoint + verify task_assigned
    const wsResult = await page.evaluate(async ({ token, cId, oId, locId, base }) => {
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
                const assignRes = await fetch(`${base}/api/dev/create-assignment`, {
                  method: 'POST',
                  headers: { 'Content-Type': 'application/json' },
                  body: JSON.stringify({ orderId: oId, courierId: cId, locationId: locId }),
                });
                messages.push(`assign_status:${assignRes.status}`);
                const assignBody = await assignRes.json();
                messages.push(`assign_body:${JSON.stringify(assignBody).slice(0, 120)}`);
              } catch (err: any) {
                messages.push(`assign_error:${err.message}`);
              }
            }

            // Accept ONLY a relayed message scoped to OUR courier room whose inner
            // type is task_assigned — a bare top-level `type: 'task_assigned'`
            // (broadcast/ping/debug) or a message addressed to another room must NOT pass.
            const relayed = data.room === `courier:${cId}` && data.data?.type === 'task_assigned';
            if (relayed) {
              // Validate the payload actually references THIS order + THIS courier,
              // not just any task_assigned that happened to arrive.
              const p = data.data?.payload ?? {};
              const orderOk = (p.orderId ?? p.id) === oId;
              const courierOk = p.courierId === cId;
              messages.push(`payload_order:${orderOk}`);
              messages.push(`payload_courier:${courierOk}`);
              if (orderOk && courierOk) {
                messages.push(`task_ok`);
                clearTimeout(timeout);
                ws.close();
                resolve(messages);
              }
            }
          } catch {
            messages.push('parse_error');
          }
        };

        ws.onerror = () => messages.push('error');
        ws.onclose = (e) => { messages.push(`close:${e.code}`); clearTimeout(timeout); resolve(messages); };
      });
    }, { token: courierToken, cId: courierId, oId: orderId, locId: locationId, base: BASE });

    console.log(`WS result: [${wsResult.join(', ')}]`);

    expect(wsResult).toContain('msg:auth_success');
    expect(wsResult).toContain('subscribed_ok');
    expect(wsResult).toContain('payload_order:true');
    expect(wsResult).toContain('payload_courier:true');
    expect(wsResult).toContain('task_ok');
    expect(wsResult).not.toContain('timeout');
  });

  // TODO(needs_staging): cross-tenant / IDOR negative control. A courier authenticated as
  // courier-A must NOT receive a task_assigned addressed to `courier:<courierB_id>`. This
  // requires a REAL second courier id in a second tenant (a nil/all-zero id proves nothing —
  // it would 404 by absence). Implement against live staging with a seeded second courier:
  // authenticate as A, subscribe to `courier:<B_id>`, create an assignment for B, and assert
  // A receives ZERO task_assigned for B (with expect(ws-opened)===true before the zero claim).
});
