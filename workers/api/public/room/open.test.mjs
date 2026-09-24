// node --test workers/api/public/room/open.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { tableOk, placeBody, TABLE_MAX } from './open.js';

test('open: a table is named, trimmed and bounded', () => {
  assert.equal(tableOk(' 7 '), true);
  assert.equal(tableOk('   '), false, 'blank is no table');
  assert.equal(tableOk(''), false);
  assert.equal(tableOk('x'.repeat(TABLE_MAX)), true);
  assert.equal(tableOk('x'.repeat(TABLE_MAX + 1)), false);
});

test('open: the placement is dine_in at the table, intents only, never a price', () => {
  const b = placeBody(' 12 ', [{ op: 'add', product_id: 'p1', quantity: 2, modifier_ids: [] }]);
  assert.deepEqual(b.fulfilment, { kind: 'dine_in', table: '12' });
  assert.deepEqual(b.items, [{ product_id: 'p1', quantity: 2, modifier_ids: [] }]);
  assert.equal(JSON.stringify(b).includes('price'), false);
});

test('open: a round is placed with no payment method -- it is chosen at pay time', () => {
  const b = placeBody('3', [{ op: 'add', product_id: 'p1', quantity: 1, modifier_ids: [] }]);
  assert.equal('payment' in b, false, JSON.stringify(b));
});
