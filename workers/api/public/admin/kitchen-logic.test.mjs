// node --test workers/api/public/admin/kitchen-logic.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as K from './kitchen-logic.js';

const b64 = o => Buffer.from(JSON.stringify(o)).toString('base64url');
const jwt = claims => `h.${b64(claims)}.s`;
const kitchenTok = jwt({ role: 'staff', sub: 'p1', caps: 'advance,catalog,stock' });
const waiterTok = jwt({ role: 'staff', sub: 'p2', caps: 'take_orders,take_payment' });
const ownerTok = jwt({ role: 'owner', sub: 'u1' });

const T = (id, status, at, items, fulfilment = { kind: 'pickup' }) => ({ id, status, created_at_ms: at, items, fulfilment });
const L = (name, quantity, station) => ({ product_id: name.toLowerCase(), name, quantity, ...(station ? { station } : {}) });

test('a kitchen token opens the board, the menu and the stock, and nothing else', () => {
  const p = K.principalOf(kitchenTok);
  assert.equal(p.staff, true);
  assert.deepEqual(K.tabsFor(p, ['orders', 'kitchen', 'menu', 'stock', 'couriers', 'more']), ['kitchen', 'menu', 'stock']);
  assert.equal(K.can(p, 'stock'), true);
  assert.equal(K.can(p, 'take_payment'), false);
});

test('ingredients and stock are one tab, opened by the shelf or the menu word', () => {
  const all = ['orders', 'kitchen', 'menu', 'stock', 'couriers', 'more'];
  assert.deepEqual(K.tabsFor({ staff: true, caps: new Set(['catalog']) }, all), ['menu', 'stock']);
  assert.deepEqual(K.tabsFor({ staff: true, caps: new Set(['stock']) }, all), ['stock']);
  assert.equal(all.filter(x => x === 'stock').length, 1, 'one entry, not two');
});

test('a waiter token opens the queue only; an owner opens everything', () => {
  const all = ['orders', 'kitchen', 'menu', 'stock', 'couriers', 'more'];
  assert.deepEqual(K.tabsFor(K.principalOf(waiterTok), all), ['orders']);
  assert.deepEqual(K.tabsFor(K.principalOf(ownerTok), all), all);
  assert.equal(K.can(K.principalOf(ownerTok), 'anything'), true);
});

test('an unreadable token is nobody, never a member of staff with rights', () => {
  for (const t of [null, '', 'garbage', 'a.!!!.c', jwt({ role: 'staff' })]) {
    const p = K.principalOf(t);
    assert.equal(p.caps.size, 0);
  }
  assert.equal(K.claimsOf('a.b'), null);
  assert.deepEqual(K.claimsOf(jwt({ role: 'staff', caps: 'x', name: 'Кухня' })).name, 'Кухня');
});

test('the list comes from the stripped kitchen read for a kitchen, the queue for the room', () => {
  assert.match(K.ordersPath(K.principalOf(kitchenTok), 'v 1'), /^\/staff\/kitchen\?location_id=v%201$/);
  assert.match(K.ordersPath(K.principalOf(waiterTok), 'v'), /^\/owner\/orders\?/);
  assert.match(K.ordersPath(K.principalOf(ownerTok), 'v'), /^\/owner\/orders\?/);
});

test('columns: new, preparing, ready; nothing else is on the pass', () => {
  assert.equal(K.columnOf('PENDING'), 'new');
  assert.equal(K.columnOf('CONFIRMED'), 'new');
  assert.equal(K.columnOf('PREPARING'), 'preparing');
  assert.equal(K.columnOf('READY'), 'ready');
  for (const s of ['IN_DELIVERY', 'DELIVERED', 'PICKED_UP', 'CANCELLED', 'REJECTED', undefined]) assert.equal(K.columnOf(s), null);
});

test('one bump per ticket, along the kitchen edges', () => {
  assert.equal(K.bumpFor({ status: 'PENDING' }), 'confirm');
  assert.equal(K.bumpFor({ status: 'CONFIRMED' }), 'preparing');
  assert.equal(K.bumpFor({ status: 'PREPARING' }), 'ready');
  assert.equal(K.bumpFor({ status: 'READY', fulfilment: { kind: 'pickup' } }), 'collected');
  assert.equal(K.bumpFor({ status: 'READY', fulfilment: { kind: 'delivery' } }), null, 'a delivery is the courier\'s from READY');
  assert.equal(K.bumpFor({ status: 'DELIVERED' }), null);
  assert.equal(K.bumpFor(null), null);
});

