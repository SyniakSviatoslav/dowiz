// node --test workers/api/public/admin/menu-margin-logic.test.mjs -- the menu's
// margin at a glance and the dish's 30-day history, from the kitchen answer.
import test from 'node:test';
import assert from 'node:assert/strict';
import { pct, badge, history, missingOf, unmatchedOf } from './menu-margin-logic.js';

const TODAY = [
  { id: 'sake', price: 1000, portionCost: 220, marginPortion: 780, foodCostPm: 220, hasRecipe: true, missing: [] },
  { id: 'unagi', price: 1200, portionCost: null, marginPortion: null, foodCostPm: null, hasRecipe: true, missing: [{ id: 'eel', name: 'Eel' }] },
  { id: 'cola', price: 200, portionCost: null, marginPortion: null, foodCostPm: null, hasRecipe: false, missing: [] },
  { id: 'loss', price: 300, portionCost: 450, marginPortion: -150, foodCostPm: 1500, hasRecipe: true, missing: [] },
];

test('per mille as a percent, the dish editor`s own formatter (ingredients-calc.js)', () => {
  assert.equal(pct(220), '22%');
  assert.equal(pct(337), '33.7%');
  assert.equal(pct(1500), '150%');
  assert.equal(pct(null), '');
});

test('a known cost shows food cost and margin; an unknown one says so, never a zero', () => {
  assert.deepEqual(badge(TODAY[0]), { tone: '', fc: 220, margin: 780 });
  assert.deepEqual(badge(TODAY[1]), { tone: 'warn', key: 'mg_unknown' });
  assert.deepEqual(badge(TODAY[2]), { tone: 'warn', key: 'mg_noRecipe' });
  assert.equal(badge(TODAY[3]).tone, 'bad', 'a portion that costs more than its price');
  assert.equal(badge(undefined), null, 'a dish the answer does not know gets no badge');
});

test('what a cost waits on: each unpriced supply, or no recipe', () => {
  assert.deepEqual(missingOf(TODAY[1]), [{ kind: 'supply', id: 'eel', name: 'Eel' }]);
  assert.deepEqual(missingOf(TODAY[2]), [{ kind: 'recipe' }]);
  assert.deepEqual(missingOf(TODAY[0]), [], 'twin: a priced dish waits on nothing');
});

test('the 30 days: sold, revenue, cost, margin, and the basis of each part of the cost', () => {
  const row = { id: 'sake', sold: 6, revenue: 5000, cogs: 1350, margin: 3650, foodCostPm: 270, costUnknown: false, costBasis: 'mixed',
    stamped: 2, stampedCogs: 600, todayPortions: 4, todayCogs: 750, attributed: 3 };
  const h = history(row);
  assert.deepEqual(h.facts, [['mg_sold', 6], ['mg_revenue', 5000, 'money'], ['mg_cogs', 1350, 'money'], ['mg_margin', 3650, 'money'], ['mg_foodCost', '27%']]);
  assert.deepEqual(h.basis, [['mg_stamped', 2, 600], ['mg_recipeToday', 4, 750]]);
  assert.equal(h.attributed, 3);
  const unknown = history({ id: 'unagi', sold: 2, revenue: 2400, costUnknown: true, stamped: 0, stampedCogs: 0, todayPortions: 2, todayCogs: null, attributed: 0 });
  assert.deepEqual(unknown.facts, [['mg_sold', 2], ['mg_revenue', 2400, 'money'], ['mg_cogs', null], ['mg_margin', null], ['mg_foodCost', null]], 'unknown is not 0');
  assert.deepEqual(unknown.basis, [['mg_recipeToday', 2, null]]);
  assert.equal(history(undefined), null, 'not sold in the window');
});

test('the till codes no dish answers to, by revenue, the first few', () => {
  const menu = { unmatched: [{ code: '4', name: 'Tempura', portions: 4, revenue: 2800 }, { code: '3', name: 'Maki', portions: 1, revenue: 500, why: 'two_dishes' }] };
  assert.deepEqual(unmatchedOf(menu, 1).map(u => u.code), ['4']);
  assert.equal(unmatchedOf(menu).length, 2);
  assert.deepEqual(unmatchedOf({}), []);
});
