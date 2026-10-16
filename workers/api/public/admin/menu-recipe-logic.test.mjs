// `node --test workers/api/public/admin/menu-recipe-logic.test.mjs` (W-TKUI, TZ 4 and 6).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { bookList, picks, setPick, pickedOf, mergeSaved, starterMilli, withGhosts } from './menu-recipe-logic.js';
import * as RL from './menu-recipe-logic.js';

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

test('TZ1: a picked component starts at 100 g/ml or 1 piece, in milli', () => {
  assert.equal(starterMilli({ unit: 'g' }), 100000, '100 g, not 0.1 g');
  assert.equal(starterMilli({ unit: 'ml' }), 100000);
  assert.equal(starterMilli({ unit: 'unit' }), 1000, '1 piece, not 0.001');
});

test('TZ3 ghost: a deleted ingredient the hub dropped from the bom is drawn as a removed line from recipeReview', () => {
  const bom = [{ supply: 'rice', qty: 100 }];
  const review = [{ supply: 'nori', name: 'Nori', qty: 2.5 }, { supply: 'salt', name: 'Salt', qty: 1, via: 'pf-sauce' }, { supply: 'cheese', name: 'Cheese', qty: 10, option: 'extra' }];
  const lines = withGhosts(bom, review);
  assert.deepEqual(lines, [{ supply: 'rice', qty: 100 }, { supply: 'nori', name: 'Nori', qty: 2.5, removed: true }], 'the direct mark becomes a ghost line after the real ones; via a ПФ or an option is not a line of this recipe');
  assert.deepEqual(withGhosts(bom, []), bom, 'no marks, no ghosts');
  assert.deepEqual(withGhosts(bom, null), bom);
  assert.deepEqual(withGhosts([{ supply: 'nori', qty: 2, removed: true }], [{ supply: 'nori', name: 'Nori', qty: 2 }]).length, 1, 'a line already there is not doubled');
  assert.deepEqual(withGhosts([], [{ supply: 'nori', name: 'Nori', qty: 2 }, { supply: 'nori', name: 'Nori', qty: 2 }]).length, 1, 'one ghost per supply');
});

// TZ 2026-10-16 "no ingredient lost": the lines a save would drop are named, not dropped.
test('unsavable: unreadable or 0 lines are named; a removed ghost line is not', () => {
  const draft = [
    { supply: 'rice', name: 'Rice', milli: 120000 },
    { supply: 'nori', name: 'Nori', milli: 0 },
    { supply: 'spice', name: 'Spice', milli: 500, bad: true, typed: '0.0005' },
    { supply: 'gone', name: 'Gone', milli: 0, removed: true },
  ];
  assert.deepEqual(RL.unsavable(draft).map(l => l.supply), ['nori', 'spice']);
  assert.deepEqual(RL.unsavable([{ supply: 'a', milli: 1 }]), []);
});

test('BOOK_MAX lets a kitchen-sized book be listed whole', () => {
  assert.ok(RL.BOOK_MAX >= 300);
});
