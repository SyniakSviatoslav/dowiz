// node --test workers/api/public/admin/start-stock-logic.test.mjs
// The starter pack and the recipe skeletons: deterministic matching in four
// languages, grams left EMPTY, and the two CSV files the importers read.
import test from 'node:test';
import assert from 'node:assert/strict';
import * as L from './start-stock-logic.js';
import { PACK, CATS, RULES } from './start-stock-pack.js';
import { LANGS } from '../lib/langs.js';

test('the pack: about forty supplies, unique ids, every name and group in every language', () => {
  assert.ok(PACK.length >= 38 && PACK.length <= 45, `${PACK.length}`);
  assert.equal(new Set(PACK.map(p => p.id)).size, PACK.length);
  for (const p of PACK) {
    assert.ok(['g', 'ml', 'unit'].includes(p.unit), p.id);
    assert.ok(CATS[p.cat], `${p.id}: ${p.cat}`);
    for (const l of LANGS) assert.ok(p.n[l]?.trim(), `${p.id} has no ${l} name`);
    if (p.clean != null) assert.ok(p.clean >= 1 && p.clean <= 1000, `${p.id} clean`);
    if (p.cook != null) assert.ok(p.cook >= 1 && p.cook <= 5000, `${p.id} cook`);
    if (p.kind === 'food_ingredient' || p.kind === 'condiment') assert.equal(p.nut?.length, 4, `${p.id}: a food has its nutrition, or its dishes read incomplete`);
  }
  for (const r of RULES) for (const id of r.add) assert.ok(PACK.some(p => p.id === id), `rule names ${id}`);
});

test('a dish name proposes lines in any of the four languages, the same every time', () => {
  const salmonRoll = ['pack-salmon', 'pack-rice', 'pack-nori', 'pack-box'];
  assert.deepEqual(L.match('QA Salmon roll'), salmonRoll);
  assert.deepEqual(L.match('Roll me salmon'), salmonRoll, 'Albanian word order');
  assert.deepEqual(L.match('Рол з лососем'), salmonRoll, 'Ukrainian, inflected');
  assert.deepEqual(L.match('Ролл с лососем'), salmonRoll, 'Russian');
  assert.deepEqual(L.match('Philadelphia'), ['pack-salmon', 'pack-cream-cheese', 'pack-box']);
  assert.deepEqual(L.match('California roll'), ['pack-surimi', 'pack-tobiko', 'pack-rice', 'pack-nori', 'pack-avocado', 'pack-cucumber', 'pack-mayo', 'pack-box']);
  assert.deepEqual(L.match('QA Water'), ['pack-water'], 'a drink: no box');
  assert.deepEqual(L.match('Çaj jeshil'), ['pack-tea'], 'accents off: caj');
  assert.deepEqual(L.match('Tiramisu'), [], 'unmatched is unmatched, never a guess');
  assert.deepEqual(L.match('QA Salmon roll'), L.match('QA Salmon roll'));
});

test('skeletons: grams EMPTY, a dish with a recipe untouched, the rest unmatched', () => {
  const s = L.skeletons([
    { id: 'd1', name: 'QA Salmon roll', bom: [] },
    { id: 'd2', name: 'QA Tuna roll', bom: [{ supply: 'x', qty: 1 }] },
    { id: 'd3', name: 'Panna cotta' },
  ]);
  assert.deepEqual(s.drafts.map(d => d.id), ['d1']);
  assert.ok(s.drafts[0].lines.every(l => l.qty === null), 'nothing is guessed');
  assert.deepEqual(s.unmatched, [{ id: 'd3', name: 'Panna cotta' }]);
  assert.equal(L.ready(s.drafts[0]), false);
  s.drafts[0].lines.forEach((l, i) => { l.qty = 10 + i; });
  assert.equal(L.ready(s.drafts[0]), true);
  assert.equal(L.ready({ lines: [] }), false);
});

test('amounts: whole base units, the unit checked', () => {
  assert.equal(L.amount('40', 'g'), 40);
  assert.equal(L.amount('1,5 kg', 'g'), 1500);
  assert.equal(L.amount('0,5 l', 'ml'), 500);
  assert.equal(L.amount('2', 'unit'), 2);
  for (const bad of ['', '0', '-3', '2.5', 'abc', '1 kg']) assert.equal(L.amount(bad, bad === '1 kg' ? 'ml' : 'g'), null, bad);
});

test('the supplies file: stable ids, names in the language, defaults in the extra columns', () => {
  const csv = L.suppliesCsv(['pack-rice', 'pack-water', 'pack-nori'], 'sq');
  const lines = csv.trim().split('\n');
  assert.equal(lines[0], 'id,name,unit,kind,category,kcal,protein,fat,carbs,weight_per_unit,clean_pm,cook_pm,pack,pack_qty');
  assert.equal(lines.length, 4, 'pack order, only the chosen');
  assert.equal(lines[1], 'pack-rice,Oriz për sushi,g,food_ingredient,Oriz dhe bazë,360,6.6,0.6,79,,,2200,10 kg,10000');
  assert.equal(lines[2], 'pack-nori,Nori (fletë),unit,food_ingredient,Oriz dhe bazë,6,1.2,0.1,0.7,3,,,x50,50');
  assert.equal(lines[3], 'pack-water,"Ujë 0,5 l",unit,resale,Pije,,,,,,,,x24,24', 'a comma inside a name is quoted');
  assert.equal(L.suppliesCsv(['pack-rice'], 'en'), L.suppliesCsv(['pack-rice'], 'en'), 'the same file every time');
});

test('the recipes file: ready dishes only, the quantity with its unit', () => {
  const drafts = [
    { id: 'd1', lines: [{ supply: 'pack-salmon', qty: 40 }, { supply: 'pack-nori', qty: 1 }] },
    { id: 'd2', lines: [{ supply: 'pack-salmon', qty: null }] },
  ];
  assert.equal(L.recipesCsv(drafts), 'dish,ingredient,qty\nd1,pack-salmon,40 g\nd1,pack-nori,1 unit\n');
});

test('what the venue has already is not offered again', () => {
  const s = L.split([{ id: 'pack-rice' }, { id: 'own-thing' }]);
  assert.ok(s.had.includes('pack-rice') && !s.fresh.includes('pack-rice'));
  assert.equal(s.fresh.length, PACK.length - 1);
});
