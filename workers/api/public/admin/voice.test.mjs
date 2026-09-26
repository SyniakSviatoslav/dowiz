// node --test workers/api/public/admin/voice.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { planOf, needsReason, lineOf, ORDER_VERBS, STATES } from './voice-plan.js';

// The console's words, read as TEXT: i18n.js imports storage by an absolute
// URL node cannot load.
const SRC = readFileSync(new URL('./i18n.js', import.meta.url), 'utf8');
const at = l => SRC.indexOf(`  ${l}: {`);
const block = l => SRC.slice(at(l), l === 'uk' ? SRC.indexOf('\n};', at(l)) : SRC.indexOf('\n  },', at(l)));
const T = Object.fromEntries(['sq', 'en', 'uk'].map(l => [l, Object.fromEntries([...block(l).matchAll(/(\w+):'([^']*)'/g)].map(m => [m[1], m[2]]))]));
const t = k => T.en[k] ?? k;

test('voice: a confirmed order verb is the action the console\'s button posts', () => {
  for (const v of ORDER_VERBS) {
    const p = planOf({ verb: v, orderId: 'o/1' }, 'v1');
    assert.equal(p.path, '/owner/orders/o%2F1/action');
    assert.equal(p.body.action, v);
    assert.equal(p.body.location_id, 'v1');
  }
});

test('voice: a refusal or cancellation carries the reason typed; the others never do', () => {
  assert.deepEqual(planOf({ verb: 'reject', orderId: 'o1' }, 'v1', ' Out of stock ').body, { location_id: 'v1', action: 'reject', reason: 'Out of stock' });
  assert.deepEqual(planOf({ verb: 'cancel', orderId: 'o1' }, 'v1', '').body, { location_id: 'v1', action: 'cancel' });
  assert.deepEqual(planOf({ verb: 'confirm', orderId: 'o1' }, 'v1', 'ignored').body, { location_id: 'v1', action: 'confirm' });
  assert.equal(needsReason('reject') && needsReason('cancel'), true);
  assert.equal(needsReason('ready'), false);
});

test('voice: a dish off or back on sale is the dish save with only `available`', () => {
  assert.deepEqual(planOf({ verb: 'dish_off', args: { productId: 'marg' } }, 'v1'), { path: '/owner/products/marg', body: { location_id: 'v1', available: false } });
  assert.deepEqual(planOf({ verb: 'dish_on', args: { productId: 'marg' } }, 'v1'), { path: '/owner/products/marg', body: { location_id: 'v1', available: true } });
});

test('voice: the venue state is the header chip\'s own write', () => {
  for (const s of STATES) assert.deepEqual(planOf({ verb: 'venue', args: { state: s } }, 'v1'), { path: '/owner/location', body: { location_id: 'v1', status: s } });
});

test('voice: nothing is planned for a verb this console does not act on, or a malformed one', () => {
  for (const d of [null, {}, { verb: 'pickup', orderId: 'o1' }, { verb: 'confirm' }, { verb: 'dish_off', args: {} },
    { verb: 'venue', args: { state: 'party' } }, { verb: 'shift_open', orderId: '-' }]) {
    assert.equal(planOf(d, 'v1'), null, JSON.stringify(d));
  }
});

test('voice: "how many are waiting" is answered at once, anything else is not a line', () => {
  assert.equal(lineOf({ action: 'status', open: 4, waiting: 2 }, t), 'Open 4, waiting 2');
  assert.equal(lineOf({ action: 'ask', question: 'x' }, t), null);
  assert.equal(lineOf(null, t), null);
});

test('voice: every word the mic uses is in all three languages', () => {
  for (const k of ['voice', 'voiceConfirm', 'voiceStatus', 'voiceDenied', 'voiceOffline', 'voiceAsking', 'voiceFailed']) {
    for (const l of ['sq', 'en', 'uk']) assert.ok(T[l][k], `${l}.${k}`);
    assert.notEqual(T.sq[k], T.en[k], k);
    assert.notEqual(T.uk[k], T.en[k], k);
  }
});
