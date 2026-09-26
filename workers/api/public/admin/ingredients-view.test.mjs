// node --test workers/api/public/admin/ingredients-view.test.mjs
// An ingredient's row and card, the alarm chips and the dishes that reduce no
// stock, drawn from supplies shaped exactly like `GET /api/owner/stock`'s
// (`services/operations/stock/view/tests.rs`).
import test from 'node:test';
import assert from 'node:assert/strict';
import * as V from './ingredients-view.js';

const fmt = { money: n => `L${n}`, t: k => `[${k}]`, when: ms => `@${ms}`, warnDays: 2 };
const salmon = {
  id: 'salmon', name: 'Salmon <script>', unit: 'g', kind: 'food_ingredient', category: 'Fish', counted: true,
  onHand: 2900, reserved: 100, available: 2800, lowAt: 500, low: false, wac: 320, costPerBasis: 250, cleanPm: 550,
  lots: [{ code: 'A', left: 900, expiry: '2026-09-25', daysLeft: -1, supplier: 'Sea' }, { code: 'B', left: 1000, expiry: '2026-09-27', daysLeft: 1 }],
  expiring: 2, expired: 1, measuredCleanPm: 560, measuredCookPm: null,
  prices: [{ at: 1, perBasis: 300, qty: 1000, supplier: 'Sea', doc: 'F-1' }],
  moves: [{ at: 2, kind: 'stocktake', qty: 2900, drift: -100, value: -320 }, { at: 3, kind: 'wasted', qty: 5, reason: 'dropped', value: 16, lot: 'A' }],
};
const nori = { id: 'nori', name: 'Nori', unit: 'unit', kind: 'food_ingredient', counted: false, onHand: -12, reserved: 0, available: -12 };

test('a row: price at the average, levels, the nearest date, the four actions', () => {
  const html = V.rowMarkup(salmon, fmt);
  assert.ok(html.includes('L320/100g [inv_wac]'), 'the average a priced delivery set, not the list price');
  assert.ok(html.includes('[inv_onHand] 2900') && html.includes('[inv_available] 2800 g'));
  assert.ok(html.includes('[inv_expiry] 2026-09-25') && html.includes('ui-badge--danger'), 'an expired lot is red');
  for (const a of ['received', 'wasted', 'stocktake', 'prep']) assert.ok(html.includes(`data-act="${a}"`), a);
  assert.ok(!html.includes('<script>'), 'a name is text');
  const n = V.rowMarkup(nori, fmt);
  assert.ok(n.includes('data-t="inv_needsCount"') && !n.includes('class="gauge"'), 'I0: an uncounted minus is a count due, with no gauge');
});

test('the alarms count and filter, and the dishes that reduce no stock group by category', () => {
  const html = V.alertsMarkup([salmon, nori], [{ id: 'cola', name: 'Cola', categoryId: 'drinks' }], 'needsCount', fmt.t);
  assert.ok(html.includes('1 [inv_needsCount]') && html.includes('1 [inv_expiring]') && html.includes('1 [inv_noStockLink]'));
  assert.ok(!html.includes('data-flag="low"'), 'no low item, no chip');
  assert.ok(/aria-pressed="true"[^>]*data-flag="needsCount"|data-flag="needsCount"[^>]*aria-pressed="true"/.test(html));
  assert.deepEqual([salmon, nori].filter(s => V.matches(s, 'expiring')).map(s => s.id), ['salmon']);
  assert.deepEqual([salmon, nori].filter(s => V.matches(s, '')).length, 2);
  const nr = V.noRecipeMarkup([{ id: 'cola', name: 'Cola', categoryId: 'drinks' }, { id: 'ayran', name: 'Ayran', categoryId: 'drinks' }], fmt.t);
  assert.ok(nr.includes('data-asiscat="drinks"') && nr.includes('data-asis="cola"') && nr.includes('data-asis="ayran"'));
  assert.equal(V.noRecipeMarkup([], fmt.t), '');
});

test('the card: losses with the measured yield to adopt, lots, prices and movements', () => {
  const html = V.cardMarkup(salmon, fmt);
  assert.ok(html.includes('1000 g') && html.includes('550 g') && html.includes('45%'), 'gross -> net -> out at the supply losses');
  assert.ok(html.includes('data-adopt="clean"') && html.includes('data-pm="560"') && !html.includes('data-adopt="cook"'));
  assert.ok(html.includes('>A<') && html.includes('-1 [inv_daysLeft]'));
  assert.ok(html.includes('@1') && html.includes('L300/100g') && html.includes('F-1'));
  assert.ok(html.includes('2900 (-100)') && html.includes('L-320') && html.includes('[dropped]'));
  for (const a of ['received', 'wasted', 'stocktake', 'prep']) assert.ok(html.includes(`data-cact="${a}"`), a);
  const bare = V.cardMarkup(nori, fmt);
  assert.ok(bare.includes('data-t="inv_needsCountHint"') && bare.includes('data-t="none"'), 'nothing to show says so');
});
