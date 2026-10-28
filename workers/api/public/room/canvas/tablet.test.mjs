// The kitchen tablet's writer in node, on the SHIPPED module (./tablet.wasm, built by
// crates/bebop-wasm/tablet.sh): `node --test workers/api/public/room/canvas/tablet.test.mjs`.
// The hub is played by an echo: it adopts what the tablet sent and answers nothing else -- the
// smallest reply the contract allows (services/operations/stock/glue.rs). The full glue against
// the hub's own code is crates/bebop-wasm/tests/tablet.rs and hubdo/glue_turn/tests.rs.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { surface, tablet, ZERO } from './tablet.js';

async function mod() {
  const { instance } = await WebAssembly.instantiate(await readFile(new URL('./tablet.wasm', import.meta.url)), {});
  return surface(instance);
}

const mem = () => { let v; return { get: async () => v, set: async x => { v = x; } }; };

test('record offline, sync, and the image lands on the tip the hub answered', async () => {
  const m = await mod();
  const store = mem();
  const sent = [];
  const post = async (path, body) => {
    sent.push({ path, body });
    return { tip: body.tip, tail: body.tail.map((_, i) => i) };
  };
  const t = tablet({ mod: m, store, post, loc: 'v1', now: () => 1_791_000_000_000 });
  assert.equal(await t.record('{"k":"received","item":"rice","qty":5}'), false);
  assert.equal(await t.record('{"k":"wasted","item":"rice","qty":1,"reason":"spoiled","by":"chef"}'), false);
  assert.equal(await t.pending(), 2);
  const r = await t.sync();
  assert.equal(sent[0].path, '/staff/stock/glue');
  assert.deepEqual(Object.keys(sent[0].body).sort(), ['base', 'ctx', 'location_id', 'tail', 'tip']);
  assert.equal(sent[0].body.base, ZERO);
  assert.equal(sent[0].body.tail.length, 2);
  assert.equal((await store.get()).base, r.tip, 'the next base is the hub tip');
  assert.equal(await t.pending(), 0, 'nothing left to send');
});

test('a refusal is thrown with its code; a reply that misses the tip is refused', async () => {
  const m = await mod();
  assert.throws(() => m.record(new Uint8Array(0), 'kitchen', '{"k":"teleported"}', 0), e => e.code === 41);
  const { image } = m.record(new Uint8Array(0), 'kitchen', '{"k":"received","item":"rice","qty":5}', 0);
  const body = m.tail(image, 'kitchen', ZERO);
  assert.throws(() => m.rebase(image, ZERO, false, 1, { tip: ZERO.replace(/^0/, '1'), tail: [0] }), e => e.code === 44);
  assert.ok(m.rebase(image, ZERO, false, 1, { tip: body.tip, tail: [0] }).length > 0, 'the twin');
});

test('the shelf the stock sheet steps through comes from the tablet, offline', async () => {
  const m = await mod();
  const t = tablet({ mod: m, store: mem(), post: async () => { const e = new Error('no network'); e.offline = true; throw e; }, loc: 'v1', now: () => 1 });
  assert.deepEqual(await t.items(), [], 'never synced: nothing to step through');
  await t.record('{"k":"received","item":"salmon","qty":5}');
  await t.record('{"k":"wasted","item":"salmon","qty":2,"reason":"spoiled","by":"k1"}');
  assert.deepEqual(await t.items(), [['salmon', 3]]);
  await assert.rejects(t.sync(), e => e.offline, 'offline: the sync says so and keeps the records');
  assert.equal(await t.pending(), 2);
});

// S2b: the order rail. The hub is played by an echo of the contract's answer
// (services/operations/orders/glue.rs); the rebase against the hub's own deciders is
// hubdo/glue_turn/orders_tests.rs and crates/bebop-wasm/tests/tablet_orders.rs.
const KEYS = ['at', 'from', 'next', 'oid', 'op', 'order'];

test('order taps offline are decided by the order machine, queued, and leave only after the hub answered', async () => {
  const m = await mod();
  const store = mem();
  const sent = [];
  let up = false;
  const post = async (path, body) => {
    if (!up) { const e = new Error('no network'); e.offline = true; throw e; }
    sent.push({ path, body });
    return { sent: body.tail.length, merged: body.tail.length, applied: body.tail.length, held: [], notes: 0 };
  };
  const t = tablet({ mod: m, store, post, loc: 'v1', now: () => 1_791_000_000_000 });
  await t.record('{"k":"received","item":"rice","qty":5}');
  const rail = await t.orderRail([{ id: 'o1', status: 'CONFIRMED' }, { id: 'o2', status: 'PENDING', kitchen: { seen: { at: 1 } } }]);
  assert.equal(rail.tickets.length, 2);
  assert.equal(await t.order('o1', 'PREPARING'), true);
  assert.equal(await t.order('o2', ''), false, 'seen already: nothing queued');
  await assert.rejects(t.order('o1', 'CONFIRMED'), e => e.code === 46 && /IllegalTransitionError/.test(e.message));
  assert.equal(await t.ordersPending(), 1);
  assert.equal(await t.pending(), 1, 'the stock record survives beside the rail');
  await assert.rejects(t.syncOrders(), e => e.offline);
  assert.equal(await t.ordersPending(), 1, 'offline: the tap stays queued');
  up = true;
  const r = await t.syncOrders();
  assert.equal(r.merged, 1);
  assert.equal(sent[0].path, '/staff/orders/glue');
  assert.deepEqual(Object.keys(sent[0].body).sort(), ['ctx', 'location_id', 'tail']);
  for (const line of sent[0].body.tail) assert.deepEqual(Object.keys(JSON.parse(line)).sort(), KEYS, 'no-tracking: op keys only');
  assert.equal(await t.ordersPending(), 0);
  const s = await store.get();
  assert.equal(JSON.parse(s.rail).tickets.find(x => x.id === 'o1').status, 'PREPARING', 'the rail keeps what the tablet showed');
});

test('two_venues: two slugs, two rows, no shared tap', async () => {
  const m = await mod();
  const rows = new Map();
  const keyed = k => ({ get: async () => rows.get(k), set: async v => { rows.set(k, v); } });
  const off = async () => { const e = new Error('no network'); e.offline = true; throw e; };
  const a = tablet({ mod: m, store: keyed('v1:kitchen'), post: off, loc: 'v1', now: () => 5 });
  const b = tablet({ mod: m, store: keyed('v2:kitchen'), post: off, loc: 'v2', now: () => 5 });
  await a.orderRail([{ id: 'o1', status: 'CONFIRMED' }]);
  await b.orderRail([{ id: 'p1', status: 'CONFIRMED' }]);
  await a.order('o1', 'PREPARING');
  await assert.rejects(b.order('o1', 'PREPARING'), e => e.code === 46, 'another venue\'s order is not on this rail');
  assert.equal(await a.ordersPending(), 1);
  assert.equal(await b.ordersPending(), 0);
  assert.ok(!rows.get('v2:kitchen').rail.includes('o1'), 'not one byte of v1 in v2\'s row');
});
