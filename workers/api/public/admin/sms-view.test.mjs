// The SMS screen's pure parts, in node (W-SMS): the status the owner reads and
// the body the hub takes. `node --test workers/api/public/admin/sms-view.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { statusOf, editBody } from './sms-view.js';
import { WORDS } from './sms-words.js';

test('the status says off, what is missing, failing or working', () => {
  assert.deepEqual(statusOf({ on: false }, {}), ['', 'off']);
  assert.deepEqual(statusOf({ on: true, missing: 'sms_missing_secret' }, {}), ['warn', 'sms_missing_secret']);
  assert.deepEqual(statusOf({ on: true }, { last_err: { at_ms: 20 }, last_ok_ms: 10 }), ['bad', 'sms_failing']);
  assert.deepEqual(statusOf({ on: true }, { last_err: { at_ms: 10 }, last_ok_ms: 20 }), ['ok', 'sms_live'], 'a success after the failure');
});

test('the edit sends the secret only when typed, and a sane daily limit', () => {
  const b = editBody({ on: true, provider: 'smsgate', url: ' ', user: ' GATE ', secret: '', from: '', daily: 'x' });
  assert.deepEqual(b, { on: true, provider: 'smsgate', url: '', user: 'GATE', from: '', daily: 60 });
  assert.equal('secret' in b, false, 'no secret typed = the stored one is kept');
  assert.equal(editBody({ secret: 'pw', daily: '25' }).secret, 'pw');
  assert.equal(editBody({ daily: '25' }).daily, 25);
});

test('every language has every word, and every word the hub can send has a translation', () => {
  const keys = Object.keys(WORDS.en);
  for (const l of ['sq', 'uk', 'ru']) assert.deepEqual(Object.keys(WORDS[l]).sort(), [...keys].sort(), l);
  for (const k of ['sms_missing_secret', 'sms_missing_user', 'sms_missing_from', 'sms_why_auth', 'sms_why_offline', 'sms_why_network', 'sms_why_refused', 'sms_why_busy', 'sms_why_provider']) {
    assert.ok(keys.includes(k), k);
  }
  for (const [l, w] of Object.entries(WORDS)) for (const [k, v] of Object.entries(w)) {
    assert.ok(!/[‘’“”]/.test(v), `${l}.${k} has a typographic quote`);
  }
});
