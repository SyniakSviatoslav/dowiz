// node --test workers/api/public/admin/kitchen-view.test.mjs
// The kitchen numbers as drawn, from an answer shaped exactly like the hub's
// (`services/analytics/kitchen/report/tests.rs`): every section is there, a
// name cannot inject markup, and an empty window says so.
import test from 'node:test';
import assert from 'node:assert/strict';
import { draw, FOOD_COST_WARN_PM } from './kitchen-view.js';

const fmt = { money: n => `L${n}`, t: k => `[${k}]` };
const answer = {
  from: '2026-09-25', to: '2026-09-26', days: ['2026-09-25', '2026-09-26'],
  totals: { orders: 2, revenue: 3000, cogs: 600, foodCostPm: 200, margin: 2400, wasteValue: 100, receivedValue: 2000, driftValue: -300, undated: 1, unmodelled: 0, uncosted: 0 },
  byDay: [{ day: '2026-09-25', orders: 1, revenue: 2000, cogs: 400, foodCostPm: 200, waste: 0, received: 2000 }, { day: '2026-09-26', orders: 1, revenue: 1000, cogs: 200, foodCostPm: 200, waste: 100, received: 0 }],
  dishes: [{ id: 'sake', name: 'Sake <b>roll</b>', sold: 3, revenue: 3000, portionCost: 200, cogs: 600, margin: 2400, marginPortion: 800, foodCostPm: 500, byDay: [2, 1], hasRecipe: false }],
  ingredients: [{ id: 'salmon', name: 'Salmon', unit: 'g', used: 300, grossG: 300, netG: 165, outG: 165, cleanLossG: 135, cookLossG: 0, cost: 600, drawn: 300, wasted: 50, drift: -100, daysCover: 2, reorder: 850 }],
  waste: [{ reason: 'dropped', rows: 1, value: 100 }],
  yields: [{ item: 'salmon', name: 'Salmon', stage: 'clean', day: '2026-09-26', qty: 5000, out: 2800, measuredPm: 560, expectedPm: 580, diffPm: -20 }],
  prices: [{ id: 'salmon', name: 'Salmon', points: [{ day: '2026-09-25', perBasis: 300, supplier: 'Sea' }, { day: '2026-09-26', perBasis: 330 }], changePm: 100 }],
};

test('every section is drawn, with the numbers the hub sent', () => {
  const html = draw(answer, fmt);
  for (const k of ['ka_byDay', 'ka_byDish', 'ka_ingredients', 'ka_waste', 'ka_yields', 'ka_prices', 'ka_undated']) assert.ok(html.includes(`data-t="${k}"`), k);
  for (const v of ['L3000', 'L600', '20%', 'L2400', 'L-300', '2026-09-25', '135 g', '850 g', '56%', '58%', '10%', 'L330', '[dropped]']) assert.ok(html.includes(v), v);
  assert.ok(html.includes('Sea'), 'the supplier of a price');
  assert.ok(html.includes('inv_noStockLink'), 'a dish without a recipe says it reduces no stock');
  assert.ok(!html.includes('<b>roll</b>') && html.includes('&lt;b&gt;roll'), 'a dish name is text, not markup');
  assert.ok(answer.dishes[0].foodCostPm > FOOD_COST_WARN_PM && html.includes('class="warn"'), 'a dish past the food-cost line warns');
  assert.ok(html.includes('class="neg">2<'), 'two days of cover is flagged');
});

test('a window with nothing in it says so, and still shows the totals', () => {
  const html = draw({ totals: {}, byDay: [{ day: '2026-09-26', orders: 0 }], ingredients: [] }, fmt);
  assert.ok(html.includes('ka_none'));
  assert.ok(html.includes('data-t="ka_revenue"'));
  assert.ok(!html.includes('data-t="ka_byDish"'));
});
