// W-NOM: the nomenclature's pure rules -- quick lines, packs, selection, the
// two-tap arm. Every refusal has its passing twin.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as N from './nom-logic.js';

test('a quick list becomes one item per line, units and groups read, repeats dropped', () => {
  const items = N.parseLines('Salmon; kg; Fish\n\n  rice \nSoy sauce | l\nSALMON\nLids\tpcs\tBoxes\nNori; Dry', { unit: 'g', category: 'Misc', kind: 'food_ingredient' });
  assert.deepEqual(items, [
    { name: 'Salmon', unit: 'g', category: 'Fish', kind: 'food_ingredient' },
    { name: 'rice', unit: 'g', category: 'Misc', kind: 'food_ingredient' },
    { name: 'Soy sauce', unit: 'ml', category: 'Misc', kind: 'food_ingredient' },
    { name: 'Lids', unit: 'unit', category: 'Boxes', kind: 'food_ingredient' },
    { name: 'Nori', unit: 'g', category: 'Dry', kind: 'food_ingredient' },
  ]);
  assert.deepEqual(N.parseLines('Шт товар; шт'), [{ name: 'Шт товар', unit: 'unit' }], 'no defaults: grams, no group, no kind');
  assert.deepEqual(N.parseLines(''), []);
});

test('a list is refused empty, too long or with a long name, and passes at the limits', () => {
  assert.equal(N.listProblem([]), 'nom_needOne');
  const many = n => Array.from({ length: n }, (_, i) => ({ name: 'n' + i }));
  assert.equal(N.listProblem(many(N.ITEMS_MAX + 1)), 'nom_tooMany');
  assert.equal(N.listProblem(many(N.ITEMS_MAX)), null);
  assert.equal(N.listProblem([{ name: 'x'.repeat(N.NAME_MAX + 1) }]), 'nom_nameLong');
  assert.equal(N.listProblem([{ name: 'x'.repeat(N.NAME_MAX) }]), null);
});

test('packs are whole base units; half a row, a repeat or too many is refused', () => {
  assert.deepEqual(N.packsOf([{ name: 'box 5 kg', qty: '5 kg' }, { name: '', qty: '' }, { name: 'kg', qty: '1000' }], 'g'),
    { packs: [{ name: 'box 5 kg', qty: 5000 }, { name: 'kg', qty: 1000 }] });
  assert.deepEqual(N.packsOf([{ name: 'bottle', qty: '0,75 l' }], 'ml'), { packs: [{ name: 'bottle', qty: 750 }] });
  assert.deepEqual(N.packsOf([{ name: 'box', qty: '' }], 'g'), { error: 'nom_packBad' });
  assert.deepEqual(N.packsOf([{ name: '', qty: '5' }], 'g'), { error: 'nom_packBad' });
  assert.deepEqual(N.packsOf([{ name: 'box', qty: '0' }], 'g'), { error: 'nom_packBad' });
  assert.deepEqual(N.packsOf([{ name: 'box', qty: '1 kg' }], 'unit'), { error: 'nom_packBad' }, 'kg of pieces is not a quantity');
  assert.deepEqual(N.packsOf([{ name: 'Box', qty: '1' }, { name: 'box', qty: '2' }], 'g'), { error: 'nom_packTwice' });
  const rows = n => Array.from({ length: n }, (_, i) => ({ name: 'p' + i, qty: '1' }));
  assert.deepEqual(N.packsOf(rows(N.PACKS_MAX + 1), 'g'), { error: 'nom_packMany' });
  assert.equal(N.packsOf(rows(N.PACKS_MAX), 'g').packs.length, N.PACKS_MAX);
});

test('a pack tap adds one pack to what the field holds', () => {
  assert.equal(N.plusPack('', { qty: 5000 }, 'g'), 5000);
  assert.equal(N.plusPack('5000', { qty: 5000 }, 'g'), 10000);
  assert.equal(N.plusPack('1,5 kg', { qty: 1000 }, 'g'), 2500);
  assert.equal(N.plusPack('junk', { qty: 3 }, 'unit'), 3, 'an unreadable field starts from zero');
});

test('selection toggles one id, and all-shown toggles to none', () => {
  let s = N.toggle(new Set(), 'a');
  assert.deepEqual([...s], ['a']);
  assert.deepEqual([...N.toggle(s, 'a')], []);
  s = N.toggleAll(new Set(['x']), ['a', 'b']);
  assert.deepEqual([...s].sort(), ['a', 'b', 'x']);
  assert.deepEqual([...N.toggleAll(s, ['a', 'b'])], ['x'], 'all shown selected -> those shown cleared, others kept');
  assert.deepEqual([...N.toggleAll(new Set(), [])], []);
});

test('the red Delete fires only on a second tap of the same row within the window', () => {
  const first = N.tap(null, 'salmon', 1000);
  assert.deepEqual(first, { fire: false, arm: { id: 'salmon', at: 1000 } });
  assert.equal(N.tap(first.arm, 'salmon', 1000 + N.ARM_MS).fire, true);
  assert.equal(N.tap(first.arm, 'salmon', 1001 + N.ARM_MS).fire, false, 'too late: it re-arms');
  assert.deepEqual(N.tap(first.arm, 'rice', 1500), { fire: false, arm: { id: 'rice', at: 1500 } }, 'another row arms itself');
});

test('the delete answer reads as one line', () => {
  const t = k => ({ nom_deleted: 'Deleted', nom_recipesChanged: 'recipes changed' })[k];
  assert.equal(N.deletedLine({ deleted: ['a', 'b'], dishes: ['roll'] }, t), 'Deleted: 2 · recipes changed: 1');
  assert.equal(N.deletedLine({ deleted: [], stock: ['a'] }, t), 'Deleted: 1', 'a retry that only cleared the shelf');
  assert.equal(N.deletedLine({ deleted: ['d1'] }, t), 'Deleted: 1');
});

test('the edges: nothing typed, nothing answered, a negative field', () => {
  assert.deepEqual(N.parseLines(null), []);
  assert.deepEqual(N.parseLines('Tofu;;'), [{ name: 'Tofu', unit: 'g' }], 'empty parts change nothing');
  assert.deepEqual(N.packsOf(null, 'g'), { packs: [] });
  assert.deepEqual(N.packsOf([{ name: 'x'.repeat(N.PACK_NAME_MAX + 1), qty: '1' }], 'g'), { error: 'nom_packBad' });
  assert.equal(N.plusPack('-5', { qty: 2 }, 'g'), 2, 'a negative field starts from zero');
  assert.equal(N.deletedLine(null, k => k), 'nom_deleted: 0');
});
