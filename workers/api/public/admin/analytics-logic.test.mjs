// node --test public/admin/analytics-logic.test.mjs -- the analytics screen's
// arithmetic (W-HIST): words, ranges, the matrix's sentence, the trace filter.
import test from 'node:test';
import assert from 'node:assert/strict';
import { pct, fill, badRange, query, kitchenQuery, spark, heat, byQuadrant, advice, keep, csv, WINDOWS } from './analytics-logic.js';

test('a change is a signed percent with one decimal, and nothing to compare is empty', () => {
  assert.equal(pct(1500), '+150.0%');
  assert.equal(pct(-125), '-12.5%');
  assert.equal(pct(0), '0.0%');
  assert.equal(pct(null), '');
  assert.equal(pct(undefined), '');
});

test('the windows are the four the hub answers', () => {
  assert.deepEqual(WINDOWS, [7, 30, 90, 365]);
  assert.equal(query({ days: 90 }), 'v=2&days=90');
  assert.equal(query({ from: '2026-01-01', to: '2026-03-31' }), 'v=2&from=2026-01-01&to=2026-03-31');
});

test('a custom period is refused as the hub refuses it; twin: a year and a day passes', () => {
  assert.equal(badRange('2025-09-22', '2026-09-22'), null);
  assert.equal(badRange('2025-09-21', '2026-09-22'), 'anTooLong');
  assert.equal(badRange('2026-09-23', '2026-09-22'), 'anFromAfterTo');
  assert.equal(badRange('2026-9-1', '2026-09-22'), 'anBadDate');
  assert.equal(badRange('', ''), 'anBadDate');
});

test('the matrix is read over the kitchen window, clipped and said so', () => {
  assert.deepEqual(kitchenQuery({ days: 30 }, { days: 30, to: '2026-09-22' }), { query: 'v=2&to=2026-09-22&days=30', clipped: false });
  assert.deepEqual(kitchenQuery({ days: 365 }, { days: 365, to: '2026-09-22' }), { query: 'v=2&to=2026-09-22&days=62', clipped: true });
  assert.deepEqual(kitchenQuery({ days: 7 }, null), { query: 'v=2&days=7', clipped: false });
});

test('templates fill their parameters and leave an unknown one visible', () => {
  assert.equal(fill('raise by {raise} or cut {line}', { raise: '218 L', line: 'Salmon' }), 'raise by 218 L or cut Salmon');
  assert.equal(fill('a {gap}', {}), 'a {gap}');
});

test('each quadrant gets its own sentence, an unknown cost its own, never a made-up cost', () => {
  const m = n => n + ' L';
  assert.deepEqual(advice({ quadrant: 'plowhorse', raiseBy: 218, costliest: { name: 'Salmon', cost: 150 } }, m), { key: 'mmPlowhorseDo', params: { raise: '218 L', line: 'Salmon', cost: '150 L' } });
  assert.equal(advice({ quadrant: 'star' }).key, 'mmStarDo');
  assert.equal(advice({ quadrant: 'puzzle' }).key, 'mmPuzzleDo');
  assert.equal(advice({ quadrant: 'dog', raiseBy: 3 }).key, 'mmDogDo');
  assert.equal(advice({ quadrant: null, costUnknown: true }).key, 'mmUnknownDo');
  const g = byQuadrant({ dishes: [{ id: 'a', quadrant: 'star' }, { id: 'e', quadrant: null }, { id: 'd', quadrant: 'dog' }] });
  assert.deepEqual([g.star.length, g.dog.length, g.unknown.length, g.puzzle.length], [1, 1, 1, 0]);
});

test('a sparkline spans the box with the max at the top, and heat has five levels', () => {
  assert.equal(spark([0, 2, 1], 80, 20), '0,20 40,0 80,10');
  assert.equal(spark([]), '');
  assert.deepEqual([heat(0, 9), heat(1, 9), heat(9, 9), heat(5, 0)], [0, 1, 4, 0]);
});

test('the trace keeps the records of a dish or an hour', () => {
  const rs = [{ id: 'o1', hour: 12, items: [{ id: 'sake' }] }, { id: 'o2', hour: 13, items: [{ id: 'maki' }] }];
  assert.deepEqual(keep(rs, { dish: 'sake' }).map(r => r.id), ['o1']);
  assert.deepEqual(keep(rs, { hour: 13 }).map(r => r.id), ['o2']);
  assert.equal(keep(rs).length, 2);
});

test('the CSV carries each day and where its orders came from', () => {
  assert.equal(csv({ byDay: [{ day: '2026-09-22', orders: 3, revenue: 4500, hot: 1, archived: 2 }] }),
    'day,orders,revenue_minor,hot_orders,archived_orders\n2026-09-22,3,4500,1,2');
});
