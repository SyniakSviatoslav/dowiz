/* eslint-disable local/no-hardcoded-string -- internal runtime/error strings (WASM glue, error codes, header names, selectors) and test seams -- not user-facing UI copy; do not wrap in t() */
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
