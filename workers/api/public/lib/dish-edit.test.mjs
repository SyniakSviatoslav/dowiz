// node workers/api/public/lib/dish-edit.test.mjs
import assert from 'node:assert/strict';
import { publishedFields, FIELDS } from './dish-edit.js';

let run = 0;
const test = (name, fn) => { fn(); run++; console.log(`  ok  ${name}`); };

const prefilled = { ings: 'Rice, Salmon', kcal: '190', protein: '9', fat: '5', carbs: '25', weight: '125' };

test('D19: an untouched sheet sends no published value, so the recipe decides', () => {
  assert.deepEqual(publishedFields(prefilled, new Set()), {});
});

test('an edited box is sent, and marks only its own group as typed', () => {
  assert.deepEqual(publishedFields({ ...prefilled, ings: 'Rice, Salmon, Shrimp' }, new Set(['ings'])),
    { ingredients: ['Rice', 'Salmon', 'Shrimp'] });
  assert.deepEqual(publishedFields({ ...prefilled, kcal: '210' }, new Set(['kcal'])),
    { nutrition: { kcal: 210, protein: 9, fat: 5, carbs: 25 } }, 'the whole panel, as it stands');
  assert.deepEqual(publishedFields({ ...prefilled, weight: '140,5' }, new Set(['weight'])), { weight_g: 140.5 });
});

test('an emptied box sends nothing for it rather than a zero', () => {
  assert.deepEqual(publishedFields({ ...prefilled, weight: '' }, new Set(['weight'])), {});
  assert.deepEqual(publishedFields({ ...prefilled, ings: ' , ' }, new Set(['ings'])), { ingredients: [] });
});

test('every tracked field is one the function reads', () => {
  const all = publishedFields(prefilled, new Set(FIELDS));
  assert.deepEqual(Object.keys(all).sort(), ['ingredients', 'nutrition', 'weight_g']);
});

console.log(`dish-edit: ${run} tests, all green`);
