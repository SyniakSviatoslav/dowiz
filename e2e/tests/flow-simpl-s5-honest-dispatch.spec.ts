import { test, expect } from '@playwright/test';
import crypto from 'node:crypto';

// flow-simplification §5 / R2-1 — HONEST DISPATCH (the no-trap red-line).
// When the owner sends an order for delivery (PATCH /orders/:id/status → IN_DELIVERY) and NO courier is
// available, the order must NOT advance to IN_DELIVERY (an IN_DELIVERY order with no courier and no recovery
// affordance is the F1 orphan). It must stay at its current status and report {dispatched:false,reason:'no_courier'}.
// The demo location has no unbound available courier (couriers live in vis-open / carry active bindings), so a
// fresh order's send finds none → the honest no-dispatch path.
const SLUG = process.env.E2E_LOCATION_SLUG || 'demo';
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

test('§5 · owner send-for-delivery with no available courier stays CONFIRMED (no IN_DELIVERY orphan)', async ({ request }) => {
  const owner = await (await request.post('/api/dev/mock-auth', { data: {} })).json();
  const locationId = owner.activeLocationId;
  const auth = { Authorization: `Bearer ${owner.access_token}` };

  // a product + an order
  const menu = await (await request.get(`/public/locations/${SLUG}/menu`)).json();
  expect(menu.location_id ?? menu.locationId, 'demo slug must resolve to the owner location').toBe(locationId);
  const productId = menu.categories?.[0]?.products?.[0]?.id;
  expect(productId).toMatch(UUID);

  const post = (body: any) => request.post('/api/orders', { headers: { 'Content-Type': 'application/json' }, data: body });
  const orderPayload = {
    locationId, type: 'delivery',
    items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
    customer: { phone: `+35569${Date.now().toString().slice(-8)}`, name: 'E2E S5' },
    delivery: { pin: { lat: 41.32, lng: 19.45 }, address_text: 'E2E S5' },
    payment: { method: 'cash' }, idempotency_key: crypto.randomUUID(), acknowledged_codes: [] as string[],
  };
  let res = await post(orderPayload);
  let body = await res.json();
  if (body?.outcome === 'soft_confirm' && Array.isArray(body.reasons)) {
    res = await post({ ...orderPayload, acknowledged_codes: body.reasons.map((r: any) => r.code).filter(Boolean) });
    body = await res.json();
  }
  expect(res.ok(), `order create: ${res.status()} ${JSON.stringify(body)}`).toBeTruthy();
  const orderId = body.id ?? body.order?.id;
  expect(orderId).toMatch(UUID);

  await request.post(`/api/owner/locations/${locationId}/orders/${orderId}/confirm`, { headers: auth, data: {} });

  // Send for delivery with no courier available.
  const sendRes = await request.patch(`/api/orders/${orderId}/status`, { headers: auth, data: { status: 'IN_DELIVERY' } });
  expect(sendRes.ok(), `send: ${sendRes.status()} ${await sendRes.text()}`).toBeTruthy();
  const send = await sendRes.json();

  // THE FIX: not advanced to IN_DELIVERY, honest no_courier signal, order still CONFIRMED.
  expect(send.dispatched, 'no courier → dispatched:false').toBe(false);
  expect(send.reason).toBe('no_courier');
  expect(send.status, 'order must NOT be orphaned IN_DELIVERY').not.toBe('IN_DELIVERY');

  const snap = await (await request.get(`/api/owner/locations/${locationId}/dashboard/snapshot`, { headers: auth })).json();
  const o = (snap.orders ?? []).find((x: any) => (x.id ?? x.orderId) === orderId);
  expect(o?.status, 'order stays CONFIRMED, never a no-courier IN_DELIVERY orphan').toBe('CONFIRMED');
});
