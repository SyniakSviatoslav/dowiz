// The replica's tests. `node public/lib/replica.test.mjs`, or via
// `bash scripts/design-gate.sh`, which is what the pre-push gate runs.
//
// These are here and not in a Rust crate for the same reason the money ones
// are: this is the fold that runs in a BROWSER, on a copy the server does not
// hold, and a Rust test passing says nothing about what a kitchen sees. The
// merge below has to agree with `workers/api/src/fold.rs` exactly -- two folds
// that disagree are a queue that disagrees with the venue's own log.

import assert from 'node:assert/strict';

// The module reaches for `localStorage`, which node has not got. A replica
// that cannot be stored must still WORK -- that is the private-window case --
// so the absence here is also a test.
globalThis.localStorage = undefined;

const { load, replace, apply, ageOf, isStale, STALE_MS } = await import('./replica.js');

let run = 0;
const test = (name, fn) => { fn(); run++; console.log(`  ok  ${name}`); };

const placed = (id, extra = {}) => ({
  id,
  status: 'PENDING',
  total: 2650,
  contact: { name: 'Ana', phone: '+355691234567' },
  fulfilment: { kind: 'delivery', address: { line: 'Rruga Taulantia 12' } },
  ...extra,
});
const change = (generation, order_id, payload) => ({
  generation,
  kind: 2, // Advanced: an order event
  order_id,
  payload: JSON.stringify(payload),
});

test('an empty browser has an empty copy, and storage that throws is not fatal', () => {
  const c = load('v1');
  assert.equal(c.venue, 'v1');
  assert.equal(c.generation, -1);
  assert.deepEqual(c.orders, []);
});

test('a delta is merged into the order it names', () => {
  const copy = replace('v1', [placed('ord_1')], 10);
  const next = apply(copy, [change(11, 'ord_1', { _d: true, status: 'COOKING', cooking_ms: 5 })], 11);
  assert.equal(next.generation, 11);
  assert.equal(next.orders.length, 1);
  assert.equal(next.orders[0].status, 'COOKING');
  assert.equal(next.orders[0].cooking_ms, 5);
  assert.equal(next.orders[0].contact.name, 'Ana', 'the rest of the order survives');
  assert.equal(next.orders[0]._d, undefined, 'the marker never reaches the order');
});

test('the _x list deletes keys, exactly as the server folds it', () => {
  const copy = replace('v1', [placed('ord_1', { courier_id: 'c1' })], 1);
  const next = apply(copy, [change(2, 'ord_1', { _d: true, _x: ['courier_id'] })], 2);
  assert.equal('courier_id' in next.orders[0], false);
});

test('A NULL IS A VALUE AND IS KEPT, not read as a deletion', () => {
  // The kernel writes explicit nulls (customer_id, channel, cash_pay_with) and
  // so does the Worker (rejection_reason). While a null meant "delete", this
  // fold and the server's disagreed about the shape of the same order.
  const copy = replace('v1', [placed('ord_1')], 1);
  const next = apply(copy, [change(2, 'ord_1', { _d: true, rejection_reason: null })], 2);
  assert.equal('rejection_reason' in next.orders[0], true);
  assert.equal(next.orders[0].rejection_reason, null);
});

test('AN AUDIT RECORD IS NOT AN ORDER and never enters the copy', () => {
  // `Revealed` (kind 4) names who read a customer's details, under a subject
  // that is not an order id. Folded as an order it would put a row with a
  // reader's name and no items in the kitchen's queue -- and keep it across
  // reloads.
  const copy = replace('v1', [placed('ord_1')], 1);
  const reveal = { generation: 2, kind: 4, order_id: 'cust:abc',
                   payload: JSON.stringify({ by: 'owner_1', at: 2, reason: 'call' }) };
  const next = apply(copy, [reveal], 2);
  assert.equal(next.orders.length, 1, 'still one order');
  assert.equal(next.orders[0].id, 'ord_1');
  assert.equal(next.generation, 2, 'and the copy still advances');
});

test('a checkpoint is ignored too', () => {
  const copy = replace('v1', [placed('ord_1')], 1);
  const mark = { generation: 2, kind: 6, order_id: '', payload: 'tip=abc events=3' };
  assert.equal(apply(copy, [mark], 2).orders.length, 1);
});

test('nested objects recurse and their siblings survive', () => {
  const copy = replace('v1', [placed('ord_1')], 1);
  const next = apply(copy, [change(2, 'ord_1', { _d: true, fulfilment: { eta_ms: 900 } })], 2);
  assert.equal(next.orders[0].fulfilment.eta_ms, 900);
  assert.equal(next.orders[0].fulfilment.address.line, 'Rruga Taulantia 12');
});

test('an array is replaced whole, never merged index by index', () => {
  const copy = replace('v1', [placed('ord_1', { items: [{ p: 'a' }, { p: 'b' }] })], 1);
  const next = apply(copy, [change(2, 'ord_1', { _d: true, items: [{ p: 'a' }] })], 2);
  assert.deepEqual(next.orders[0].items, [{ p: 'a' }]);
});

test('a snapshot replaces the order rather than merging with it', () => {
  const copy = replace('v1', [placed('ord_1', { note: 'no onions' })], 1);
  const next = apply(copy, [change(2, 'ord_1', { id: 'ord_1', status: 'CONFIRMED' })], 2);
  assert.equal(next.orders[0].status, 'CONFIRMED');
  assert.equal(next.orders[0].note, undefined, 'a snapshot is the whole order');
});

test('a placement for an order the copy has never seen is ADDED', () => {
  const copy = replace('v1', [], 1);
  const next = apply(copy, [change(2, 'ord_9', placed('ord_9'))], 2);
  assert.equal(next.orders.length, 1);
  assert.equal(next.orders[0].id, 'ord_9');
});

test('A DELTA FOR AN ORDER THE COPY DOES NOT HOLD IS REFUSED', () => {
  // The caller then reads the whole list. Folding a delta onto nothing would
  // draw a row with a status and no items, and a kitchen would cook it.
  const copy = replace('v1', [placed('ord_1')], 1);
  assert.equal(apply(copy, [change(2, 'ord_2', { _d: true, status: 'COOKING' })], 2), null);
});

test('a damaged payload is refused rather than half-applied', () => {
  const copy = replace('v1', [placed('ord_1')], 1);
  assert.equal(apply(copy, [{ generation: 2, order_id: 'ord_1', payload: 'not json' }], 2), null);
  assert.equal(apply(copy, [{ generation: 2, payload: '{}' }], 2), null, 'no order id');
  assert.equal(apply(null, [], 2), null);
});

test('changes older than the copy change nothing', () => {
  const copy = replace('v1', [placed('ord_1')], 10);
  const same = apply(copy, [change(5, 'ord_1', { _d: true, status: 'COOKING' })], 5);
  assert.equal(same.orders[0].status, 'PENDING', 'the copy is already ahead');
});

test('a copy knows how old it is', () => {
  const copy = replace('v1', [placed('ord_1')], 1);
  assert.ok(ageOf(copy) < 1000);
  assert.equal(isStale(copy), false);
  assert.equal(isStale({ ...copy, at: Date.now() - STALE_MS - 1 }), true);
  assert.equal(isStale(null), true, 'no copy is as stale as it gets');
});

console.log(`replica: ${run} tests ok`);
