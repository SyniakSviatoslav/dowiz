// RED PROOF (W-SERVICE-MODE, 2026-11-03): THE DELIVERY FEE IS CHARGED ONLY
// FOR A DELIVERY, the hall is offered exactly when the venue's plan has a
// table, and the basket sends the hub the shape it prices.
// `node --test workers/api/public/store/how.test.mjs`
//
// The venue blocks below are the live answers of 2026-11-03 (dubin-sushi:
// fee 300, free from 2000, pickup on, one zone "salla" with seven tables;
// sushi-durres: no plan), cut to the fields these rules read. The glue
// (state.js, cart.js, checkout.js) imports by absolute URL and touches the
// DOM, so its SOURCE is read at the end for the one line each must delegate.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { DELIVERY, PICKUP, DINE_IN, TABLE, KEY, PLAN_SLOT_LEAD_MIN, leavesTheBuilding, offered, pick, tripFee,
  hallTables, hallTable, tableText, missing, fulfilmentBody, keep, recall, planSlotMin } from './how.js';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const DUBIN = { pickup: true, deliveryFee: 300, freeDeliveryThreshold: 2000, minOrder: 500, currencyCode: 'ALL' };
const DURRES = { pickup: true, deliveryFee: 300, freeDeliveryThreshold: null, minOrder: 0, currencyCode: 'ALL' };
const SALLA = { slotMin: 29896900, zones: [{ id: 'salla', name: 'Salla', tables: [
  { n: 1, seats: 2, occupied: false }, { n: 2, seats: 2 }, { n: 3, seats: 2 }, { n: 4, seats: 4 }, { n: 5, seats: 4 }, { n: 6, seats: 4 }, { n: 7, seats: 6 }] }] };
const NO_PLAN = { slotMin: 29896900, zones: [] };

function recorder(){
  const m = new Map(), log = [];
  return { log, getItem: k => (log.push(['get', k]), m.has(k) ? m.get(k) : null),
    setItem: (k, v) => { log.push(['set', k, v]); m.set(k, String(v)); }, removeItem: k => { log.push(['del', k]); m.delete(k); } };
}

test('only a delivery leaves the building', () => {
  assert.equal(leavesTheBuilding(DELIVERY), true);
  for (const how of [PICKUP, DINE_IN, TABLE, '', undefined, 'banana']) assert.equal(leavesTheBuilding(how), false, String(how));
});

test('the hall is offered exactly when the plan has a table; pickup when the switch is on', () => {
  assert.deepEqual(offered(DUBIN, SALLA), [DELIVERY, PICKUP, DINE_IN]);
  assert.deepEqual(offered(DURRES, NO_PLAN), [DELIVERY, PICKUP], 'sushi-durres has no plan: no hall');
  assert.deepEqual(offered({ ...DUBIN, pickup: false }, SALLA), [DELIVERY, DINE_IN]);
  assert.deepEqual(offered({ pickup: false }, null), [DELIVERY], 'the plan not yet asked for');
  assert.deepEqual(offered(DUBIN, {}), [DELIVERY, PICKUP], 'an error body is a hall with no tables');
  assert.deepEqual(offered(DUBIN, { zones: [{ id: 'a', name: 'A', tables: [] }] }), [DELIVERY, PICKUP], 'a zone with no table');
  assert.deepEqual(offered(null, SALLA), [DELIVERY, DINE_IN], 'no venue yet: still never a pickup');
});

test('a remembered way the venue does not offer falls back to the first it does', () => {
  assert.equal(pick(DINE_IN, [DELIVERY, PICKUP]), DELIVERY);
  assert.equal(pick(DINE_IN, [DELIVERY, PICKUP, DINE_IN]), DINE_IN);
  assert.equal(pick(PICKUP, [DELIVERY]), DELIVERY);
  assert.equal(pick(undefined, [DELIVERY, PICKUP]), DELIVERY);
});

test('the fee buys a trip: 300 for a delivery under the threshold, nothing for pickup, hall or table', () => {
  // The hub's own table (services/ordering/fulfilment/tests.rs: fee), case for case.
  assert.equal(tripFee(DELIVERY, 1500, DUBIN), 300);
  assert.equal(tripFee(PICKUP, 1500, DUBIN), 0);
  assert.equal(tripFee(DINE_IN, 1500, DUBIN), 0);
  assert.equal(tripFee(TABLE, 1500, DUBIN), 0);
  assert.equal(tripFee(DELIVERY, 2000, DUBIN), 0, 'at the threshold the delivery is free');
  assert.equal(tripFee(DELIVERY, 1999, DUBIN), 300);
  assert.equal(tripFee(DELIVERY, 99999, DURRES), 300, 'no threshold: the fee stands at any size');
  assert.equal(tripFee(DINE_IN, 99999, DURRES), 0);
  assert.equal(tripFee(DELIVERY, 1500, null), 0, 'no venue yet: nothing to charge');
  assert.equal(tripFee(DELIVERY, 1500, { deliveryFee: 3.5 }), 0, 'money is an integer or it is nothing');
  assert.equal(tripFee(DELIVERY, 1500, { deliveryFee: null }), 0);
});

