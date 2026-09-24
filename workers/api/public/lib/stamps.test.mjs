// The stamp card's browser rules, called for real. `node --test workers/api/public/lib/stamps.test.mjs`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { stampView, stampValues } from './stamps.js';

test('no card, nothing drawn; a card is drawn as the hub counted it', () => {
  assert.equal(stampView(null), null);
  assert.equal(stampView({ on: false }), null);
  assert.equal(stampView({ on: true, n: 0 }), null);
  assert.deepEqual(stampView({ on: true, have: 3, n: 10, reward: 500, used: 0 }), { have: 3, n: 10, reward: 500, used: 0 });
});

test('the count is clamped to the card, never invented', () => {
  assert.equal(stampView({ on: true, have: 14, n: 10 }).have, 10);
  assert.equal(stampView({ on: true, have: -2, n: 10 }).have, 0);
  assert.equal(stampView({ on: true, have: 'x', n: 10 }).have, 0);
});

test('the switch is saved last, after the count and the reward', () => {
  assert.deepEqual(stampValues(true, ' 8 ', '500'), [
    ['loyalty.stamps.n', '8'], ['loyalty.stamps.reward_minor', '500'], ['loyalty.stamps.enabled', '1']]);
  assert.equal(stampValues(false, 10, '')[2][1], '0');
});
