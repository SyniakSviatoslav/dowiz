// ux-words.js: every language carries every key, ASCII delimiters only, and
// the merge never lets a key fall through t() as itself.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { WORDS, merge } from './ux-words.js';

test('every language has the same keys as English, and none is empty', () => {
  const keys = Object.keys(WORDS.en).sort();
  assert.ok(keys.length > 5);
  for (const [lang, table] of Object.entries(WORDS)) {
    assert.deepEqual(Object.keys(table).sort(), keys, `${lang} keys`);
    for (const [k, v] of Object.entries(table)) assert.ok(typeof v === 'string' && v.trim(), `${lang}.${k}`);
  }
});

test('the file uses ASCII quotes as delimiters (rule 11)', () => {
  const src = readFileSync(new URL('./ux-words.js', import.meta.url), 'utf8');
  // A typographic quote may appear INSIDE a word (it is text), never as the
  // character that opens or closes one: every value opens with an ASCII quote.
  for (const line of src.split('\n')) {
    if (!/^\s+[a-zA-Z_]+: /.test(line) || /^\s+[a-z]+: \{/.test(line)) continue;
    for (const m of line.matchAll(/[a-zA-Z_]+: (.)/g)) assert.equal(m[1], "'", `delimiter in: ${line.trim().slice(0, 60)}`);
  }
});

test('merge: adds the words to each language and fills a missing language from English', () => {
  const T = { sq: { save: 'Ruaj' }, en: { save: 'Save' }, uk: { save: 'Зберегти' }, ru: { save: 'Сохранить' } };
  const out = merge(T);
  assert.equal(out, T);
  assert.equal(T.sq.save, 'Ruaj');
  assert.equal(T.sq.appearance, 'Pamja');
  assert.equal(T.uk.theme_dark, 'Темний');
  // ru has no words yet: it reads English, never the key
  assert.equal(T.ru.appearance, 'Appearance');
  assert.equal(T.ru.save, 'Сохранить');
});

test('merge: a language only the words have is created; own words win over English (positive twin)', () => {
  const T = { en: {} };
  merge(T, { en: { a: 'A', b: 'B' }, sq: { a: 'Aa' } });
  assert.deepEqual(T.sq, { a: 'Aa', b: 'B' });
  assert.deepEqual(T.en, { a: 'A', b: 'B' });
});
