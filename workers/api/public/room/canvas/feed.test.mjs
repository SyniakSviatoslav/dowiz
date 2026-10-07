// The canvas board's feed, both halves: `node --test workers/api/public/room/canvas/feed.test.mjs`
//
// feed.js WRITES what crates/dowiz-canvas src/board/feed.rs READS. The second half loads the
// shipped board.wasm in node (a fixed-advance txt_measure stands in for Canvas2D) and checks the
// Rust reader takes every record the JS writer makes -- the contract between the two languages,
// tested on the real module rather than on a re-statement of it.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { toFeed, clean, opens, ROLE, BUMPS } from './feed.js';

const NOW = 1_800_000_000_000;
const orders = [
  { id: 'ord_1', status: 'PENDING', created_at_ms: NOW - 120_000, fulfilment: { kind: 'dine_in', table: '4', note: 'pa\tqepë\n' }, kitchen: { seen: false },
    items: [{ name: 'Salmon nigiri', quantity: 2, station: 'sushi' }, { product_id: 'p9', quantity: 1, note: 'no onion' }] },
  { id: 'ord_2', status: 'READY', created_at_ms: NOW, scheduled_for_ms: NOW + 3_600_000, fulfilment: { kind: 'pickup' }, kitchen: { seen: true },
    items: [{ name: 'Lemonade', quantity: 1, station: 'bar' }] },
];
const sittings = [{ sitting_id: 's1', table: '4', rounds: [{ id: 'r1', status: 'PREPARING', total: 1500, payment_status: 'unpaid' }, { id: 'r2', status: 'PENDING', placed_by: 'guest', total: 700 }] }];

test('records: one T per order, one L per line, one R per sitting; no tab or newline leaks', () => {
  const f = toFeed({ orders, sittings, currency: 'ALL', locale: 'en', now: NOW, when: () => '19:00' });
  const recs = f.trim().split('\n').map(l => l.split('\t'));
  assert.deepEqual(recs.map(r => r[0]), ['T', 'L', 'L', 'T', 'L', 'R']);
  assert.deepEqual(recs[0], ['T', 'ord_1', 'PENDING', String(NOW - 120_000), '0', 't', '4', 'pa qepë ', '']);
  assert.deepEqual(recs[1], ['L', '2', '1', 'Salmon nigiri', '']);
  assert.deepEqual(recs[2], ['L', '1', '2', 'p9', 'no onion'], 'no station IS the kitchen; no name falls back to the product id');
  assert.equal(recs[3][3], String(NOW + 3_600_000), 'a ticket for later is measured from its hour');
  assert.equal(recs[3][8], '19:00');
  assert.match(recs[5][4], /1.?500/, 'the guest round still waiting is not on the bill (sittingDue)');
  assert.equal(recs[5][5], '1', 'a guest round waits');
  assert.equal(recs[5][6], 'PREPARING,PENDING');
  assert.equal(clean('a\tb\r\nc'), 'a b c');
});

test('caps open the pass and the floor; bump words are the console route\'s', () => {
  assert.deepEqual(opens({ staff: { caps: 'advance' } }), [true, false]);
  assert.deepEqual(opens({ staff: { caps: 'take_orders,take_payment' } }), [false, true]);
  assert.deepEqual(opens(null), [false, false]);
  assert.deepEqual(BUMPS, ['confirm', 'preparing', 'ready', 'collected'], 'board/model.rs Bump order');
  assert.equal(ROLE['counter-manager'], 1);
});

async function board() {
  const bytes = await readFile(new URL('./board.wasm', import.meta.url));
  const env = { txt_measure: (p, n, px) => n * px * 0.55 };
  const { instance } = await WebAssembly.instantiate(bytes, { env });
  const ex = instance.exports;
  const put = s => new TextEncoder().encodeInto(s, new Uint8Array(ex.memory.buffer, ex.inbuf(), ex.inbuf_cap())).written;
  const stats = () => Array.from(new Uint32Array(ex.memory.buffer, ex.stats(), 10));
  const tours = () => new TextDecoder().decode(new Uint8Array(ex.memory.buffer, ex.inbuf(), ex.tour_list())).split('\n').filter(Boolean);
  return { ex, put, stats, tours };
}

test('the shipped module reads every record the writer makes', async () => {
  const { ex, put, stats } = await board();
  ex.init(390, 844, 1, 0);
  ex.session(1, 1, 1, 1);
  const n = ex.feed(put(toFeed({ orders, sittings, currency: 'ALL', locale: 'sq', now: NOW })));
  assert.equal(n, 2);
  ex.frame(NOW);
  const s = stats();
  assert.deepEqual({ tickets: s[8], tables: s[9], dropped: s[5], bad: s[6], small: s[7], overflow: s[3] }, { tickets: 2, tables: 1, dropped: 0, bad: 0, small: 0, overflow: 0 });
});

test('the room lessons\' anchors are scene nodes (learn W1, W2; the kitchen board lesson)', async () => {
  const { ex, put, tours } = await board();
  ex.init(390, 844, 1, 0);
  ex.frame(NOW);
  const out = tours();
  for (const a of ['hud.sync', 'hud.lang', 'login.email', 'login.password', 'login.submit', 'login.claimToggle']) assert.ok(out.includes(a), a);
  ex.session(1, 1, 1, 1);
  ex.feed(put(toFeed({ orders, sittings, now: NOW })));
  ex.frame(NOW);
  const inn = tours();
  for (const a of ['hud.sync', 'hud.lang', 'room.refresh', 'room.signout', 'room.role', 'kitchen.station', 'kitchen.board', 'kitchen.seen', 'kitchen.bump', 'kitchen.reject']) assert.ok(inn.includes(a), a);
});
