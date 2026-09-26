// node --test workers/api/public/admin/ingredients-calc.test.mjs
// The ingredients screens' arithmetic, pinned. The weights test mirrors the
// hub's own (`workers/api/src/recipe/weights/tests.rs`) number for number, so
// the editor draws what the hub will store.
import test from 'node:test';
import assert from 'node:assert/strict';
import * as C from './ingredients-calc.js';

test('an amount is whole base units: kg and l scale, a fraction of a gram is refused', () => {
  assert.equal(C.amount('1500'), 1500);
  assert.equal(C.amount('1 500'), 1500);
  assert.equal(C.amount('1,5 kg'), 1500);
  assert.equal(C.amount('0.25 l', 'ml'), 250);
  assert.equal(C.amount('2', 'unit'), 2);
  for (const bad of ['', '  ', 'abc', '1.5', '1,2345 kg', '2 kg x', '1e3']) assert.equal(C.amount(bad), null, bad);
  assert.equal(C.amount('1 kg', 'unit'), null, 'a piece is not weighed in kg');
  assert.equal(C.amount('-5'), -5, 'the sign is the caller\'s to refuse');
});

test('salmon is cleaned and rice grows (R6, as the hub computes it)', () => {
  const salmon = { unit: 'g', cleanPm: 550 };
  assert.deepEqual(C.weights(salmon, 100), { gross: 100, net: 55, out: 55, cleanPm: 550, cookPm: 1000 });
  assert.equal(C.lossPm(C.weights(salmon, 100)), 450);
  const rice = { unit: 'g', cookPm: 2200 };
  assert.equal(C.weights(rice, 100).out, 220);
  assert.equal(C.lossPm(C.weights(rice, 100)), -1200);
  const w = C.weights({ unit: 'g', cleanPm: 550, cookPm: 900 }, 100, 60, null);
  assert.deepEqual([w.net, w.out], [60, 54], 'a typed net wins and the out follows it');
  assert.deepEqual(C.weights({ unit: 'unit', weightPerUnit: 60, cleanPm: 880 }, 2), { gross: 120, net: 106, out: 106, cleanPm: 880, cookPm: 1000 });
  assert.equal(C.weights({ unit: 'unit' }, 2).out, null, 'a piece with no weight has none');
  assert.equal(C.pmOf({ cleanPm: 1001 }, 'cleanPm', C.CLEAN_MAX), 1000);
});

test('percent, cost, price and yield', () => {
  assert.equal(C.pmOfPct('55'), 550);
  assert.equal(C.pmOfPct('55,5'), 555);
  assert.equal(C.pmOfPct('220'), 2200);
  for (const bad of ['', '5.55', '-5', 'x']) assert.equal(C.pmOfPct(bad), null, bad);
  assert.equal(C.pct(450), '45%');
  assert.equal(C.pct(125), '12.5%');
  assert.equal(C.pct(-1200), '-120%');
  assert.equal(C.pct(null), '');
  assert.equal(C.costOf(300, 150, 100), 450);
  assert.equal(C.costOf(1, 1, 2), 1, 'half up');
  assert.equal(C.costOf(null, 5, 100), null);
  assert.deepEqual(C.priceOf({ wac: 320, costPerBasis: 250 }), { perBasis: 320, from: 'wac' });
  assert.deepEqual(C.priceOf({ costPerBasis: 250 }), { perBasis: 250, from: 'list' });
  assert.deepEqual(C.priceOf({}), { perBasis: null, from: null });
  assert.deepEqual(C.priceBody('g', 1200, 'per'), { unitCost: 1200, per: 1000 });
  assert.deepEqual(C.priceBody('unit', 150, 'per'), { unitCost: 150, per: 1 });
  assert.deepEqual(C.priceBody('g', 9000, 'total'), { total: 9000 });
  assert.deepEqual(C.priceBody('g', null, 'total'), {});
  assert.equal(C.yieldPm(5000, 2750), 550);
  assert.equal(C.yieldPm(0, 5), null);
});

test('dates, bars, levels and the dishes that reduce no stock', () => {
  assert.deepEqual([-1, 0, 2, 3, null].map(d => C.expiryTone(d, 2)), ['bad', 'warn', 'warn', 'ok', null]);
  assert.deepEqual(C.bars([0, 50, 100, -25]), [0, 50, 100, 25]);
  assert.deepEqual(C.bars([0, 0]), [0, 0]);
  assert.equal(C.levelState({ counted: false, onHand: -180 }), 'needsCount', 'I0: an uncounted negative is a count due');
  assert.equal(C.levelState({ counted: false, onHand: 0 }), 'uncounted');
  assert.equal(C.levelState({ counted: true, available: 0 }), 'out');
  assert.equal(C.levelState({ counted: true, available: 5, low: true }), 'low');
  assert.equal(C.levelState({ counted: true, available: 5 }), 'ok');
  const g = C.byCategory([{ id: 'b', name: 'Beer', categoryId: 'drinks' }, { id: 'a', name: 'Ayran', categoryId: 'drinks' }, { id: 'x', name: 'X' }]);
  assert.deepEqual(g.map(([c, ds]) => [c, ds.map(d => d.id)]), [['drinks', ['a', 'b']], ['', ['x']]]);
  assert.equal(C.basisOf('unit'), 1);
  assert.equal(C.basisOf('g'), 100);
});
