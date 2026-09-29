// node --test workers/api/public/admin/prep-logic.test.mjs
// The operator's example through the editor's live arithmetic (the hub's
// numbers, `recipe/prep.rs`), the typed lines checked, the request body,
// the where-used sentence, three decimals of a leaf. Every refusal beside
// its twin.
import test from 'node:test';
import assert from 'node:assert/strict';
import * as P from './prep-logic.js';

const raw = (id, unit, costPerBasis, more = {}) => ({ id, name: id, unit, kind: 'food_ingredient', costPerBasis, ...more });
const vinegar = raw('vinegar', 'ml', 15), salt = raw('salt', 'g', 5), sugar = raw('sugar', 'g', 12), rice = raw('rice-dry', 'g', 20);
const water = raw('water', 'ml', null, { untracked: true });
const mitsukan = { id: 'mitsukan', name: 'Mitsukan', unit: 'g', kind: 'prep', costPerBasis: 14 };
const t = k => `[${k}]`;

test('K and the batch cost follow the operatorʼs example, and a piece without a weight leaves K unknown', () => {
  const m = [{ sup: vinegar, qty: 800 }, { sup: salt, qty: 50 }, { sup: sugar, qty: 150 }];
  assert.equal(P.kPm(m, 1000, 'g'), 1000, '1000 / (800 + 50 + 150)');
  assert.equal(P.batchCost(m), 120 + 3 + 18, '800 ml at 15/100 + 50 g at 5/100 (2.5 -> 3) + 150 g at 12/100');
  assert.equal(P.costPer(141, 1000, 'g'), 141, 'per kg');
  const r = [{ sup: rice, qty: 1000 }, { sup: water, qty: 1100 }, { sup: mitsukan, qty: 250 }];
  assert.equal(P.kPm(r, 2100, 'g'), 894, '2100 / 2350');
  assert.equal(P.batchCost(r), 200 + 35, 'water is free and untracked; mitsukan at its derived 14/100');
  assert.equal(P.costPer(235, 2100, 'g'), 112, '111.9 per kg, half up');
  assert.equal(P.costPer(235, 2100, 'ml'), 112);
  assert.equal(P.costPer(50, 25, 'unit'), 2, 'per piece');
  assert.equal(P.perWord('ml'), 'pf_costPerL');
  const egg = raw('egg', 'unit', 25);
  assert.equal(P.kPm([{ sup: egg, qty: 2 }, { sup: vinegar, qty: 10 }], 100, 'g'), null, 'unknown, never refused');
  assert.equal(P.kPm([{ sup: { ...egg, weightPerUnit: 55 }, qty: 2 }, { sup: vinegar, qty: 10 }], 100, 'g'), 833, '100 / 120');
  assert.equal(P.kPm(r, 0, 'g'), null);
  assert.equal(P.kPm([], 10, 'g'), null);
  assert.equal(P.batchCost([{ sup: raw('x', 'g', null), qty: 1 }]), null, 'a component without a price: no cost, not a partial one');
  assert.equal(P.costPer(null, 100, 'g'), null);
  assert.equal(P.lineCost(mitsukan, 250), 35);
});

test('typed lines are whole base units in range, distinct, one to forty; the yield likewise', () => {
  assert.deepEqual(P.readLines([{ item: 'rice-dry', qty: '1 kg', unit: 'g' }, { item: 'water', qty: '1,1 l', unit: 'ml' }, { item: '', qty: '9' }]),
    { lines: [{ item: 'rice-dry', qty: 1000 }, { item: 'water', qty: 1100 }] });
  assert.deepEqual(P.readLines([{ item: 'salt', qty: '0.5', unit: 'g' }]), { error: 'pf_qtyBad' });
  assert.deepEqual(P.readLines([{ item: 'salt', qty: '0', unit: 'g' }]), { error: 'pf_qtyBad' });
  assert.deepEqual(P.readLines([{ item: 'salt', qty: String(P.QTY_MAX + 1), unit: 'g' }]), { error: 'pf_qtyBad' });
  assert.deepEqual(P.readLines([{ item: 'salt', qty: String(P.QTY_MAX), unit: 'g' }]), { lines: [{ item: 'salt', qty: P.QTY_MAX }] });
  assert.deepEqual(P.readLines([{ item: 'salt', qty: '1', unit: 'g' }, { item: 'salt', qty: '2', unit: 'g' }]), { error: 'nom_packTwice' });
  assert.deepEqual(P.readLines([]), { error: 'pf_noLines' });
  const many = Array.from({ length: P.LINES_MAX + 1 }, (_, i) => ({ item: 'i' + i, qty: '1', unit: 'g' }));
  assert.deepEqual(P.readLines(many), { error: 'nom_tooMany' });
  assert.equal(P.readLines(many.slice(0, P.LINES_MAX)).lines.length, P.LINES_MAX);
  assert.equal(P.readYield('2,1 kg', 'g'), 2100);
  assert.equal(P.readYield('0', 'g'), null);
  assert.equal(P.readYield('12', 'unit'), 12);
  assert.equal(P.readYield('1 kg', 'unit'), null, 'a piece has no kilograms');
});

