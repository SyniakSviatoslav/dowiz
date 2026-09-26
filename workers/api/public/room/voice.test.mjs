// node --test workers/api/public/room/voice.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { draftOf, applyNow, planOf, readbackOf, micButton, mountVoice } from './voice.js';
import { readFileSync } from 'node:fs';

// THE WORDS, READ FROM i18n.js AS TEXT: it imports the console's dictionary by
// an absolute URL node cannot load, so the three blocks are parsed here.
const SRC = readFileSync(new URL('./i18n.js', import.meta.url), 'utf8');
const block = l => SRC.slice(SRC.indexOf(`  ${l}: {`), SRC.indexOf('\n  },', SRC.indexOf(`  ${l}: {`)));
const T = Object.fromEntries(['sq', 'en', 'uk'].map(l => [l, Object.fromEntries([...block(l).matchAll(/(\w+): '([^']*)'/g)].map(m => [m[1], m[2]]))]));
const t = k => T.en[k] ?? k;

test('voice: the round being built is the Open view\'s table and basket, and only that view\'s', () => {
  assert.deepEqual(draftOf({ view: 'open', openTable: '5', basket: { marg: 2, cola: 0, tira: 1 } }),
    { table: '5', items: [{ product_id: 'marg', quantity: 2 }, { product_id: 'tira', quantity: 1 }] });
  assert.deepEqual(draftOf({ view: 'open' }), { table: '', items: [] });
  // The Add view's basket belongs to a round already placed: not a draft.
  assert.equal(draftOf({ view: 'add', openTable: '5', basket: { marg: 1 } }), null);
});

test('voice: "open table" opens the view on this phone and writes nothing', () => {
  const S = { view: 'room', basket: { old: 1 } };
  assert.equal(applyNow(S, { action: 'open', table: '5', guests: 4 }, t), 'Table 5 · 4 guests');
  assert.deepEqual([S.view, S.openTable, S.basket], ['open', '5', {}]);
  assert.equal(applyNow({}, { action: 'open', table: '7', guests: null }, t), 'Table 7');
});

test('voice: a dish joins the round being built, on the table the hub named', () => {
  const S = { view: 'open', openTable: '5', basket: { cola: 1 } };
  assert.equal(applyNow(S, { action: 'draft_add', table: '5', productId: 'cola', name: 'Cola', quantity: 2 }, t), '2 × Cola');
  assert.deepEqual(S.basket, { cola: 3 });
  // Another table: the view starts that table's round.
  const R = { view: 'room' };
  applyNow(R, { action: 'draft_add', table: '8', productId: 'marg', name: 'Margherita', quantity: 1 }, t);
  assert.deepEqual([R.view, R.openTable, R.basket], ['open', '8', { marg: 1 }]);
});

test('voice: the room at a glance, and nothing for an answer this file does not know', () => {
  assert.equal(applyNow({}, { action: 'status', open: 3, waiting: 1 }, t), '3 tables open, 1 waiting');
  assert.equal(applyNow({}, { action: 'ask' }, t), null);
});

test('voice: a confirmed add is the amend a tap sends, quoting the version read back', () => {
  const p = planOf({ verb: 'add', args: { orderId: 'r/1', baseSeq: 7, productId: 'marg', quantity: 2 } }, { loc: 'v1' });
  assert.deepEqual(p, { via: 'write', tag: 'amend:r/1', path: '/staff/orders/r%2F1/amend',
    body: { location_id: 'v1', base_seq: 7, ops: [{ op: 'add', product_id: 'marg', modifier_ids: [], quantity: 2 }] } });
});

test('voice: a confirmed payment is exactly the amount and method read back', () => {
  const p = planOf({ verb: 'pay', args: { orderId: 'r1', baseSeq: 3, amount: 1100, method: 'card' } }, { loc: 'v1' });
  assert.deepEqual(p, { via: 'write', tag: 'pay:r1', path: '/staff/orders/r1/pay', body: { location_id: 'v1', amount: 1100, method: 'card', base_seq: 3 } });
});

test('voice: a confirmed round is the Open view\'s placement, intents only', () => {
  const p = planOf({ verb: 'place', args: { table: '5', items: [{ product_id: 'marg', quantity: 2 }] } }, { slug: 'dubin-sushi' });
  assert.equal(p.via, 'place');
  assert.equal(p.path, '/public/locations/dubin-sushi/orders');
  assert.deepEqual(p.body.fulfilment, { kind: 'dine_in', table: '5' });
  assert.deepEqual(p.body.items, [{ product_id: 'marg', quantity: 2, modifier_ids: [] }]);
  assert.equal(JSON.stringify(p.body).includes('price'), false);
});

test('voice: an instruction this file does not know, or none, plans nothing', () => {
  assert.equal(planOf({ verb: 'confirm', orderId: 'o1' }, {}), null);
  assert.equal(planOf({ verb: 'add' }, {}), null);
  assert.equal(planOf(null, {}), null);
});

test('voice: a payment is read back with its amount, drawn by money.js', () => {
  const S = { currency: 'ALL' };
  const line = readbackOf({ verb: 'pay', readback: 'table 5: paid in cash', amount: 1500 }, S, 'en');
  assert.match(line, /^table 5: paid in cash · /);
  assert.match(line, /1,?500/);
  assert.equal(readbackOf({ verb: 'add', readback: 'add 1 × Cola to table 5' }, S, 'en'), 'add 1 × Cola to table 5');
  // A currency not yet known draws a dash, never a guess.
  assert.match(readbackOf({ verb: 'pay', readback: 'x', amount: 1500 }, {}, 'en'), /—$/);
});

test('voice: the mic is a design-system icon button carrying its lesson anchor', () => {
  const html = micButton();
  assert.match(html, /class="ui-iconbtn ui-iconbtn--plain"/);
  assert.match(html, /data-tour="hud\.voice"/);
  assert.match(html, /aria-pressed="false"/);
  assert.match(html, /ti-microphone/);
});

test('voice: no mic is drawn where the browser cannot recognise speech', () => {
  const header = { querySelector: () => null, insertBefore: () => { throw new Error('drawn'); } };
  assert.doesNotThrow(() => mountVoice({ t }, header, null));
  assert.doesNotThrow(() => mountVoice({ t }, null, null));
});

test('voice: every word the mic uses is in all three languages', () => {
  for (const k of ['voice', 'voiceConfirm', 'voiceOpened', 'voiceGuests', 'voiceStatus', 'voiceDenied', 'voiceOffline']) {
    for (const l of ['sq', 'en', 'uk']) assert.ok(T[l][k], `${l}.${k}`);
    assert.notEqual(T.sq[k], T.en[k], k);
    assert.notEqual(T.uk[k], T.en[k], k);
  }
});
