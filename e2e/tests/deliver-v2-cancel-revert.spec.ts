import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';

// deliver v2 — RESOLVE round 5, D1 (the C-2 red-line) E2E.
// Proves the SHIPPED HTTP wiring of the /cancel fix end-to-end against deployed staging: a courier
// cancelling (within the 5-min accept-regret window) an order the owner force-assigned to IN_DELIVERY must
// leave the order back at READY — NOT stranded IN_DELIVERY (the C-2 trap) and NOT falsely CANCELLED (R2-5).
// The unit suite (apps/api/tests/deliver-drift-resolve5.test.ts) proves the rail; this proves the handler
// actually rides it over HTTP.
//
// Run:  VITE_BASE_URL=https://dowiz-staging.fly.dev DEV_AUTH_SECRET=<stg-secret> \
//         pnpm exec playwright test deliver-v2-cancel-revert --reporter=list
// (playwright.config.ts injects baseURL + the x-dev-auth-secret header; the `request` fixture uses both.)
//
// Staging setup assumptions (validated loudly on first run — a failure here points at config, not the fix):
//   • ALLOW_DEV_LOGIN=true + DEV_AUTH_SECRET set on staging (the /api/dev/* guard).
//   • The default dev owner from /api/dev/mock-auth owns a location with ≥1 published product, and a
//     /api/dev/mock-auth courier (role:'courier', same locationId) is provisioned active + in courier_locations
//     (required by assign-courier's courierCheck: couriers.status='active' AND a courier_locations row).
const SLUG = process.env.E2E_LOCATION_SLUG || 'demo';
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

type Mock = { access_token: string; userId: string; activeLocationId: string };

async function mockAuth(request: any, data: Record<string, unknown>): Promise<Mock> {
  const res = await request.post('/api/dev/mock-auth', { data });
  expect(res.ok(), `mock-auth ${JSON.stringify(data)} failed: ${res.status()} ${await res.text()}`).toBeTruthy();
  return res.json();
}

async function snapshotStatus(request: any, locationId: string, auth: Record<string, string>, orderId: string): Promise<string | undefined> {
  const res = await request.get(`/api/owner/locations/${locationId}/dashboard/snapshot`, { headers: auth });
  expect(res.ok(), `snapshot failed: ${res.status()} ${await res.text()}`).toBeTruthy();
  const body = await res.json();
  const orders = body.orders ?? body.data?.orders ?? [];
  return orders.find((o: any) => (o.id ?? o.orderId) === orderId)?.status;
}

