// node --test workers/api/public/admin/prep-list-view.test.mjs
// The prep list as drawn, from an answer shaped exactly like the hub's
// (`services/analytics/forecast/tests.rs`, kitchen.prep_forecast.v1): what to
// make, the error, "learning" as a word and never a zero, no markup injected.
import test from 'node:test';
import assert from 'node:assert/strict';
import { draw, card, num, offBy, nameOf } from './prep-list-view.js';
import { WORDS, merge } from './prep-list-words.js';
import { LANGS } from '../lib/langs.js';
import { prepCard } from './kitchen-view.js';

const est = (value, weeks = 4) => ({ value, learning: false, weeks, method: 'median' });
const answer = {
  contract: 'kitchen.prep_forecast.v1', day: '2026-09-22', today: '2026-09-22', weekday: 1,
  hours: { known: true, open: true },
  orders: est(10), portions: { ...est(20), offBy: 3, naiveOffBy: 5, masePm: 600, checkedDays: 28 },
  bands: [{ ...est(0), band: 0, from: '11:00', to: '14:00' }, { ...est(10), band: 1, from: '14:00', to: '18:00' }],
  dishes: [{ ...est(10), id: 'philadelphia', name: 'Phila <b>delphia</b>', offBy: 1, fromBookings: 4 }, { value: null, learning: true, weeks: 2, method: null, id: 'new', name: { en: 'New roll', sq: 'Rol i ri' }, offBy: null, fromBookings: 0 }],
  bookings: { covers: 4, portions: 8, error: null },
  preps: [{ id: 'rice-seasoned', name: 'Rice seasoned', unit: 'g', need: 1300, onHand: 500, make: 800, from: [{ id: 'rice-dry', name: 'rice-dry', unit: 'g', qty: 381 }] }],
  raw: [{ id: 'rice-dry', name: 'rice-dry', unit: 'g', qty: 1519 }],
  unmodelled: ['cola'], refused: [], samples: [], history: { archivedDays: 0, error: null },
};

test('the list draws what to make, from what, and the measured error', () => {
  const html = draw(answer, { lang: 'en' });
  for (const k of ['pl_preps', 'pl_bands', 'pl_dishes', 'pl_raw', 'pl_offBy', 'pl_unmodelled', 'pl_limits']) assert.ok(html.includes(`data-t="${k}"`), k);
  for (const v of ['1300 g', '>500<', '<b>800</b>', 'rice-dry 381 g', '&plusmn;3', '&plusmn;5', '14:00-18:00', '1519 g', 'cola']) assert.ok(html.includes(v), v);
  assert.ok(!html.includes('<b>delphia</b>') && html.includes('&lt;b&gt;delphia'), 'a dish name is text, not markup');
  assert.ok(html.includes('New roll'), 'a name in languages reads the viewer\'s');
  for (const a of ['prepList.summary', 'prepList.preps', 'prepList.error']) assert.ok(html.includes(`data-tour="${a}"`), a);
});

test('learning is a word, never a zero, and nothing is listed to make', () => {
  const young = { ...answer, portions: { value: null, learning: true, weeks: 2 }, orders: { value: null, learning: true, weeks: 2 }, preps: [], raw: [] };
  const html = draw(young, { lang: 'en' });
  assert.ok(html.includes('pl_learningHint'));
  assert.ok(!html.includes('data-t="pl_preps"'), 'no prep list while learning');
  assert.equal(num({ value: null, learning: true }), '<span data-t="pl_learning"></span>');
  assert.equal(num(est(0)), '0', 'the twin: a forecast of zero is a zero');
});

test('a shelf that covers the forecast says so; a closed day says so', () => {
  const full = { ...answer, preps: [{ ...answer.preps[0], onHand: 2000, make: 0, from: [] }] };
  assert.ok(draw(full).includes('pl_nothingToMake'));
  assert.ok(draw({ ...answer, hours: { known: true, open: false }, bands: [] }).includes('pl_closed'));
  assert.ok(draw({ ...answer, hours: { known: false, open: true } }).includes('pl_hoursUnknown'));
  assert.ok(draw({ ...answer, history: { error: 'cube <x>' } }).includes('cube &lt;x&gt;'));
});

test('the error line appears only when the hub measured one', () => {
  assert.equal(offBy({ offBy: null }), '');
  assert.equal(offBy(undefined), '');
  assert.ok(offBy({ offBy: 2, naiveOffBy: null }).includes('&plusmn;2') && !offBy({ offBy: 2, naiveOffBy: null }).includes('pl_naive'));
});

test('the card opens the list and the kitchen numbers re-export it', () => {
  assert.ok(card(answer).includes('data-openprep') && card(answer).includes('pl_card'));
  assert.equal(card(null), '');
  assert.equal(prepCard, card);
  assert.equal(nameOf(null), '');
  assert.equal(nameOf({ sq: 'Rol' }, 'uk'), 'Rol');
});

test('every language has every word of the prep list', () => {
  const keys = new Set(Object.values(WORDS).flatMap(Object.keys));
  for (const l of LANGS) for (const k of keys) assert.ok(WORDS[l] && WORDS[l][k], `${l}.${k}`);
  const T = { en: { pl_title: 'kept' }, de: {} };
  merge(T);
  assert.equal(T.en.pl_title, 'kept', 'a word the console has stays');
  assert.equal(T.de.pl_tile, WORDS.en.pl_tile, 'a language without words reads English');
});
