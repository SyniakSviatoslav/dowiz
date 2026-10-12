// node --test workers/api/public/admin/fiscal-tip.test.mjs
// The tip's till item picker (D34, W-TIPS2): the options, a lost choice kept,
// and the words in every console language.
import test from 'node:test';
import assert from 'node:assert/strict';
import { WORDS, merge, tipOptions } from './fiscal-tip.js';
import { LANGS } from '../lib/langs.js';

const items = [{ code: '16', name: 'Cola' }, { code: '99', name: 'Tip' }];

test('none first, then every till item as code - name', () => {
  assert.deepEqual(tipOptions(items, ''), [{ value: '', key: 'fx_noTip' }, { value: '16', label: '16 - Cola' }, { value: '99', label: '99 - Tip' }]);
  assert.deepEqual(tipOptions(undefined, ''), [{ value: '', key: 'fx_noTip' }], 'a hub with no items still offers none');
});

test('a chosen item the till no longer lists is kept; a listed one is not doubled', () => {
  const lost = tipOptions(items, '77');
  assert.deepEqual(lost[lost.length - 1], { value: '77', label: '77' });
  assert.equal(tipOptions(items, '99').filter(o => o.value === '99').length, 1);
});

test('every language has every word, and the hint names VAT 0', () => {
  const keys = Object.keys(WORDS.en);
  for (const l of LANGS) assert.deepEqual(Object.keys(WORDS[l] || {}).sort(), [...keys].sort(), l);
  for (const l of LANGS) assert.match(WORDS[l].fx_tipHint, /0 %/, l);
});

test('merge adds the words and never overwrites the console', () => {
  const T = { en: { fx_tipItem: 'kept' }, sq: {} };
  merge(T);
  assert.equal(T.en.fx_tipItem, 'kept');
  assert.equal(T.sq.fx_noTip, WORDS.sq.fx_noTip);
  assert.equal(T.uk.fx_tipHint, WORDS.uk.fx_tipHint, 'a language the console lacks is added');
});
