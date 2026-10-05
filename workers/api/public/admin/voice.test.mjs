// node --test workers/api/public/admin/voice.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { planOf, needsReason, lineOf, assistPath, ORDER_VERBS, STATES, countSession, COUNT_IDLE_MS } from './voice-plan.js';
import { LANGS } from '../lib/langs.js';

// The console's words, read as TEXT: i18n.js imports storage by an absolute
// URL node cannot load.
// Russian lives in its own file (admin/i18n-ru.js), read whole.
const SRC = readFileSync(new URL('./i18n.js', import.meta.url), 'utf8');
const RU_SRC = readFileSync(new URL('./i18n-ru.js', import.meta.url), 'utf8');
const at = l => SRC.indexOf(`  ${l}: {`);
const block = l => l === 'ru' ? RU_SRC : SRC.slice(at(l), SRC.indexOf('\n  },', at(l)));
const T = Object.fromEntries(LANGS.map(l => [l, Object.fromEntries([...block(l).matchAll(/(\w+):'([^']*)'/g)].map(m => [m[1], m[2]]))]));
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

test('voice: every word the mic uses is in every language', () => {
  for (const k of ['voice', 'voiceConfirm', 'voiceStatus', 'voiceDenied', 'voiceOffline', 'voiceAsking', 'voiceFailed']) {
    for (const l of LANGS) assert.ok(T[l][k], `${l}.${k}`);
    for (const l of LANGS.filter(l => l !== 'en')) assert.notEqual(T[l][k], T.en[k], `${l}.${k}`);
  }
});

test('voice: the kitchen\'s shelf verbs are the stock route, venue in the query, only the fields it takes', () => {
  assert.deepEqual(planOf({ verb: 'receive', args: { itemId: 'salmon', qty: 4000 } }, 'v 1'),
    { path: '/owner/stock/received?location_id=v%201', body: { item: 'salmon', qty: 4000 } });
  assert.deepEqual(planOf({ verb: 'waste', args: { itemId: 'soy', qty: 200, reason: 'spoiled' } }, 'v1'),
    { path: '/owner/stock/wasted?location_id=v1', body: { item: 'soy', qty: 200, reason: 'spoiled' } });
  for (const d of [{ verb: 'receive', args: { itemId: 'x' } }, { verb: 'receive', args: { itemId: 'x', qty: 1.5 } },
    { verb: 'waste', args: { itemId: 'x', qty: 1 } }, { verb: 'waste', args: { qty: 1, reason: 'spoiled' } }]) {
    assert.equal(planOf(d, 'v1'), null, JSON.stringify(d));
  }
});

test('voice: a question from the kitchen goes to the kitchen\'s assistant, the owner\'s to the owner\'s', () => {
  assert.equal(assistPath(true, 'v 1'), '/staff/assist?location_id=v%201');
  assert.equal(assistPath(false, 'v1'), '/owner/assist');
});

test('voice count (P14): a confirmed line joins the open count session through the count sheet\'s own route', () => {
  assert.deepEqual(planOf({ verb: 'count', args: { itemId: 'salmon', observed: 2300 } }, 'v 1', '', 'vc_1'),
    { path: '/owner/stock/count?location_id=v%201', body: { session: 'vc_1', lines: [{ item: 'salmon', observed: 2300 }] } });
  assert.deepEqual(planOf({ verb: 'count', args: { itemId: 'nori', observed: 0 } }, 'v1', '', 'vc_1').body.lines, [{ item: 'nori', observed: 0 }]);
  for (const d of [{ verb: 'count', args: { itemId: 'x' } }, { verb: 'count', args: { itemId: 'x', observed: 1.5 } },
    { verb: 'count', args: { itemId: 'x', observed: -1 } }, { verb: 'count', args: { observed: 1 } }]) {
    assert.equal(planOf(d, 'v1', '', 'vc_1'), null, JSON.stringify(d));
  }
  assert.equal(planOf({ verb: 'count', args: { itemId: 'x', observed: 1 } }, 'v1', '', ''), null, 'no open session, no line');
});

test('voice count (P14): the PROPOSAL alone adds nothing -- only the confirmation the token buys does', () => {
  // What POST /api/voice answers to "count salmon 2,3 kg": a read-back and a token.
  const proposal = { understood: true, needsConfirmation: true, verb: 'count', readback: 'counted: 2300 g Salmon', token: 't.x', itemId: 'salmon', observed: 2300 };
  assert.equal(planOf(proposal, 'v1', '', 'vc_1'), null);
});

test('voice count (P14): one session while the lines keep coming, a new one after the idle gap', () => {
  const a = countSession(null, 1000);
  assert.equal(a.id, 'vc_1000');
  const b = countSession(a, 1000 + COUNT_IDLE_MS - 1);
  assert.equal(b.id, 'vc_1000');
  const c = countSession(b, b.at + COUNT_IDLE_MS);
  assert.equal(c.id, `vc_${b.at + COUNT_IDLE_MS}`);
});