test('reject before accepting, cancel after, nothing once cooking', () => {
  assert.equal(K.stopFor({ status: 'PENDING' }), 'reject');
  assert.equal(K.stopFor({ status: 'CONFIRMED' }), 'cancel');
  assert.equal(K.stopFor({ status: 'PREPARING' }), null);
  assert.equal(K.stopFor(null), null);
});

test('ticket age colours by whole minutes, and the venue\'s own cooking time', () => {
  const min = 60_000;
  assert.equal(K.ageClass(0, 9 * min + 59_000), 'ok');
  assert.equal(K.ageClass(0, 10 * min), 'warn');
  assert.equal(K.ageClass(0, 20 * min), 'late');
  assert.equal(K.ageMin(5 * min, 0), 0, 'a clock behind the hub is not a negative age');
  assert.equal(K.ageMin(undefined, 5), 0);
  assert.deepEqual(K.thresholds({ kitchen: { defaultCookingMin: 12 } }), { warn: 12, late: 24 });
  assert.deepEqual(K.thresholds(null), { warn: K.WARN_MIN, late: K.LATE_MIN });
});

test('a station filter shows only its lines, and a ticket with none of them is hidden', () => {
  const orders = [
    T('a', 'PENDING', 2, [L('Miso', 1), L('Beer', 2, 'bar')]),
    T('b', 'PREPARING', 1, [L('Dragon', 3, 'sushi')]),
    T('c', 'DELIVERED', 0, [L('Miso', 9)]),
  ];
  assert.deepEqual(K.ticketsFor(orders, 'bar').map(o => [o.id, o.items.length]), [['a', 1]]);
  assert.deepEqual(K.ticketsFor(orders, 'kitchen').map(o => o.id), ['a']);
  assert.deepEqual(K.ticketsFor(orders, 'sushi').map(o => o.id), ['b']);
  assert.deepEqual(K.ticketsFor(orders).map(o => o.id), ['b', 'a'], 'oldest first');
  assert.deepEqual(K.stationCounts(orders), { all: 2, sushi: 1, kitchen: 1, bar: 1 });
  assert.equal(K.stationOf({ station: 'grill' }), 'kitchen');
  const b = K.board(orders);
  assert.deepEqual([b.new.length, b.preparing.length, b.ready.length], [1, 1, 0]);
});

test('all-day counts sum what open tickets still need, READY excluded', () => {
  const orders = [
    T('a', 'PENDING', 1, [L('Miso', 1), L('Gyoza', 2)]),
    T('b', 'PREPARING', 2, [L('Miso', 3)]),
    T('c', 'READY', 3, [L('Miso', 50)]),
  ];
  assert.deepEqual(K.allDay(orders).map(d => [d.name, d.qty]), [['Miso', 4], ['Gyoza', 2]]);
  assert.deepEqual(K.allDay([]), []);
});

test('where a ticket goes names a table, pickup or delivery -- never an address', () => {
  assert.deepEqual(K.whereOf({ fulfilment: { table: 7 } }), { key: 'kTable', table: '7' });
  assert.deepEqual(K.whereOf({ fulfilment: { kind: 'delivery', address: { line: 'x' } } }), { key: 'kDelivery', table: '' });
  assert.deepEqual(K.whereOf({}), { key: 'kPickup', table: '' });
  assert.equal(K.shortId('ord_abc123'), 'C123');
});

test('a bump and a stop are the console\'s own request; a blank reason is not sent', () => {
  assert.deepEqual(K.actionRequest('o/1', 'ready', 'v'), { path: '/owner/orders/o%2F1/action', body: { location_id: 'v', action: 'ready' } });
  assert.deepEqual(K.actionRequest('o', 'reject', 'v', '  no salmon '), { path: '/owner/orders/o/action', body: { location_id: 'v', action: 'reject', reason: 'no salmon' } });
  assert.deepEqual(K.actionRequest('o', 'cancel', 'v', '   ').body, { location_id: 'v', action: 'cancel' });
  assert.deepEqual(K.WASTE_REASONS, ['spoiled', 'dropped', 'unsold', 'returned', 'staff_meal']);
});