test('D1 · courier /cancel of an owner-forced IN_DELIVERY order reverts it to READY (no C-2 trap, no false cancel)', async ({ request }) => {
  // ── Auth ─────────────────────────────────────────────────────────────────────────────────────────────
  const owner = await mockAuth(request, {});
  const locationId = owner.activeLocationId;
  const ownerAuth = { Authorization: `Bearer ${owner.access_token}` };

  const courier = await mockAuth(request, { role: 'courier', locationId });
  const courierId = courier.userId;
  const courierAuth = { Authorization: `Bearer ${courier.access_token}` };

  // ── A product on the owner's location (public menu) ─────────────────────────────────────────────────
  const menuRes = await request.get(`/public/locations/${SLUG}/menu`);
  expect(menuRes.ok(), `menu fetch for slug '${SLUG}' failed: ${menuRes.status()}`).toBeTruthy();
  const menu = await menuRes.json();
  const menuLocationId = menu.location_id ?? menu.locationId;
  expect(menuLocationId, `slug '${SLUG}' must resolve to the dev owner's location (${locationId}) — set E2E_LOCATION_SLUG`).toBe(locationId);
  const productId = menu.categories?.[0]?.products?.[0]?.id;
  expect(productId, `no product on '${SLUG}' menu — seed the location`).toMatch(UUID);

  // ── Create an order (cash, delivery) ───────────────────────────────────────────────────────────────
  const orderPayload = {
    locationId,
    type: 'delivery',
    items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
    customer: { phone: `+35569${Date.now().toString().slice(-8)}`, name: 'E2E D1' },
    delivery: { pin: { lat: 41.32, lng: 19.45 }, address_text: 'E2E D1 Address' },
    payment: { method: 'cash' },
    cash_pay_with: 5000,
    idempotency_key: crypto.randomUUID(),
    acknowledged_codes: [] as string[],
  };
  const postOrder = (p: typeof orderPayload) => request.post('/api/orders', { headers: { 'Content-Type': 'application/json' }, data: p });
  let orderRes = await postOrder(orderPayload);
  let orderBody = await orderRes.json();
  // Soft-confirm (velocity heuristics under repeated test traffic) → a real client re-submits acknowledging.
  if (orderBody?.outcome === 'soft_confirm' && Array.isArray(orderBody.reasons)) {
    orderRes = await postOrder({ ...orderPayload, acknowledged_codes: orderBody.reasons.map((r: any) => r.code).filter(Boolean) });
    orderBody = await orderRes.json();
  }
  expect(orderRes.ok(), `order create failed: ${orderRes.status()} ${JSON.stringify(orderBody)}`).toBeTruthy();
  const orderId = orderBody.id ?? orderBody.order?.id ?? orderBody.orderId;
  expect(orderId, 'order id missing from create response').toMatch(UUID);

  // ── Owner: confirm (→ CONFIRMED) then force-assign the courier (→ IN_DELIVERY + an 'accepted' binding) ─
  const confirmRes = await request.post(`/api/owner/locations/${locationId}/orders/${orderId}/confirm`, { headers: ownerAuth, data: {} });
  expect(confirmRes.ok(), `confirm failed: ${confirmRes.status()} ${await confirmRes.text()}`).toBeTruthy();

  // Provision the courier INTO this location (couriers.status defaults 'active' + courier_locations + shift)
  // via the dev create-assignment shortcut — assign-courier's courierCheck requires an active courier in the
  // location, and the mock-auth courier is a bare JWT with no DB row. The 'assigned' binding it creates is
  // incidental: the owner assign-courier below terminalizes it and creates the real 'accepted' + IN_DELIVERY
  // binding (the C-2 setup). (x-dev-auth-secret is injected globally by playwright.config.)
  const provRes = await request.post('/api/dev/create-assignment', { data: { orderId, courierId, locationId } });
  expect(provRes.ok(), `courier provisioning (dev create-assignment) failed: ${provRes.status()} ${await provRes.text()}`).toBeTruthy();

  const assignRes = await request.post(`/api/owner/locations/${locationId}/orders/${orderId}/assign-courier`, { headers: ownerAuth, data: { courierId } });
  expect(assignRes.ok(), `assign-courier failed: ${assignRes.status()} ${await assignRes.text()}`).toBeTruthy();
  const assignmentId = (await assignRes.json()).id;
  expect(assignmentId, 'assignment id missing').toMatch(UUID);

  expect(await snapshotStatus(request, locationId, ownerAuth, orderId), 'order should be IN_DELIVERY after force-assign').toBe('IN_DELIVERY');

  // ── Courier: cancel within the 5-min accept-regret window ───────────────────────────────────────────
  const cancelRes = await request.post(`/api/courier/assignments/${assignmentId}/cancel`, { headers: courierAuth, data: { reason: 'e2e-d1' } });
  expect(cancelRes.ok(), `cancel failed (window expired? → deploy ran >5min after assign): ${cancelRes.status()} ${await cancelRes.text()}`).toBeTruthy();
  const cancelBody = await cancelRes.json();
  expect(cancelBody.success).toBe(true);

  // ── THE C-2 FIX: the order is back to assignable, not stranded IN_DELIVERY and not falsely CANCELLED ──
  const finalStatus = await snapshotStatus(request, locationId, ownerAuth, orderId);
  expect(finalStatus, 'C-2 trap: order must revert to READY, never stranded IN_DELIVERY').toBe('READY');
  expect(finalStatus, 'R2-5: a READY revert must not be a false CANCELLED').not.toBe('CANCELLED');
});
