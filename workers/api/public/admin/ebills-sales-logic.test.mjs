// node --test public/admin/ebills-sales-logic.test.mjs -- the till tab's arithmetic (W-EBILLS-TAB).
import test from 'node:test';
import assert from 'node:assert/strict';
import { heights, pct, weekdays, matchOf, payKey, search, ofDay, csv } from './ebills-sales-logic.js';

const rows = [
  { sale_id: 9329, inv_ord_num: 9325, at_ms: 1_793_728_800_000, kind: 'dine_in', table: '4', fic: 'ABC-1', payment: 'CASH', payment_status: null, status: 'PICKED_UP', total: 700,
    items: [{ code: '56', name: 'Sake roll', product_id: 'p-roll', matched_by: 'map', quantity: 2, unit_price: 250 }, { code: '12', name: 'Miso', product_id: 'p-miso', matched_by: 'name', quantity: 1, unit_price: 200 }] },
  { sale_id: 9330, inv_ord_num: 9326, at_ms: 1_793_642_400_000, kind: 'pickup', table: null, fic: 'ABC-2', payment: 'CARD', payment_status: 'paid', status: 'PICKED_UP', total: 900,
    items: [{ code: '77', name: 'Dragon "roll", xl', product_id: null, matched_by: null, quantity: 1, unit_price: 900 }] },
];

test('bar heights scale to the tallest, and an empty or flat series is all zero', () => {
  assert.deepEqual(heights([{ revenue: 50 }, { revenue: 100 }, { revenue: 0 }]), [50, 100, 0]);
  assert.deepEqual(heights([{ orders: 3 }, { orders: 1 }], 'orders'), [100, 33]);
  assert.deepEqual(heights([]), []);
  assert.deepEqual(heights([{ revenue: 0 }, {}]), [0, 0]);
});

test('a share in ppm is a percent with one decimal', () => {
  assert.equal(pct(123456), '12.3%');
  assert.equal(pct(1_000_000), '100.0%');
  assert.equal(pct(0), '0.0%');
  assert.equal(pct(999), '0.1%');
  assert.equal(pct('x'), '');
});

test('weekdays start on Monday in the locale asked for', () => {
  assert.equal(weekdays('en').length, 7);
  assert.match(weekdays('en')[0], /Mon/);
  assert.match(weekdays('en')[6], /Sun/);
});

test('a line is matched by the tap, by its name, or open', () => {
  assert.equal(matchOf(rows[0].items[0]), 'map');
  assert.equal(matchOf(rows[0].items[1]), 'name');
  assert.equal(matchOf(rows[1].items[0]), 'open');
  assert.equal(matchOf(undefined), 'open');
});

test('the payment word covers the three card spellings and nothing else', () => {
  assert.equal(payKey('CASH'), 'cash');
  for (const c of ['CARD', 'CARD_ON_POS', 'POK_CARD', 'card']) assert.equal(payKey(c), 'card');
  assert.equal(payKey('BANK_TRANSFER'), null);
  assert.equal(payKey(undefined), null);
});

test('the search reads sale ids, tables, FICs, names and codes, diacritics apart', () => {
  assert.deepEqual(search(rows, '').map(r => r.sale_id), [9329, 9330]);
  assert.deepEqual(search(rows, '9330').map(r => r.sale_id), [9330]);
  assert.deepEqual(search(rows, 'sake 4').map(r => r.sale_id), [9329]);
  assert.deepEqual(search(rows, 'DRAGON').map(r => r.sale_id), [9330]);
  assert.deepEqual(search(rows, 'abc-1').map(r => r.sale_id), [9329]);
  assert.deepEqual(search(rows, 'nothing here'), []);
});

test('one day of rows, by the day function the screen uses', () => {
  const dayOf = ms => (ms >= 1_793_700_000_000 ? '2026-11-03' : '2026-11-02');
  assert.deepEqual(ofDay(rows, '2026-11-02', dayOf).map(r => r.sale_id), [9330]);
  assert.equal(ofDay(rows, null, dayOf).length, 2);
});

test('the CSV is one line per till line with RFC 4180 quoting', () => {
  const lines = csv(rows).split('\n');
  assert.equal(lines.length, 4);
  assert.equal(lines[0].split(',')[0], 'sale_id');
  assert.equal(lines[1], '9329,9325,1793728800000,dine_in,4,CASH,,PICKED_UP,56,Sake roll,p-roll,map,2,250,700');
  assert.equal(lines[3], '9330,9326,1793642400000,pickup,,CARD,paid,PICKED_UP,77,"Dragon ""roll"", xl",,,1,900,900');
});