test('the plan flattens to tables the hub names as zone:n; a table the plan lost is no table', () => {
  const all = hallTables(SALLA);
  assert.equal(all.length, 7);
  assert.deepEqual(all[3], { zone: 'salla', name: 'Salla', n: 4 });
  assert.equal(tableText('salla', 4), 'salla:4');
  assert.deepEqual(hallTable(SALLA, { zone: 'salla', n: 4 }), { zone: 'salla', name: 'Salla', n: 4 });
  assert.equal(hallTable(SALLA, { zone: 'salla', n: 9 }), null);
  assert.equal(hallTable(SALLA, { zone: 'terasa', n: 1 }), null);
  assert.equal(hallTable(SALLA, null), null);
  assert.equal(hallTable(NO_PLAN, { zone: 'salla', n: 4 }), null);
  assert.deepEqual(hallTables({ zones: [{ id: '', tables: [{ n: 1 }] }, { id: 'z', tables: [{ n: '2' }, { n: 3 }] }] }), [{ zone: 'z', name: 'z', n: 3 }], 'a zone without an id and a table without a number are skipped');
});

test('what each way cannot be placed without (the hub\'s fulfilment::needs, said first)', () => {
  assert.equal(missing(DELIVERY, { addressLine: '  ' }), 'address');
  assert.equal(missing(DELIVERY, { addressLine: 'Rruga 1' }), null);
  assert.equal(missing(DINE_IN, { table: null }), 'table');
  assert.equal(missing(DINE_IN, { table: { zone: 'salla', n: 4 } }), null);
  assert.equal(missing(PICKUP, {}), null);
});

test('the order body: one shape per way, the one the hub reads (FulfilmentIn)', () => {
  const address = { line: 'Rruga 1 7', parts: { street: 'Rruga 1', house: '7' }, lat_udeg: 41313100, lon_udeg: 19445300 };
  assert.deepEqual(fulfilmentBody(DELIVERY, { address, note: 'ring twice' }),
    { kind: 'delivery', address: { ...address, note: 'ring twice' } });
  assert.deepEqual(fulfilmentBody(PICKUP, { address, note: '' }), { kind: 'pickup', note: null });
  assert.deepEqual(fulfilmentBody(DINE_IN, { address, note: 'no wasabi', table: { zone: 'salla', n: 4 } }),
    { kind: 'dine_in', table: 'salla:4', note: 'no wasabi' }, 'no address rides on a hall order');
  assert.deepEqual(fulfilmentBody(DINE_IN, {}), { kind: 'dine_in', table: null, note: null }, 'a missing table is sent as null and refused by the hub, never guessed');
});

test('the choice is remembered in the session store under one key, and junk reads as nothing', () => {
  const s = recorder();
  keep(s, DINE_IN, { zone: 'salla', n: 4, name: 'Salla' });
  assert.deepEqual(s.log.filter(e => e[0] === 'set').map(e => e[1]), [KEY]);
  assert.deepEqual(recall(s), { how: DINE_IN, table: { zone: 'salla', n: 4 } }, 'the name is the plan\'s, not remembered');
  keep(s, PICKUP, null);
  assert.deepEqual(recall(s), { how: PICKUP, table: null });
  s.setItem(KEY, '{"how":"banana","table":null}');
  assert.equal(recall(s), null);
  s.setItem(KEY, '{"how":"dine_in","table":{"zone":5,"n":"4"}}');
  assert.deepEqual(recall(s), { how: DINE_IN, table: null });
  s.setItem(KEY, 'not json');
  assert.equal(recall(s), null);
  const thrower = { getItem(){ throw new Error('private mode'); }, setItem(){ throw new Error('private mode'); } };
  assert.equal(recall(thrower), null);
  assert.doesNotThrow(() => keep(thrower, DELIVERY, null));
});

test('the plan is asked for a slot a few minutes ahead, in minutes since the epoch', () => {
  const now = Date.UTC(2026, 10, 3, 12, 0, 30);
  assert.equal(planSlotMin(now), Math.floor(now / 60000) + PLAN_SLOT_LEAD_MIN);
  assert.equal(PLAN_SLOT_LEAD_MIN, 10);
});

test('the glue delegates: one fee rule, one body shape, the mode block on both sheets', () => {
  const src = f => fs.readFileSync(path.join(HERE, f), 'utf8');
  const state = src('state.js'), cart = src('cart.js'), checkout = src('checkout.js'), eta = src('eta.js');
  assert.match(state, /export const deliveryFee = \(\) => tripFee\(state\.how, subtotal\(\), state\.loc\)/, 'state.js: the fee is how.js\'s');
  assert.doesNotMatch(state, /state\.how === 'pickup' \|\| state\.how === 'table'/, 'state.js: the old two-mode rule is gone');
  assert.match(cart, /leavesTheBuilding\(state\.how\)/, 'cart.js: the totals row follows the one predicate');
  assert.ok(cart.includes('${howBlock()}'), 'cart.js: the choice is drawn where the fee first appears');
  assert.ok(checkout.includes('${howBlock({ tables: true })}'), 'checkout.js: the same block, with the tables');
  assert.match(checkout, /fulfilment: fulfilmentBody\(state\.how,/, 'checkout.js: the body shape is how.js\'s');
  assert.doesNotMatch(checkout, /kind: 'delivery'|kind: 'pickup'/, 'checkout.js: no hand-written fulfilment shape');
  assert.match(eta, /pickup = !leavesTheBuilding\(state\.how\)/, 'eta.js: no road for anything that is not a delivery');
});
