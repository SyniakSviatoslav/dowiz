// node --test workers/api/public/admin/supply-resolve.test.mjs
// A typed name resolved to a position: the fold (the literal cases the Worker
// reads too), the match, the question, the new position.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import * as R from './supply-resolve.js';
import { bookList } from './menu-recipe-logic.js';
import { match as matchLine } from './receipt-photo-logic.js';

const CASES = JSON.parse(readFileSync(new URL('./supply-resolve-cases.json', import.meta.url), 'utf8'));
const sup = (name, more = {}) => ({ id: name.toLowerCase().replace(/\s+/g, '-'), name, unit: 'g', kind: 'food_ingredient', category: '', ...more });

test('the fold: every literal case, the pairs that are one name and the pairs that are not', () => {
  assert.ok(CASES.fold.length >= 20 && CASES.same.length >= 5 && CASES.differ.length >= 5, 'the list is read, not empty');
  for (const [typed, want] of CASES.fold) assert.equal(R.fold(typed), want, typed);
  for (const [a, b] of CASES.same) assert.equal(R.fold(a), R.fold(b), `${a} = ${b}`);
  for (const [a, b] of CASES.differ) assert.notEqual(R.fold(a), R.fold(b), `${a} != ${b}`);
  assert.equal(R.fold('ёлка'), R.fold('ёлка'), 'a letter typed as base + combining mark folds as the composed one');
  assert.equal(R.fold(null), '');
});

test('match: the fold equals one name or id, no prompt; a retired supply is not a candidate', () => {
  const eggs = sup('Яйця', { id: 'eggs', unit: 'unit' }), soy = sup('Соус соєвий', { id: 'soy-sauce', unit: 'ml' });
  const all = [eggs, soy, sup('Рис')];
  assert.equal(R.resolve('яйця', all).match, eggs);
  assert.equal(R.resolve('  ЯЙЦЯ ', all).match, eggs);
  assert.equal(R.resolve('соус соевый', all).match, soy, 'the Russian spelling of the same words is the same name');
  assert.equal(R.resolve('Soy sauce', all).match, soy, 'the id is a name too');
  assert.deepEqual(R.resolve('', all), { none: true, empty: true });
  assert.deepEqual(R.resolve(' - ', all), { none: true, empty: true });
  const gone = [{ ...eggs, active: false }];
  assert.deepEqual(R.resolve('яйця', gone), { none: true }, 'retired: not offered, not matched');
  const twins = [sup('Сир', { id: 'a' }), sup('сыр', { id: 'b' })];
  assert.deepEqual(R.resolve('сир', twins).near.map(s => s.id), ['a', 'b'], 'two supplies under one fold are a question, never a silent pick');
});

test('near: the literal shapes that must be a question, each way round', () => {
  assert.ok(CASES.near.length >= 12);
  for (const [typed, name] of CASES.near) {
    const s = sup(name), r = R.resolve(typed, [sup('Лимон'), s, sup('Сало')]);
    assert.deepEqual(r.near, [s], `${typed} ~ ${name}: ${JSON.stringify(r)}`);
    assert.ok(!r.match && !r.none, `${typed}: never a match, never new`);
  }
});

test('apart: the literal negatives stay new positions', () => {
  assert.ok(CASES.apart.length >= 10);
  for (const [typed, name] of CASES.apart) assert.deepEqual(R.resolve(typed, [sup(name)]), { none: true }, `${typed} x ${name}`);
  assert.equal(R.nearWord('рис', 'сир'), false);
  assert.equal(R.nearWord('сил', 'сало'), false);
  assert.equal(R.nearWord('яице', 'яицо'), true, 'twin: the same rule does answer yes');
});

test('a differing number is never near; one name without a number may be the one with it', () => {
  const cups = [sup('Соус 50 мл', { id: 'c50' }), sup('Соус 100 мл', { id: 'c100' })];
  assert.equal(R.resolve('соус 50мл', cups).match.id, 'c50');
  assert.deepEqual(R.resolve('соус 30 мл', cups), { none: true });
  assert.deepEqual(R.resolve('соус', cups).near.map(s => s.id), ['c50', 'c100']);
});

