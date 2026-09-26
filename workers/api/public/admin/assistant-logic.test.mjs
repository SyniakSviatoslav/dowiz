// node --test workers/api/public/admin/assistant-logic.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as A from './assistant-logic.js';

const kitchen = { staff: true, caps: new Set(['advance', 'catalog', 'stock']) };
const waiter = { staff: true, caps: new Set(['take_orders', 'take_payment']) };
const owner = { staff: false, caps: new Set() };

test('a proposal is a write to confirm, carrying its token and whether it needs a reason', () => {
  const m = A.reading({ understood: true, needsConfirmation: true, verb: 'reject', readback: 'reject 4821', token: 'tok' });
  assert.deepEqual(m, { from: 'hub', kind: 'propose', text: 'reject 4821', token: 'tok', verb: 'reject', reason: true, state: 'open' });
  assert.equal(A.reading({ understood: true, needsConfirmation: true, verb: 'receive', readback: 'x', token: 't' }).reason, false);
  // No token, no proposal: nothing could be confirmed.
  assert.equal(A.reading({ understood: true, needsConfirmation: true, verb: 'ready' }).kind, 'say');
});

test('now-answers: a count, a screen, a question', () => {
  assert.deepEqual(A.reading({ understood: true, action: 'status', open: '3', waiting: 1 }), { from: 'hub', kind: 'status', open: 3, waiting: 1 });
  assert.deepEqual(A.reading({ understood: true, action: 'show', screen: 'stock' }), { from: 'hub', kind: 'show', screen: 'stock' });
  assert.equal(A.reading({ understood: true, action: 'show', screen: 'payouts' }).kind, 'say', 'a screen that is not a console tab is not navigated to');
  assert.deepEqual(A.reading({ understood: true, action: 'ask', question: 'which dishes use salmon' }), { from: 'hub', kind: 'ask', question: 'which dishes use salmon' });
  assert.equal(A.reading({ understood: true, action: 'ask' }).kind, 'say');
});

test('a refusal says why and what was heard; nothing at all is a plain failure', () => {
  assert.deepEqual(A.reading({ understood: false, say: 'Which ingredient?', heard: 'received 5 kg' }), { from: 'hub', kind: 'say', text: 'Which ingredient?', heard: 'received 5 kg' });
  assert.deepEqual(A.reading(null), { from: 'hub', kind: 'say', text: null, heard: null });
  assert.deepEqual(A.reading({ understood: true, action: 'mystery' }), { from: 'hub', kind: 'say', text: null, heard: null });
});

test('the log keeps the last lines, and settling touches only the proposal named', () => {
  let log = [];
  for (let i = 0; i < A.MAX_KEPT + 5; i++) log = A.push(log, { i });
  assert.equal(log.length, A.MAX_KEPT);
  assert.equal(log[0].i, 5);
  assert.deepEqual(A.push(null, { a: 1 }), [{ a: 1 }]);
  const two = [{ kind: 'propose', state: 'open' }, { kind: 'say' }];
  assert.deepEqual(A.settle(two, 0, 'done'), [{ kind: 'propose', state: 'done' }, { kind: 'say' }]);
  assert.deepEqual(A.settle(two, 1, 'done'), two, 'a line that is not a proposal is not settled');
  assert.deepEqual(A.settle(null, 0, 'done'), []);
});

test('starters are the lines this person may act on', () => {
  assert.deepEqual(A.starters(kitchen), ['asStatus', 'asShowKitchen', 'asShowStock', 'asLow']);
  assert.deepEqual(A.starters(waiter), ['asStatus']);
  assert.deepEqual(A.starters(owner), ['asStatus', 'asShowKitchen', 'asShowStock', 'asLow']);
  assert.deepEqual(A.starters({ staff: true, caps: new Set(['stock']) }), ['asShowStock', 'asLow']);
});

test('a line is trimmed, and an empty or endless one is not sent', () => {
  assert.equal(A.clean('  received   4 kg\nsalmon '), 'received 4 kg salmon');
  assert.equal(A.clean('   '), null);
  assert.equal(A.clean(null), null);
  assert.equal(A.clean('x'.repeat(A.MAX_LINE + 1)), null);
  assert.deepEqual(A.lineBody('status', 'sq'), { transcript: 'status', confidence: 1, is_final: true, lang: 'sq' });
});
