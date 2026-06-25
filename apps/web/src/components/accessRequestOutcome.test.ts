import test from 'node:test';
import assert from 'node:assert/strict';
import { decideOutcome } from './accessRequestOutcome.js';

// R3-3a proof: the success state can NEVER be reached on a no-consent send, even when the
// server replies 200 (the server stays uniform-200 for anti-enumeration).
test('decideOutcome: consent-in-body + 2xx → success', () => {
  assert.deepEqual(decideOutcome(true, { ok: true, status: 200 }), { state: 'success' });
});

test('decideOutcome: NO consent in sent body + 200 → error (never false success)', () => {
  const out = decideOutcome(false, { ok: true, status: 200 });
  assert.equal(out.state, 'error');
  assert.notEqual(out.state, 'success');
});

test('decideOutcome: 429 → rate-limit error copy', () => {
  assert.deepEqual(decideOutcome(true, { ok: false, status: 429 }), { state: 'error', err: 'rate' });
});

test('decideOutcome: 5xx → generic error', () => {
  assert.deepEqual(decideOutcome(true, { ok: false, status: 503 }), { state: 'error', err: 'generic' });
});
