// `node --test workers/api/public/admin/menu-recipe-logic.test.mjs` (W-TKUI, TZ 4 and 6).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { bookList, picks, setPick, pickedOf, mergeSaved } from './menu-recipe-logic.js';

const BOOK = [{ id: 'tuna', name: 'Tuna', category: 'Fish', kind: 'food_ingredient' }, { id: 'nori', name: 'Nori', category: 'Dry', kind: 'food_ingredient' },
  { id: 'box', name: 'Box', category: '', kind: 'packaging' }, { id: 'salmon', name: 'Salmon', category: 'Fish', kind: 'food_ingredient' }];

test('bookList: search by name or category, by kind, never a line already on the recipe', () => {
  assert.deepEqual(bookList(BOOK).map(s => s.id), ['tuna', 'nori', 'box', 'salmon']);
  assert.deepEqual(bookList(BOOK, { q: 'fish' }).map(s => s.id), ['tuna', 'salmon']);
  assert.deepEqual(bookList(BOOK, { q: ' SAL ' }).map(s => s.id), ['salmon']);
  assert.deepEqual(bookList(BOOK, { kind: 'packaging' }).map(s => s.id), ['box']);
  assert.deepEqual(bookList(BOOK, { draft: [{ supply: 'tuna' }] }).map(s => s.id), ['nori', 'box', 'salmon']);
  assert.deepEqual(bookList(null), []);
});

test('TZ4: ticks live outside the list -- a search typed and cleared changes nothing in them', () => {
  const p = picks();
  setPick(p, 'nori', true);
  bookList(BOOK, { q: 'sal' }); bookList(BOOK, { q: '' }); // the list redrawn twice
  assert.ok(p.has('nori'), 'the tick survived');
  assert.deepEqual(pickedOf(BOOK, p).map(s => s.id), ['nori']);
  setPick(p, 'salmon', true); setPick(p, 'nori', false);
  assert.deepEqual(pickedOf(BOOK, p).map(s => s.id), ['salmon'], 'an untick is honoured');
});

test('pickedOf: hidden by the search still counts; already on the recipe never does; book order', () => {
  const p = picks(); setPick(p, 'salmon', true); setPick(p, 'tuna', true); setPick(p, 'ghost', true);
  assert.deepEqual(pickedOf(BOOK, p).map(s => s.id), ['tuna', 'salmon'], 'an id the book does not know is not invented');
  assert.deepEqual(pickedOf(BOOK, p, [{ supply: 'tuna' }]).map(s => s.id), ['salmon']);
});

test('TZ6 mergeSaved: the saved dish replaces its row in place and keeps what the answer lacks', () => {
  const list = [{ id: 'd1', name: 'Tuna roll', price: 1000, categoryId: 'c1', categoryName: 'Rolls', translations: { uk: { name: 'Тунець' } } }, { id: 'd2', name: 'B', price: 5 }];
  assert.equal(mergeSaved(list, { id: 'd1', name: 'Tuna roll XL', price: 1200, available: false }), true);
  assert.deepEqual(list[0], { id: 'd1', name: 'Tuna roll XL', price: 1200, available: false, categoryId: 'c1', categoryName: 'Rolls', translations: { uk: { name: 'Тунець' } } });
  assert.deepEqual(list[1], { id: 'd2', name: 'B', price: 5 }, 'the other row untouched');
  assert.equal(mergeSaved(list, { id: 'd9', name: 'new' }), false, 'a dish the list does not know asks for a full reload');
  assert.equal(mergeSaved(list, null), false);
  assert.equal(list.length, 2);
});
