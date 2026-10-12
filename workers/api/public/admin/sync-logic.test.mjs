// `node --test workers/api/public/admin/sync-logic.test.mjs` (W-TKUI, TZ 6).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { findDish, foldMenu } from './sync-logic.js';

const state = () => {
  const c1 = { id: 'c1', name: 'Rolls', products: [{ id: 'd1', name: 'Tuna roll', price: 1000 }, { id: 'd2', name: 'Rice roll', price: 900 }] };
  const c2 = { id: 'c2', name: 'Soups', products: [{ id: 'd3', name: 'Miso', price: 500 }] };
  const S = { categories: [c1, c2], products: [] };
  S.products = S.categories.flatMap(c => c.products.map(p => ({ ...p, categoryId: c.id, categoryName: c.name, translations: { en: { name: p.name, description: '' }, uk: { name: p.name + ' uk', description: '' } } })));
  return S;
};

test('foldMenu: the dish category takes the answer rows; S.products is rebuilt in place with the words kept', () => {
  const S = state(); const products = S.products;
  const ok = foldMenu(S, { categories: [{ id: 'c1', name: 'Rolls', products: [{ id: 'd2', name: 'Rice roll', price: 900 }, { id: 'd1', name: 'Tuna roll XL', price: 1200, available: false }] }] }, 'd1', 'en', { sq: { name: 'Ton XL', description: '' } });
  assert.equal(ok, true);
  assert.equal(S.products, products, 'the same array: every screen holding it sees the change');
  assert.deepEqual(S.categories[0].products.map(p => p.id), ['d2', 'd1'], 'the reorder landed');
  const d1 = S.products.find(p => p.id === 'd1');
  assert.equal(d1.price, 1200); assert.equal(d1.available, false); assert.equal(d1.categoryName, 'Rolls');
  assert.deepEqual(d1.translations, { en: { name: 'Tuna roll XL', description: '' }, uk: { name: 'Tuna roll uk', description: '' }, sq: { name: 'Ton XL', description: '' } }, 'console language from the answer, the rest from memory and the save');
  assert.equal(S.products.find(p => p.id === 'd3').translations.uk.name, 'Miso uk', 'the other category untouched');
  assert.equal(S.products.length, 3);
});

test('foldMenu: a dish moved to another category leaves the old one; a dish the answer lacks asks for the full reload', () => {
  const S = state();
  assert.equal(foldMenu(S, { categories: [{ id: 'c2', name: 'Soups', products: [{ id: 'd3', name: 'Miso', price: 500 }, { id: 'd1', name: 'Tuna roll', price: 1000 }] }] }, 'd1', 'en'), true);
  assert.deepEqual(S.categories.map(c => c.products.map(p => p.id)), [['d2'], ['d3', 'd1']]);
  assert.equal(S.products.find(p => p.id === 'd1').categoryId, 'c2');
  assert.equal(foldMenu(S, { categories: [] }, 'd1', 'en'), false);
  assert.equal(foldMenu(S, null, 'd1', 'en'), false);
  assert.equal(findDish({ categories: [{ id: 'x', products: [{ id: 'q' }] }] }, 'q')[1].id, 'x');
});
