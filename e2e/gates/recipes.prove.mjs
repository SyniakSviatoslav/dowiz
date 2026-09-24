// THE REPORT'S OWN PROOF, offline: `coverage` over stub reads, each count
// moved by exactly the case that should move it.   node e2e/gates/recipes.prove.mjs
import assert from 'node:assert/strict';
import { coverage, line } from './recipes.mjs';

const supplies = [
  { id: 'rice', kind: 'food_ingredient', kcalPer100: 130 },
  { id: 'salmon', kind: 'food_ingredient', kcalPer100: 208 },
  { id: 'nori', kind: 'food_ingredient', kcalPer100: null },
  { id: 'box', kind: 'packaging', kcalPer100: null },
];
const dish = (id, bom) => ({ id, bom });
const products = [
  dish('maki-salmon', [{ supply: 'rice', qty: 90 }, { supply: 'salmon', qty: 35 }, { supply: 'box', qty: 1 }]),
  dish('maki-nori', [{ supply: 'rice', qty: 90 }, { supply: 'nori', qty: 2 }]),
  dish('box-only', [{ supply: 'box', qty: 1 }]),
  dish('no-recipe', null),
  dish('empty-recipe', []),
];
const c = coverage(products, supplies);
// Packaging never makes a dish incomplete; a missing kcal on food does; a
// dish with only packaging has no food to be complete about; null and [] are
// both "no recipe".
assert.deepEqual(c, { dishes: 5, withRecipe: 3, supplies: 4, kcalComplete: 1 });
assert.equal(line(c), 'dishes 5 · with recipe 3 · supplies 4 · dishes whose every food line has kcal 1');
// TWIN: once nori has a kcal figure, its dish counts.
supplies[2].kcalPer100 = 35;
assert.equal(coverage(products, supplies).kcalComplete, 2);
// Nothing at all is zeros, not a crash.
assert.deepEqual(coverage([], []), { dishes: 0, withRecipe: 0, supplies: 0, kcalComplete: 0 });
console.log('recipes.prove: 4 cases hold');
