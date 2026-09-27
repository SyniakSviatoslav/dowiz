// apple-words.js: every language has every key, every lookup lands on a real
// word, the merge fills a new language from English, and the quotes are ASCII.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { WORDS, EXAMPLES, FOOTERS, merge, lookup } from './apple-words.js';
import { LANGS as ALL } from '../lib/langs.js';

const LANGS = Object.keys(WORDS);

test('every language of lib/langs.js, and no language is missing a key another has', () => {
  assert.deepEqual([...LANGS].sort(), [...ALL].sort());
  const all = new Set(LANGS.flatMap(l => Object.keys(WORDS[l])));
  for (const l of LANGS) {
    const missing = [...all].filter(k => !(k in WORDS[l]));
    assert.deepEqual(missing, [], `${l} lacks ${missing.join(', ')}`);
  }
});

test('every key wears the ap_ prefix, so nothing in the console dictionary is overwritten', () => {
  for (const l of LANGS) for (const k of Object.keys(WORDS[l])) assert.match(k, /^ap_(h|l|ex|f|why)_/, k);
});

test('every example and footer points at a word every language has', () => {
  for (const [field, key] of [...Object.entries(EXAMPLES), ...Object.entries(FOOTERS)]) {
    for (const l of LANGS) assert.ok(typeof WORDS[l][key] === 'string' && WORDS[l][key].length > 0, `${field} -> ${key} (${l})`);
  }
});

test('an example reads as an example, a footer as a sentence', () => {
  for (const l of LANGS) for (const [k, v] of Object.entries(WORDS[l])) {
    if (k.startsWith('ap_ex_') && k !== 'ap_ex_search') assert.match(v, /^(p\.sh\.|e\.g\.|напр\.) /, `${l}.${k}: ${v}`);
    if (k.startsWith('ap_f_') || k.startsWith('ap_h_') || k.startsWith('ap_why_')) assert.match(v, /[.]$/, `${l}.${k} ends with a full stop`);
  }
});

test('lookup: the id wins over the key, and nothing is invented', () => {
  assert.equal(lookup(EXAMPLES, 'i-name', 'name'), 'ap_ex_personName');
  assert.equal(lookup(EXAMPLES, 'nd-name', 'name'), 'ap_ex_dishName');
  assert.equal(lookup(EXAMPLES, null, 'phone'), 'ap_ex_phone');
  assert.equal(lookup(EXAMPLES, 'zz-none', 'zz-none'), null);
  assert.equal(lookup(FOOTERS, 'v-name', 'venueName'), null);
});

test('merge: a language with no words here gets English for every key, existing words are untouched', () => {
  const T = { sq: { save: 'Ruaj' }, en: { save: 'Save' }, ru: { save: 'Сохранить' }, xx: { save: 'X' } };
  merge(T);
  assert.equal(T.sq.save, 'Ruaj');
  assert.equal(T.sq.ap_h_venue, WORDS.sq.ap_h_venue);
  assert.equal(T.ru.ap_h_venue, WORDS.ru.ap_h_venue);
  assert.equal(T.xx.ap_h_venue, WORDS.en.ap_h_venue);
  assert.equal(T.ru.save, 'Сохранить');
});

test('the file holds only ASCII quotes (rule 11)', () => {
  const src = fs.readFileSync(new URL('./apple-words.js', import.meta.url), 'utf8');
  assert.ok(!/[‘’“”]/.test(src.replace(/«|»/g, '')), 'a typographic quote in apple-words.js');
});
