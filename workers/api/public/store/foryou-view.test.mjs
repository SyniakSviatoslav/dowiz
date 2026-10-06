// `node --test workers/api/public/store/foryou-view.test.mjs`
// W-TASTE2 row 2: the order page draws the hub's "For you" only when it was shown, only dishes this
// page can name, never a number, in four languages.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { forYouMarkup, STATES } from './foryou-view.js';

const names = { 'ebi': 'Ebi tempura', 'unagi': 'Unagi <b>', 'gone': null };
const nameOf = id => names[id] ?? null;

test('shown: the dishes the page can name, escaped, with the taste words and the hint', () => {
  const h = forYouMarkup({ contract: 'order.for-you.v1', state: 'shown', items: [{ id: 'ebi' }, { id: 'gone' }, { id: 'unagi' }], because: ['a:smoky', 'x:crispy'] }, nameOf);
  assert.match(h, /data-t="fy_title"/);
  assert.match(h, /data-fy-order="ebi"[^>]*><span>Ebi tempura<\/span>/);
  assert.match(h, /class="ui-chip fy"/, 'drawn with /lib/ui chip (ui-adoption)');
  assert.match(h, /Unagi &lt;b&gt;/, 'a name is escaped');
  assert.doesNotMatch(h, /gone/, 'a dish this page cannot name is left out');
  assert.match(h, /data-t="sx_a_smoky"/);
  assert.match(h, /data-t="fy_hint"/);
  assert.doesNotMatch(h, /\d{2,}/, 'no number drawn');
});

test('nothing is drawn for an objection, no profile, no taste, an empty or a broken answer', () => {
  for (const state of STATES.filter(s => s !== 'shown')) assert.equal(forYouMarkup({ state, items: [{ id: 'ebi' }] }, nameOf), '', state);
  assert.equal(forYouMarkup({ state: 'shown', items: [] }, nameOf), '');
  assert.equal(forYouMarkup({ state: 'shown', items: [{ id: 'gone' }] }, nameOf), '', 'no nameable dish, no box');
  assert.equal(forYouMarkup(null, nameOf), '');
  assert.equal(forYouMarkup({ state: 'shown' }, nameOf), '');
  // Positive twin of the last line.
  assert.notEqual(forYouMarkup({ state: 'shown', items: [{ id: 'ebi' }] }, nameOf), '');
});

test('the words exist in sq/en/uk/ru and the order page mounts it', () => {
  const words = readFileSync(new URL('./taste-words.js', import.meta.url), 'utf8');
  assert.equal((words.match(/fy_title:/g) || []).length, 4);
  assert.equal((words.match(/fy_hint:/g) || []).length, 4);
  const venue = readFileSync(new URL('./taste-venue.js', import.meta.url), 'utf8');
  assert.match(venue, /id="forYouOrder"/, 'the placeholder is on the order page');
  assert.match(venue, /\/taste\/for-you`/, 'it asks the hub');
  assert.match(venue, /mountForYou\(order, tok\)/, 'and is mounted with the order link');
});
