// node --test workers/api/public/admin/stock-week-logic.test.mjs
// The week's purchase list, the pure part: the quantity each row asks for,
// the rows grouped by supplier into the text a supplier receives, how many
// weeks the average stands on, the share of sales with no recipe, and every
// language holding every word. Shaped as `analytics/kitchen/report/week.rs`
// answers (`GET /api/owner/analytics/kitchen` `week`).
import test from 'node:test';
import assert from 'node:assert/strict';
import { qtyOf, groups, dataText, shareText, supplierText, screenQty } from './stock-week-logic.js';
import { WORDS } from './stock-week-words.js';
import { LANGS } from '../lib/langs.js';

const w = k => `[${k}]`;
const week = {
  from: '2026-10-26', to: '2026-11-22', dataFrom: '2026-11-09', dataDays: 14,
  rows: [
    { id: 'nori', name: 'Nori', unit: 'g', supplier: '', pack: null, weekly: 70, need: 80, stockKnown: false, toBuy: null, cost: null, priced: false },
    { id: 'rice', name: 'Rice', unit: 'g', supplier: 'Sea', pack: 1000, weekly: 1601, need: 2058, stockKnown: true, toBuy: 2000, cost: 1000, priced: true },
    { id: 'salmon', name: 'Salmon', unit: 'g', supplier: 'Sea', pack: 500, weekly: 700, need: 800, stockKnown: false, toBuy: null, cost: null, priced: false },
    { id: 'soy', name: 'Soy', unit: 'ml', supplier: 'Sea', pack: null, weekly: 10, need: 12, stockKnown: true, toBuy: 0, cost: 0, priced: true },
  ],
  coverage: { revenue: 4500, noRecipe: { revenue: 1500, pm: 333, portions: 3 }, top: [{ id: 'tea', name: 'Tea', revenue: 1000, sold: 2 }] },
};

test('a counted row asks what the hub said to buy; an uncounted one its need, up to whole packs', () => {
  assert.equal(qtyOf(week.rows[1]), 2000);
  assert.equal(qtyOf(week.rows[2]), 1000, '800 in packs of 500');
  assert.equal(qtyOf(week.rows[0]), 80, 'no pack: the need itself');
  assert.equal(qtyOf(week.rows[3]), 0, 'counted and covered: nothing');
});

test('rows to buy, grouped by supplier, with no supplier last; a covered row is left out', () => {
  const g = groups(week);
  assert.deepEqual(g.map(x => [x.supplier, x.lines.map(l => [l.id, l.qty])]), [['Sea', [['rice', 2000], ['salmon', 1000]]], [null, [['nori', 80]]]]);
  // The supplier's text names whole packs, and only rows with a quantity.
  const text = supplierText(g[0], 'Dubin', w);
  assert.match(text, /- Rice: 2 kg \(2 x 1 kg\)/);
  assert.match(text, /- Salmon: 1 kg \(2 x 500 g\)/);
  assert.doesNotMatch(text, /Soy/);
  // Twin: nothing to buy anywhere is no group at all.
  assert.deepEqual(groups({ rows: [week.rows[3]] }), []);
  assert.deepEqual(groups(null), []);
});

test('the average says what it stands on: whole weeks, else days; no draw is no average', () => {
  assert.equal(dataText(week, w), '2 [wp_weeks] · [wp_since] 2026-11-09');
  assert.equal(dataText({ dataDays: 10, dataFrom: '2026-11-13' }, w), '10 [wp_days] · [wp_since] 2026-11-13');
  assert.equal(dataText({ dataDays: 7, dataFrom: '2026-11-16' }, w), '1 [wp_week] · [wp_since] 2026-11-16');
  assert.equal(dataText({ dataDays: 0, dataFrom: null }, w), '[wp_noData]');
});

test('the share of sales no recipe draws for, in whole per cent; nothing sold claims nothing', () => {
  assert.equal(shareText(week.coverage), '33 %');
  assert.equal(shareText({ noRecipe: { pm: 5 } }), '1 %', 'rounded half up');
  assert.equal(shareText({ noRecipe: { pm: 1000 } }), '100 %');
  assert.equal(shareText({ noRecipe: { pm: null } }), null);
  assert.equal(shareText(undefined), null);
});

test('the screen writes an amount exactly with a point, as every stock screen does; the supplier text keeps its own', () => {
  assert.equal(screenQty(2798, 'g'), '2.798 kg');
  assert.equal(screenQty(26480, 'g'), '26.48 kg');
  assert.equal(screenQty(1000, 'ml'), '1 l');
  assert.equal(screenQty(999, 'g'), '999 g');
  assert.equal(screenQty(0, 'g'), '0 g');
  assert.equal(screenQty(-32448, 'g'), '-32.448 kg');
  assert.equal(screenQty(12, 'unit'), '12');
});

test('the lead time reads after any number in English: never "1 days"', () => {
  assert.ok(!/^days\b/.test(WORDS.en.wp_lead), WORDS.en.wp_lead);
});

test('every language has every word, and no word is empty', () => {
  const keys = new Set(Object.values(WORDS).flatMap(Object.keys));
  for (const l of LANGS) {
    assert.ok(WORDS[l], `${l} is there`);
    for (const k of keys) assert.ok(WORDS[l][k], `${l}.${k}`);
  }
});
