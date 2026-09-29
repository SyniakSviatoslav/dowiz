// W-CRUD: the dish sheet's translation boxes and the base-language save body.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { translationBoxes, baseEdits } from './menu-edit.js';

const LANGS = ['sq', 'en', 'uk', 'ru'];

test('a language that answered the base name has an EMPTY box, a real translation shows', () => {
  const tr = {
    en: { name: 'Salmon roll', description: 'eight pieces' },
    sq: { name: 'Salmon roll', description: 'eight pieces' },   // no sq words: the menu fell back
    uk: { name: 'Рол з лососем', description: 'eight pieces' }, // a name, no description
    ru: { name: 'Salmon roll', description: 'восемь штук' },
  };
  const boxes = translationBoxes(tr, 'en', LANGS);
  assert.deepEqual(Object.keys(boxes), ['sq', 'uk', 'ru'], 'the base language has no box');
  assert.deepEqual(boxes.sq, { name: '', description: '' }, 'RED 2026-09-29: the base name was prefilled and saved back as Albanian');
  assert.deepEqual(boxes.uk, { name: 'Рол з лососем', description: '' });
  assert.deepEqual(boxes.ru, { name: '', description: 'восемь штук' });
  assert.deepEqual(translationBoxes({}, 'sq', LANGS).en, { name: '', description: '' }, 'a dish the other menus did not answer');
});

test('a save sends only the base words and the category that changed', () => {
  const before = { name: 'Sake', description: '', categoryId: 'rolls' };
  assert.deepEqual(baseEdits(before, { name: ' Sake ', description: '', categoryId: 'rolls' }), {}, 'nothing typed, nothing sent');
  assert.deepEqual(baseEdits(before, { name: 'Sake Nigiri', description: 'two pieces', categoryId: 'nigiri' }),
    { name: 'Sake Nigiri', description: 'two pieces', category_id: 'nigiri' });
  assert.deepEqual(baseEdits(before, { name: '   ', description: '', categoryId: 'rolls' }), {}, 'a blanked name is kept, not sent');
  assert.deepEqual(baseEdits({ ...before, description: 'old' }, { name: 'Sake', description: '', categoryId: 'rolls' }), { description: '' }, 'a blanked description is "no description"');
  assert.deepEqual(baseEdits(before, { name: 'Sake', description: '', categoryId: '' }), {}, 'no category chosen sends no move');
});
