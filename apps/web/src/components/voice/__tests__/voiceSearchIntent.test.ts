import { test } from 'node:test';
import assert from 'node:assert/strict';

// The unit under test wires the BUILT matcher (@deliveryos/voice) to the storefront search box.
// We intentionally exercise the real matchIntent (no reimplementation) so a regression in the
// sq/en/uk grammar OR in the SET_SEARCH → query mapping fails here.
import { matchIntent } from '@deliveryos/voice';
import type { MenuContext } from '@deliveryos/voice';
import { intentToSearchQuery, voiceSearchAvailability } from '../voiceSearchIntent';

const MENU: MenuContext = {
  products: [
    { id: 'p1', name: 'Pizza Margherita' },
    { id: 'p2', name: 'Sufllaqe' },
  ],
  categories: [{ id: 'c1', name: 'Pizza' }],
};

// --- intent → SET_SEARCH mapping across the three locales -----------------------------------------
// Fixture per the task: "pizza" / "pica" / "піца" utterances resolve to a SET_SEARCH intent whose
// query the storefront search box consumes. Each transcript carries the locale's search trigger.

test('en: "search pizza" → SET_SEARCH(query="pizza")', () => {
  const proposal = matchIntent('search pizza', 'en', MENU);
  assert.ok(proposal, 'matcher produced a proposal');
  assert.equal(proposal!.kind, 'SET_SEARCH');
  assert.equal(intentToSearchQuery(proposal), 'pizza');
});

test('sq: "kerko pica" → SET_SEARCH(query="pica")', () => {
  const proposal = matchIntent('kerko pica', 'sq', MENU);
  assert.ok(proposal, 'matcher produced a proposal');
  assert.equal(proposal!.kind, 'SET_SEARCH');
  assert.equal(intentToSearchQuery(proposal), 'pica');
});

test('uk: "знайти піца" → SET_SEARCH(query="піца")', () => {
  const proposal = matchIntent('знайти піца', 'uk', MENU);
  assert.ok(proposal, 'matcher produced a proposal');
  assert.equal(proposal!.kind, 'SET_SEARCH');
  assert.equal(intentToSearchQuery(proposal), 'піца');
});

// --- search-only: no other intent leaks through as a search --------------------------------------

test('ADD_TO_CART proposal maps to null (search-only, never acts by voice)', () => {
  const proposal = matchIntent('add two sufllaqe', 'en', MENU);
  assert.ok(proposal);
  assert.equal(proposal!.kind, 'ADD_TO_CART');
  assert.equal(intentToSearchQuery(proposal), null);
});

test('NAVIGATE_CHECKOUT proposal maps to null', () => {
  const proposal = matchIntent('go to checkout', 'en', MENU);
  assert.ok(proposal);
  assert.equal(proposal!.kind, 'NAVIGATE_CHECKOUT');
  assert.equal(intentToSearchQuery(proposal), null);
});

test('intentToSearchQuery: null/empty/non-string query → null', () => {
  assert.equal(intentToSearchQuery(null), null);
  assert.equal(intentToSearchQuery({ kind: 'SET_SEARCH', args: {}, transcript: '', confidence: 1 }), null);
  assert.equal(
    intentToSearchQuery({ kind: 'SET_SEARCH', args: { query: '   ' }, transcript: '', confidence: 1 }),
    null,
  );
  assert.equal(
    intentToSearchQuery({ kind: 'SET_SEARCH', args: { query: 42 }, transcript: '', confidence: 1 }),
    null,
  );
});

// --- render/enable predicate (true-dark gating) ---------------------------------------------------

test('voiceSearchAvailability: flag off → not rendered (true-dark)', () => {
  const a = voiceSearchAvailability({ flagEnabled: false, secureContext: true, hasMediaDevices: true });
  assert.deepEqual(a, { render: false, reason: 'flag-off' });
});

test('voiceSearchAvailability: insecure context → not rendered', () => {
  const a = voiceSearchAvailability({ flagEnabled: true, secureContext: false, hasMediaDevices: true });
  assert.deepEqual(a, { render: false, reason: 'insecure-context' });
});

test('voiceSearchAvailability: no mic → not rendered', () => {
  const a = voiceSearchAvailability({ flagEnabled: true, secureContext: true, hasMediaDevices: false });
  assert.deepEqual(a, { render: false, reason: 'no-media-devices' });
});

test('voiceSearchAvailability: all present → rendered', () => {
  const a = voiceSearchAvailability({ flagEnabled: true, secureContext: true, hasMediaDevices: true });
  assert.deepEqual(a, { render: true });
});
