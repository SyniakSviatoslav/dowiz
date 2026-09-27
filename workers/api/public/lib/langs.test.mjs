// The one language set, in node: `node --test workers/api/public/lib/langs.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as L from './langs.js';

test('LANGS: four languages, Albanian first, frozen, ru included', () => {
  assert.equal(L.LANGS[0], 'sq');
  assert.ok(L.LANGS.includes('ru'));
  assert.equal(new Set(L.LANGS).size, L.LANGS.length);
  assert.ok(Object.isFrozen(L.LANGS));
});

test('every language has a name and an Intl tag that parses', () => {
  for (const l of L.LANGS) {
    assert.ok(L.NAMES[l], l);
    assert.equal(L.INTL[l].slice(0, 2), l);
    assert.doesNotThrow(() => new Intl.NumberFormat(L.INTL[l]));
  }
  assert.deepEqual(Object.keys(L.NAMES).sort(), [...L.LANGS].sort());
  assert.deepEqual(Object.keys(L.INTL).sort(), [...L.LANGS].sort());
});

test('MEDIA_LANGS: every UI language but ru (videos stay out of Russian)', () => {
  assert.ok(!L.MEDIA_LANGS.includes('ru'));
  assert.deepEqual(L.MEDIA_LANGS, L.LANGS.filter(l => l !== 'ru'));
});

test('isLang / norm: codes and BCP-47 tags of ours; anything else refused', () => {
  assert.ok(L.isLang('ru'));
  assert.ok(!L.isLang('de'));
  assert.equal(L.norm('ru-RU'), 'ru');
  assert.equal(L.norm('UK'), 'uk');
  assert.equal(L.norm('de-DE'), '');
  assert.equal(L.norm(null), '');
});

test('tagFor: the tag, else the fallback language, else English', () => {
  assert.equal(L.tagFor('ru'), 'ru-RU');
  assert.equal(L.tagFor('xx', 'uk'), 'uk-UA');
  assert.equal(L.tagFor('xx'), 'en-GB');
  assert.equal(L.tagFor('xx', 'yy'), 'en-GB');
});

test('pickLang: a stored choice wins; then the browser list in order; then the default', () => {
  assert.equal(L.pickLang(['uk'], ['ru-RU']), 'uk');
  assert.equal(L.pickLang([null, 'zz'], ['de-DE', 'ru-RU', 'en-US']), 'ru');
  assert.equal(L.pickLang([], ['de-DE']), 'sq');
  assert.equal(L.pickLang([], ['de-DE'], 'en'), 'en');
  assert.equal(L.pickLang(), 'sq');
});

test('pluralIndex: East Slavic three forms for uk and ru, two for the rest', () => {
  for (const l of ['uk', 'ru']) {
    assert.deepEqual([1, 2, 4, 5, 11, 12, 14, 21, 22, 25, 111].map(n => L.pluralIndex(l, n)), [0, 1, 1, 2, 2, 2, 2, 0, 1, 2, 2]);
  }
  assert.deepEqual([1, 2, 5].map(n => L.pluralIndex('en', n)), [0, 2, 2]);
  assert.deepEqual([1, 2].map(n => L.pluralIndex('sq', n, 2)), [0, 1]);
  assert.deepEqual([1, 3].map(n => L.pluralIndex('ru', n, 2)), [0, 1]);
});

test('scriptLang: Cyrillic is not one language -- Ukrainian letters, Russian letters, else the UI', () => {
  assert.equal(L.scriptLang('Дуже смачно, дякуємо'), 'uk');
  assert.equal(L.scriptLang('Очень вкусно, спасибо, всё отлично'), 'ru');
  assert.equal(L.scriptLang('Вкусно', 'uk'), 'uk');
  assert.equal(L.scriptLang('Вкусно', 'ru'), 'ru');
  assert.equal(L.scriptLang('Вкусно', 'en'), 'ru');
  assert.equal(L.scriptLang('Shumë e mirë'), 'sq');
  assert.equal(L.scriptLang('ushqim dhe shërbim'), 'sq');
  assert.equal(L.scriptLang('Very good'), 'en');
  assert.equal(L.scriptLang(null), 'en');
});

test('pick: the language, then English, then the venue default, then anything; empty maps give ""', () => {
  assert.equal(L.pick({ ru: 'Суп', en: 'Soup', sq: 'Supë' }, 'ru'), 'Суп');
  assert.equal(L.pick({ en: 'Soup', sq: 'Supë' }, 'ru'), 'Soup');
  assert.equal(L.pick({ sq: 'Supë', uk: 'Суп' }, 'ru'), 'Supë');
  assert.equal(L.pick({ uk: 'Суп', ru: '' }, 'ru', 'sq'), 'Суп');
  assert.equal(L.pick({ ru: '' }, 'ru'), '');
  assert.equal(L.pick(null, 'ru'), '');
});
