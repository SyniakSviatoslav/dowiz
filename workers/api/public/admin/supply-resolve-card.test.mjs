// node --test workers/api/public/admin/supply-resolve-card.test.mjs
// PRICES BY DATE on a position's card (ingredients-view.js cardMarkup): every
// lot on hand with the day it came in, what is left and what it cost, newest
// first; and the price history under it, newest first.
import test from 'node:test';
import assert from 'node:assert/strict';
import * as V from './ingredients-view.js';

const fmt = { money: n => `L${n}`, t: k => `[${k}]`, when: ms => `@${ms}`, warnDays: 2 };
const rice = (more = {}) => ({
  id: 'rice', name: 'Рис', unit: 'g', kind: 'food_ingredient', counted: true, onHand: 2500, available: 2500, wac: 203,
  lots: [{ code: '#3', left: 1500, received: 2000, at: 100, expiry: null, daysLeft: null, supplier: 'Agro' },
    { code: '#5', left: 1000, received: 1000, at: 200, expiry: null, daysLeft: null, supplier: 'Agro' }],
  prices: [{ at: 100, perBasis: 180, qty: 2000, supplier: 'Agro', doc: 'F-1', lot: null }, { at: 200, perBasis: 250, qty: 1000, supplier: 'Agro', doc: 'F-2', lot: null }],
  ...more,
});
/// The rows of the table whose head carries `key`, as text.
const rowsOf = (html, key) => {
  const at = html.indexOf(`data-t="${key}"></th>`), body = html.slice(html.indexOf('<tbody>', at), html.indexOf('</tbody>', at));
  return body.split('<tr>').slice(1).map(r => r.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim());
};

test('lots on hand: the day each came in, what is left and its price, newest first', () => {
  const rows = rowsOf(V.cardMarkup(rice(), fmt), 'inv_lotIn');
  assert.equal(rows.length, 2);
  assert.ok(rows[0].startsWith('#5 @200 1000 g L250/100g'), rows[0]);
  assert.ok(rows[1].startsWith('#3 @100 1500 g L180/100g'), rows[1]);
});

test('lots on hand: the hub\'s own price on the lot wins; a price that cannot be told apart is not guessed', () => {
  const own = rowsOf(V.cardMarkup(rice({ lots: [{ code: 'A', left: 5, received: 5, at: 100, perBasis: 260 }] }), fmt), 'inv_lotIn');
  assert.ok(own[0].startsWith('A @100 5 g L260/100g'), own[0]);
  const twins = rice({ lots: [{ code: '#1', left: 1000, received: 1000, at: 7 }, { code: '#2', left: 1000, received: 1000, at: 7 }],
    prices: [{ at: 7, perBasis: 100, qty: 1000, lot: null }, { at: 7, perBasis: 120, qty: 1000, lot: null }] });
  for (const r of rowsOf(V.cardMarkup(twins, fmt), 'inv_lotIn')) assert.ok(/@7 1000 g - /.test(r), `two prices, one moment: ${r}`);
  const named = rice({ lots: [{ code: 'L1', left: 10, received: 10, at: 7 }, { code: 'L1/2', left: 5, received: 5, at: 7 }],
    prices: [{ at: 7, perBasis: 50, qty: 10, lot: 'L1' }, { at: 7, perBasis: 60, qty: 5, lot: 'L1/2' }] });
  const got = rowsOf(V.cardMarkup(named, fmt), 'inv_lotIn');
  assert.ok(got.some(r => r.startsWith('L1 @7 10 g L50/100g')) && got.some(r => r.startsWith('L1/2 @7 5 g L60/100g')), got.join(' | '));
  const bare = rowsOf(V.cardMarkup(rice({ lots: [{ code: '-', left: 3, received: 3 }], prices: [] }), fmt), 'inv_lotIn');
  assert.ok(bare[0].startsWith('- - 3 g - '), `no date, no price: ${bare[0]}`);
});

test('price history: one row per priced delivery with its day, price and quantity, newest first', () => {
  const rows = rowsOf(V.cardMarkup(rice(), fmt), 'inv_doc');
  assert.deepEqual(rows, ['@200 L250/100g 1000 g Agro F-2', '@100 L180/100g 2000 g Agro F-1']);
});
