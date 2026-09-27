// node --test workers/api/public/admin/access.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import * as A from './access.js';
import { principalOf } from './kitchen-logic.js';

const b64 = o => Buffer.from(JSON.stringify(o)).toString('base64url');
const jwt = claims => `h.${b64(claims)}.s`;
const kitchen = principalOf(jwt({ role: 'staff', sub: 'p1', caps: 'advance,catalog,stock' }));
const waiter = principalOf(jwt({ role: 'staff', sub: 'p2', caps: 'take_orders,take_payment' }));
const owner = principalOf(jwt({ role: 'owner', sub: 'u1' }));
const TABS = ['orders', 'kitchen', 'menu', 'stock', 'couriers', 'more'];

const here = new URL('.', import.meta.url);
const src = f => readFileSync(new URL(f, here), 'utf8');
/// The Venue rows as more.js declares them, read from its source.
const moreKeys = () => [...src('more.js').split('const GROUPS')[1].split('];\n')[0].matchAll(/\['(\w+)', '[\w-]+', open\w+\]/g)].map(m => m[1]);
const tileIds = () => [...src('wire.js').matchAll(/\{ id: '(\w+)'/g)].map(m => m[1]);

test('every Venue row and every tile has a decision, and no decision names a row that is gone', () => {
  const rows = moreKeys(), tiles = tileIds();
  assert.ok(rows.length >= 30, `parsed only ${rows.length} rows of more.js`);
  assert.ok(tiles.length >= 8, `parsed only ${tiles.length} tiles of wire.js`);
  assert.deepEqual([...rows].sort(), Object.keys(A.SECTION_CAPS).sort());
  assert.deepEqual([...tiles].sort(), Object.keys(A.TILE_CAPS).sort());
});

test('the kitchen opens the pass, the menu, the shelf and the Venue rows it needs', () => {
  assert.deepEqual(A.tabsFor(kitchen, TABS), ['kitchen', 'menu', 'stock', 'more']);
  for (const k of ['bookings', 'floorPlan', 'hours', 'printer', 'learn', 'preview']) assert.equal(A.canOpen(kitchen, k), true, k);
  assert.equal(A.canTile(kitchen, 'catWords'), true);
});

test('the kitchen is refused money, customers, people, keys, legal and safety', () => {
  for (const k of ['payments', 'analytics', 'customers', 'staff', 'dpa', 'health', 'apiKeys', 'mcp', 'ebills', 'exceptions', 'inbox', 'promos', 'venue', 'deliveryTerms', 'cloud', 'notifications'])
    assert.equal(A.canOpen(kitchen, k), false, k);
  for (const id of ['wallets', 'tax', 'safety', 'messages', 'history', 'brand', 'graph']) assert.equal(A.canTile(kitchen, id), false, id);
  assert.equal(A.tabsFor(kitchen, TABS).includes('orders'), false);
  assert.equal(A.tabsFor(kitchen, TABS).includes('couriers'), false);
});

test('an owner loses nothing', () => {
  assert.deepEqual(A.tabsFor(owner, TABS), TABS);
  for (const k of Object.keys(A.SECTION_CAPS)) { assert.equal(A.canOpen(owner, k), true, k); assert.equal(A.readOnly(owner, k), false, k); }
  for (const id of Object.keys(A.TILE_CAPS)) assert.equal(A.canTile(owner, id), true, id);
});

test('a waiter holds none of the kitchen words: no Venue tab, no kitchen rows', () => {
  assert.deepEqual(A.tabsFor(waiter, TABS), ['orders']);
  for (const k of ['bookings', 'printer', 'hours']) assert.equal(A.canOpen(waiter, k), false, k);
});

test('what the kitchen only reads is drawn without its save', () => {
  for (const k of ['bookings', 'floorPlan', 'hours']) assert.equal(A.readOnly(kitchen, k), true, k);
  assert.equal(A.readOnly(kitchen, 'printer'), false);
});

test('the Venue list keeps only open rows and drops an empty group', () => {
  const groups = [['inbox', [['inbox', 'i']]], ['roomGroup', [['bookings', 'b'], ['floorPlan', 'f'], ['tableQr', 't']]], ['settingsVenue', [['payments', 'p'], ['hours', 'h']]]];
  assert.deepEqual(A.sectionsFor(kitchen, groups), [['roomGroup', [['bookings', 'b'], ['floorPlan', 'f']]], ['settingsVenue', [['hours', 'h']]]]);
  assert.deepEqual(A.sectionsFor(owner, groups), groups);
  const tiles = [{ id: 'catWords' }, { id: 'wallets' }];
  assert.deepEqual(A.tilesFor(kitchen, tiles), [{ id: 'catWords' }]);
  assert.deepEqual(A.tilesFor(owner, tiles), tiles);
});

test('lessons follow the screens a person can open and change', () => {
  const L = ['kitchen', 'menu', 'stock', 'orders', 'bookings', 'payments', 'staff'].map(module => ({ id: module, module }));
  assert.deepEqual(A.lessonsFor(kitchen, L, TABS).map(l => l.module), ['kitchen', 'menu', 'stock']);
  assert.deepEqual(A.lessonsFor(owner, L, TABS), L);
});
