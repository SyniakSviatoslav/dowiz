// node --test workers/api/public/lib/session-logic.test.mjs
// Background renewal (W-SESSION TZ 7): when a token is due, and a renewer that
// asks once, only while the page is visible, with injected clock and timers.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { claimsOf, expOf, roleOf, dueToRenew, createRenewer, RENEW_AHEAD_MS, RENEW_EVERY_MS } from './session-logic.js';

const b64 = o => Buffer.from(JSON.stringify(o)).toString('base64url');
const jwt = (claims) => `${b64({ alg: 'HS256', typ: 'JWT', kid: 'v1' })}.${b64(claims)}.sig`;
const T0 = 1_700_000_000_000;
const H = 60 * 60 * 1000;

test('the claims are read from the token without trusting them for anything but timing', () => {
  const t = jwt({ role: 'owner', sub: 'u1', exp: T0 + 24 * H });
  assert.equal(claimsOf(t).sub, 'u1');
  assert.equal(expOf(t), T0 + 24 * H);
  assert.equal(roleOf(t), 'owner');
  for (const bad of [null, '', 'x', 'a.b', 'a.!!!.c', `a.${b64([1])}.c`]) {
    assert.equal(claimsOf(bad), null, String(bad));
    assert.equal(expOf(bad), null, String(bad));
  }
});

test('a token is due inside the two-hour window, and an expired one is due too', () => {
  assert.equal(RENEW_AHEAD_MS, 2 * H);
  const t = jwt({ role: 'owner', exp: T0 + 24 * H });
  assert.equal(dueToRenew(t, T0), false);
  assert.equal(dueToRenew(t, T0 + 22 * H - 1), false);
  assert.equal(dueToRenew(t, T0 + 22 * H), true);
  assert.equal(dueToRenew(t, T0 + 25 * H), true);
  assert.equal(dueToRenew(null, T0), false, 'no token, nothing to renew');
});

test('the renewer ticks every five minutes, renews once when due and only while visible', async () => {
  assert.equal(RENEW_EVERY_MS, 5 * 60 * 1000);
  let now = T0, token = jwt({ role: 'owner', exp: T0 + 24 * H }), visible = true, renews = 0;
  const timers = [];
  const r = createRenewer({
    token: () => token,
    renew: async () => { renews++; token = jwt({ role: 'owner', exp: now + 24 * H }); return true; },
    now: () => now, visible: () => visible,
    setInterval: (f, ms) => { timers.push([f, ms]); return 1; }, clearInterval: () => { timers.length = 0; },
  });
  r.start();
  assert.equal(timers[0][1], RENEW_EVERY_MS);
  await r.tick();
  assert.equal(renews, 0, 'fresh: nothing');
  now = T0 + 23 * H;
  visible = false; await r.tick();
  assert.equal(renews, 0, 'hidden: a tab in the background does not spend the hub');
  visible = true; await r.tick();
  assert.equal(renews, 1, 'visible and due: renewed');
  await r.tick();
  assert.equal(renews, 1, 'the new token is fresh');
  r.stop();
  assert.equal(timers.length, 0);
});

test('a renewal that fails is not retried in the same tick and does not throw', async () => {
  let now = 1_700_000_000_000, calls = 0;
  const r = createRenewer({ token: () => jwt({ exp: now + 1000 }), renew: async () => { calls++; throw new Error('offline'); }, now: () => now, visible: () => true, setInterval: () => 1, clearInterval: () => {} });
  await r.tick(); await r.tick();
  assert.equal(calls, 2);
});