test('finder and suggest: a search by another spelling finds the position; the best first', () => {
  const all = [sup('Яйця', { category: 'Продукти' }), sup('Палички', { kind: 'utensil', category: 'Приладдя' }), sup('Рис'), sup('Рисовий оцет'), sup('Сир')];
  const names = q => all.filter(R.finder(q)).map(s => s.name);
  assert.deepEqual(names('яйцо'), ['Яйця']);
  assert.deepEqual(names('палочки'), ['Палички']);
  assert.deepEqual(names('приладдя'), ['Палички'], 'the category is searched too');
  assert.deepEqual(names('рис'), ['Рис', 'Рисовий оцет'], 'cheese is not rice');
  assert.equal(names('').length, all.length, 'no query: everything');
  assert.deepEqual(R.suggest('рис', all).map(s => s.name), ['Рис', 'Рисовий оцет']);
  assert.deepEqual(R.suggest('оцет', all).map(s => s.name), ['Рисовий оцет']);
  assert.deepEqual(R.suggest('', all), []);
  assert.equal(R.suggest('с', Array.from({ length: 30 }, (_, i) => sup('Соус ' + i, { id: 's' + i }))).length, R.NEAR_MAX);
});

test('categories: a typed one whose fold exists is saved under the existing spelling', () => {
  const known = [...new Set(CASES.category.map(c => c[1]))];
  for (const [typed, want] of CASES.category) assert.equal(R.canonCategory(typed, [want]), want, typed);
  for (const [a, b] of CASES.categoryApart) assert.equal(R.canonCategory(a, [b]), a, `${a} stays its own group beside ${b}`);
  assert.equal(R.canonCategory('  Одноразовий посуд ', known), 'Одноразовий посуд', 'a new one is kept as typed, trimmed');
  assert.equal(R.canonCategory('', known), '');
  assert.deepEqual(R.categoriesOf([sup('a', { category: 'Овочі' }), sup('b', { category: 'овочи' }), sup('c', { category: '' }), sup('d', { category: 'Упаковка' })]), ['Овочі', 'Упаковка']);
});

test('every picker finds by the fold: the recipe picker (with its kind filter) and the invoice line match', () => {
  const book = [sup('Яйця', { id: 'eggs' }), sup('Судочок 500 мл', { id: 'tub', kind: 'packaging', unit: 'unit', category: 'Упаковка' }), sup('Рис', { id: 'rice' }), sup('Сир', { id: 'cheese' })];
  const ids = o => bookList(book, o).map(s => s.id);
  assert.deepEqual(ids({ q: 'яйцо' }), ['eggs'], 'another spelling');
  assert.deepEqual(ids({ q: 'судочки', kind: 'packaging' }), ['tub'], 'a typed container, under its kind');
  assert.deepEqual(ids({ q: 'судочки', kind: 'food_ingredient' }), [], 'the kind filter still filters');
  assert.deepEqual(ids({ kind: 'packaging' }), ['tub']);
  assert.deepEqual(ids({ q: 'упаковка' }), ['tub'], 'by category, as before');
  assert.deepEqual(ids({ q: 'рис' }), ['rice'], 'cheese is not rice');
  assert.deepEqual(ids({ q: 'яйця', draft: [{ supply: 'eggs' }] }), [], 'what the recipe has is not offered again');
  assert.deepEqual(ids({}), ['eggs', 'tub', 'rice', 'cheese'], 'no query: the book in its order');
  const line = text => ({ text, key: text.toLowerCase() });
  assert.deepEqual(matchLine(line('Яйцо куриное С0'), {}, book), { item: 'eggs', by: 'name' }, 'the supplier writes it the Russian way');
  assert.deepEqual(matchLine(line('Судочки 500мл'), {}, book), { item: 'tub', by: 'name' });
  assert.deepEqual(matchLine(line('Сир твердий'), {}, book), { item: 'cheese', by: 'name' }, 'cheese, not rice');
  assert.deepEqual(matchLine(line('Лосось'), {}, book), { item: '', by: '' });
  assert.deepEqual(matchLine(line('whatever'), { whatever: 'rice' }, book), { item: 'rice', by: 'alias' }, 'the remembered alias still wins');
});

test('linear in the supplies: one resolve reads each name a bounded number of times, at 500 and at 1000', () => {
  const build = n => {
    let reads = 0;
    const list = Array.from({ length: n }, (_, i) => new Proxy(sup(`Позиція ${i} соус`, { id: 'p' + i }), { get(o, k){ if (k === 'name') reads++; return o[k]; } }));
    return { list, reads: () => reads };
  };
  const per = n => {
    const b = build(n);
    for (const q of ['я', 'яй', 'яйц', 'яйце', 'яйце с0', 'позиція 7 соус']) R.resolve(q, b.list);
    R.suggest('соус', b.list);
    return b.reads() / n;
  };
  const a = per(500), b = per(1000);
  assert.ok(a <= 40 && b <= 40, `reads per supply: ${a} at 500, ${b} at 1000`);
  assert.ok(b <= a * 1.2, `it does not grow with the list: ${a} -> ${b}`);
});
