// node --test workers/api/public/admin/outcome-logic.test.mjs
// Every write a person makes ends in a notice: saved, or not saved and why.
import test from 'node:test';
import assert from 'node:assert/strict';
import * as O from './outcome-logic.js';

const t = k => `<${k}>`;

test('a write that landed says saved, in the success tone', () => {
  assert.deepEqual(O.notice({ ok: true }, t), { text: '<saved>', tone: 'success', icon: 'circle-check' });
});

test('a refused write says not saved, why by its status, and the hub\'s own words', () => {
  const cases = [[400, 'out_bad'], [422, 'out_bad'], [403, 'out_denied'], [404, 'out_missing'], [409, 'out_conflict'], [413, 'out_tooBig'], [429, 'out_busy'], [500, 'out_server'], [503, 'out_server']];
  for (const [status, why] of cases) {
    const n = O.notice({ ok: false, status, message: 'an ingredient needs a short id' }, t);
    assert.equal(n.text, `<out_notSaved>: <${why}> (an ingredient needs a short id)`, String(status));
    assert.equal(n.tone, 'danger');
  }
});

test('the hub\'s words are not repeated when they are only the status', () => {
  assert.equal(O.notice({ ok: false, status: 500, message: 'HTTP 500' }, t).text, '<out_notSaved>: <out_server>');
  assert.equal(O.notice({ ok: false, status: 418, message: '' }, t).text, '<out_notSaved>: <out_failed> 418');
});

test('no answer at all is the connection, never a refusal', () => {
  assert.deepEqual(O.notice({ ok: false, offline: true }, t), { text: '<out_notSaved>: <out_offline>', tone: 'danger', icon: 'wifi-off' });
});

test('a long hub message is cut so the notice stays one glance', () => {
  const n = O.notice({ ok: false, status: 400, message: 'x'.repeat(500) }, t);
  assert.ok(n.text.length < 260, String(n.text.length));
});

test('only a write a person made is announced; reads and the session\'s own calls are not', () => {
  assert.equal(O.announces('POST', '/owner/supplies', {}), true);
  assert.equal(O.announces('DELETE', '/owner/x', {}), true);
  assert.equal(O.announces('GET', '/owner/stock', {}), false);
  assert.equal(O.announces('POST', '/owner/x', { quiet: true }), false);
  for (const p of ['/auth/refresh', '/auth/login', '/staff/refresh', '/staff/login', '/telemetry', '/owner/push/subscribe']) assert.equal(O.announces('POST', p, {}), false, p);
});
