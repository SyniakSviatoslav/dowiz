// `node --test workers/api/public/store/avoid.test.mjs`
// The guest's allergen filter (W-MR0 row MR0 iii): only the guest's explicit choice hides,
// an undeclared dish is hidden by that choice, and an inferred allergy never hides.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { declaration, hiddenBecause, warnBecause, hiddenCounts } from './avoid.js';

const undeclared = { id: 'u' };                       // the hub's "nobody has said"
const none = { id: 'n', allergens: [] };              // "none of the fourteen"
const shrimp = { id: 's', allergens: ['crustaceans', 'soy'] };

test('three states, as the hub stores them', () => {
  assert.equal(declaration(undeclared).kind, 'undeclared');
  assert.equal(declaration({ allergens: null }).kind, 'undeclared', 'null is not a declaration');
  assert.equal(declaration(none).kind, 'none');
  assert.deepEqual(declaration(shrimp), { kind: 'contains', codes: ['crustaceans', 'soy'] });
});

test('no choice made, nothing hidden: not even the undeclared', () => {
  for (const p of [undeclared, none, shrimp]) {
    assert.equal(hiddenBecause(p, []), null);
    assert.equal(hiddenBecause(p, undefined), null);
  }
});

test("the guest's explicit choice hides a dish that contains it AND every undeclared dish", () => {
  assert.equal(hiddenBecause(shrimp, ['crustaceans']), 'contains');
  assert.equal(hiddenBecause(undeclared, ['crustaceans']), 'undeclared', 'nobody said is not safe');
  assert.equal(hiddenBecause(none, ['crustaceans']), null, '"none of the 14" stays');
  assert.equal(hiddenBecause(shrimp, ['milk']), null);
  assert.deepEqual(hiddenCounts([undeclared, none, shrimp], ['soy']), { contains: 1, undeclared: 1 });
});

test('an INFERRED allergy never hides: it can only warn (EU 1169/2011)', () => {
  const inferred = ['crustaceans'];
  assert.equal(hiddenBecause(shrimp, []), null, 'with no explicit choice the dish stays on screen');
  assert.equal(warnBecause(shrimp, inferred), 'contains');
  assert.equal(warnBecause(undeclared, inferred), 'undeclared');
  assert.equal(warnBecause(none, inferred), null);
  assert.deepEqual(hiddenCounts([undeclared, none, shrimp], []), { contains: 0, undeclared: 0 });
});
