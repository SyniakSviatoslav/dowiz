// Plain Node test proving the kernel produces a REAL order (not the old
// placeholder). Uses the node-target wasm-bindgen glue (web/src/lib/kernel/node/),
// which loads + instantiates the wasm synchronously via CommonJS require — no
// fetch shim needed. The browser (--target web) glue in ../kernel/ is exercised
// by `npm run build`; this test proves the kernel logic end-to-end in Node.
//
// Run: node web/src/lib/kernel/kernel.test.mjs
import { createRequire } from 'node:module';
import assert from 'node:assert/strict';

const require = createRequire(import.meta.url);
const K = require('./node/dowiz_kernel.cjs');

const items = [
  {
    product_id: '11111111-1111-1111-1111-111111111111',
    modifier_ids: ['21111111-1111-1111-1111-111111111111'],
    quantity: 2,
    unit_price: 0,
  },
];

const orderJson = K.place_order_js(undefined, JSON.stringify(items), 'storefront');
const order = JSON.parse(orderJson);

console.log('kernel order:', order);

assert.notEqual(order.id, 'placeholder-order', 'must be a real kernel order id');
assert.ok(order.id && order.id.length > 0, 'order id must be non-empty');
assert.equal(order.status, 'PENDING', 'new order status must be Pending (wire form PENDING)');
assert.equal(order.items.length, 1, 'items folded');

// Prove the state machine: Pending -> Confirmed is legal.
const confirmed = JSON.parse(K.apply_event_js(orderJson, 'CONFIRMED'));
assert.equal(confirmed.status, 'CONFIRMED', 'Pending->Confirmed transition works');
assert.equal(confirmed.id, order.id, 'id preserved across transition');

// Prove illegal transition throws.
let threw = false;
try {
  K.apply_event_js(orderJson, 'DELIVERED');
} catch {
  threw = true;
}
assert.ok(threw, 'illegal transition Pending->Delivered must throw');

console.log('PASS: real kernel order id =', order.id, 'status =', order.status);

// ── Channel attribution + anomaly equivalence (RW-02 gate) ──
// Same SAMPLE_EVENTS as web/src/components/OwnerDashboard.svelte. Proves the
// canonical kernel `channel_ledger_js` produces the identical attribution the
// legacy JS port (web/src/lib/channel.js, now deleted) computed — so the
// frontend rewire is behavior-preserving.
const sample = [
  { order_id: 'o1', channel: 'tiktok', status: 'PENDING', at_ms: 1 },
  { order_id: 'o1', channel: 'instagram', status: 'CONFIRMED', at_ms: 2 }, // dup id -> ignored re-attribute
  { order_id: 'o2', channel: 'tiktok', status: 'CONFIRMED', at_ms: 3 },
  { order_id: 'o3', channel: 'tiktok', status: 'DELIVERED', at_ms: 4 },
  { order_id: 'o4', channel: 'instagram', status: 'PENDING', at_ms: 5 },
  { order_id: 'o5', channel: 'instagram', status: 'REJECTED', at_ms: 6 },
  { order_id: 'o6', channel: 'organic', status: 'DELIVERED', at_ms: 7 },
  { order_id: 'o7', channel: 'tiktok', status: 'PREPARING', at_ms: 8 },
  { order_id: 'o_anom1', channel: 'tiktok', status: 'PENDING', at_ms: 9 },
  { order_id: 'o_anom1', channel: 'tiktok', status: 'CONFIRMED', at_ms: 10 },
  { order_id: 'o_anom1', channel: 'tiktok', status: 'PREPARING', at_ms: 11 },
  { order_id: 'o_anom1', channel: 'tiktok', status: 'READY', at_ms: 12 },
  { order_id: 'o_anom1', channel: 'tiktok', status: 'IN_DELIVERY', at_ms: 13 },
  { order_id: 'o_anom1', channel: 'tiktok', status: 'DELIVERED', at_ms: 14 },
  { order_id: 'o_anom2', channel: 'instagram', status: 'PENDING', at_ms: 15 },
  { order_id: 'o_anom2', channel: 'instagram', status: 'DELIVERED', at_ms: 16 }, // illegal -> anomaly
];

const ledger = JSON.parse(K.channel_ledger_js(JSON.stringify(sample)));
console.log('kernel channel_ledger:', JSON.stringify(ledger));

// orders_by_channel: tiktok 5, instagram 3, organic 1 (desc by count, then name).
assert.deepEqual(
  ledger.orders_by_channel,
  [['tiktok', 5], ['instagram', 3], ['organic', 1]],
  'orders_by_channel must match the legacy JS port oracle'
);

// funnel per channel (fixed 10-stage shape; missing stages read 0).
const tiktok = Object.fromEntries(ledger.funnel.tiktok);
assert.equal(tiktok.CONFIRMED, 2, 'tiktok CONFIRMED');
assert.equal(tiktok.DELIVERED, 2, 'tiktok DELIVERED');
assert.equal(tiktok.PREPARING, 1, 'tiktok PREPARING');

const instagram = Object.fromEntries(ledger.funnel.instagram);
assert.equal(instagram.PENDING, 1, 'instagram PENDING');
assert.equal(instagram.REJECTED, 1, 'instagram REJECTED');
assert.equal(instagram.DELIVERED, 1, 'instagram DELIVERED (anomaly order)');

const organic = Object.fromEntries(ledger.funnel.organic);
assert.equal(organic.DELIVERED, 1, 'organic DELIVERED');

// anomalies: exactly one illegal sequence (o_anom2: PENDING -> DELIVERED).
assert.equal(ledger.anomalies, 1, 'exactly one anomaly (o_anom2)');

console.log('PASS: channel_ledger_js matches OwnerDashboard legacy oracle (RW-02 safe to rewire)');
