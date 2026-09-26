// more-find.js: finding a destination by its word, in any language, accents ignored.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { norm, matches, filter, count } from './more-find.js';

const WORDS = { hours: 'Orari', hoursSub: 'orari javor', venue: 'Lokali', venueSub: 'emri, telefoni, adresa', promos: 'Kodet e zbritjes', promosSub: 'kode zbritjeje', learn: 'Mësimet' };
const t = k => WORDS[k] ?? k;
const GROUPS = [['settings', [['venue', 'home'], ['hours', 'clock']]], ['marketing', [['promos', 'ticket']]], ['learnGroup', [['learn', 'player-play']]]];

test('norm: lower-case and accent-free', () => {
  assert.equal(norm('Mësimet'), 'mesimet');
  assert.equal(norm('  Orari '), '  orari ');
  assert.equal(norm(null), '');
});

test('matches: by title, by subtitle, by every term, accents ignored', () => {
  assert.ok(matches('hours', 'orar', t));
  assert.ok(matches('hours', 'javor', t));
  assert.ok(matches('venue', 'telefoni adresa', t));
  assert.ok(matches('learn', 'mesim', t));
  assert.ok(matches('venue', 'venue', t), 'the key itself finds the row in any language');
  assert.ok(matches('hours', '', t), 'an empty query matches everything');
});

test('matches: a term that is nowhere refuses (negative twin), and a missing Sub key is not a match', () => {
  assert.equal(matches('hours', 'zbritje', t), false);
  assert.equal(matches('venue', 'telefoni pizza', t), false);
  // learn has no learnSub: the literal key "learnSub" must not be searchable
  assert.equal(matches('learn', 'learnsub', t), false);
});

test('filter: keeps only matching rows, drops empty groups, keeps order', () => {
  assert.deepEqual(filter(GROUPS, 'orar', t), [['settings', [['hours', 'clock']]]]);
  assert.deepEqual(filter(GROUPS, 'zbritje', t), [['marketing', [['promos', 'ticket']]]]);
  assert.deepEqual(filter(GROUPS, 'nothing-here', t), []);
});

test('filter: an empty or blank query returns the same groups object (positive twin)', () => {
  assert.equal(filter(GROUPS, '', t), GROUPS);
  assert.equal(filter(GROUPS, '   ', t), GROUPS);
  assert.equal(count(GROUPS), 4);
  assert.equal(count(filter(GROUPS, 'x', t)), 0);
});