test('the body names the card as the hub takes it; the id follows the name', () => {
  const b = P.body({ id: 'rice-seasoned', name: 'Rice seasoned', unit: 'g', category: 'Prep', lines: [{ item: 'rice-dry', qty: 1000 }], yield: 2100 });
  assert.deepEqual(b, { id: 'rice-seasoned', name: 'Rice seasoned', unit: 'g', category: 'Prep', lines: [{ item: 'rice-dry', qty: 1000 }], yield: 2100 });
  assert.equal(P.body({ id: 'ball', unit: 'unit', lines: [], yield: 50, weightPerUnit: 20 }).weightPerUnit, 20);
  assert.equal('weightPerUnit' in P.body({ id: 'x', unit: 'g', lines: [], yield: 1, weightPerUnit: 20 }), false, 'a weight per piece only for pieces');
  // The supply form's own rule (ingredients.js): NFD strips the breve of a Cyrillic й too, as it always has.
  assert.equal(P.idOf('Рис заправлений  готовий'), 'рис-заправлении-готовии');
  assert.equal(P.idOf('Mitsukan dressing!'), 'mitsukan-dressing');
});

test('where used reads as one line, from one answer or many', () => {
  const uses = { preps: [{ id: 'mitsukan', name: 'Mitsukan' }, { id: 'rs', name: 'Rice seasoned' }], dishes: [{ id: 'p', name: 'Philadelphia' }] };
  assert.equal(P.usesLine(uses, t), '2 [pf_preps]: Mitsukan, Rice seasoned · 1 [pf_dishes]: Philadelphia');
  assert.equal(P.usesLine({ salt: uses, sugar: { preps: [{ id: 'mitsukan', name: 'Mitsukan' }], dishes: [] } }, t), '2 [pf_preps]: Mitsukan, Rice seasoned · 1 [pf_dishes]: Philadelphia', 'a map, de-duplicated');
  assert.equal(P.usesLine({}, t), '');
  assert.equal(P.usesLine(null, t), '');
  assert.equal(P.isUsed(uses), true);
  assert.equal(P.isUsed({ preps: [], dishes: [] }), false);
  const many = { preps: [], dishes: Array.from({ length: 8 }, (_, i) => ({ id: 'd' + i, name: 'D' + i })) };
  assert.ok(P.usesLine(many, t).endsWith('D5 …'), 'six named, the rest an ellipsis');
});

test('a leaf shows three decimals, rounded once', () => {
  assert.equal(P.qty3(773810), '0.774');
  assert.equal(P.qty3(61904762), '61.905');
  assert.equal(P.qty3(12380952), '12.381');
  assert.equal(P.qty3(999600), '1.000', 'carries into the whole part');
  assert.equal(P.qty3(0), '0.000');
});

test('the picker offers raw items and OTHER semi-finished products, not what is on the card', () => {
  const all = [salt, mitsukan, { ...rice, active: false }, { id: 'rs', name: 'Rice seasoned', unit: 'g', kind: 'prep' }, vinegar];
  assert.deepEqual(P.pickable(all, '', new Set(['salt']), 'rs').map(s => s.id), ['mitsukan', 'vinegar'], 'preps first, then by name; not itself, not on the card, not retired');
  assert.deepEqual(P.pickable(all, 'mits').map(s => s.id), ['mitsukan']);
  assert.deepEqual(P.pickable([], 'x'), []);
  assert.equal(P.gramsOf({ unit: 'ml' }, 30), 30);
  assert.equal(P.gramsOf({ unit: 'unit' }, 3), null);
  assert.equal(P.gramsOf({ unit: 'unit', weightPerUnit: 12.4 }, 3), 37);
});
