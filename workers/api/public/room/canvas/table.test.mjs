// The table sheet (CV1b), both halves: `node --test workers/api/public/room/canvas/table.test.mjs`
//
// table.js `rows` WRITES what crates/dowiz-canvas src/board/tsheet.rs READS, from the room's own
// rules (room/logic.js). The first tests hold the rows to those rules; the last loads the shipped
// board.wasm and checks the Rust reader takes every row the writer makes, with no row dropped,
// no record refused and no control under 44 px -- on the real module.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { rows, A, F, word, methodWord } from './table.js';
import { REASONS, METHODS, refusalKey } from '../logic.js';
import { W, WORDS } from './feed.js';

const sittings = [
  { sitting_id: 's1', table: '4', rounds: [
    { id: 'r1', seq: 3, status: 'PENDING', total: 1500, subtotal: 1500, payment_status: 'unpaid',
      items: [{ name: 'Salmon nigiri', quantity: 2, unit_price: 600 }, { name: 'Miso', quantity: 1, unit_price: 300, comped: true }] },
    { id: 'r2', seq: 1, status: 'PENDING', placed_by: 'guest', total: 700, subtotal: 700, items: [{ name: 'Ramen', quantity: 1, unit_price: 700 }] },
  ] },
  { sitting_id: 's2', table: '9', rounds: [{ id: 'r3', seq: 1, status: 'CONFIRMED', total: 800, subtotal: 800, items: [{ name: 'Gyoza', quantity: 1, unit_price: 800 }] }] },
];
const ctx = (caps, extra = {}) => ({ sittings, caps: new Set(caps), currency: 'ALL', locale: 'en', menu: null, menuErr: null, field: () => '', ...extra });
const T = (o = {}) => ({ id: 's1', view: 'sit', round: null, ask: null, pay: null, move: null, basket: {}, ...o });
const recs = text => text.trim().split('\n').map(l => l.split('\t'));
const acts = text => recs(text).filter(r => r[0] === 'b' || r[0] === 'B').map(r => [Number(r[1]), r[2], r[6]]);
const tours = text => recs(text).filter(r => r[0] === 'b' || r[0] === 'B' || r[0] === 'f').map(r => r[0] === 'f' ? r[4] : r[6]);

test('a waiter sees lines, sums and only what the caps allow (logic.js actionsFor)', () => {
  const waiter = rows(T(), ctx(['take_orders']));
  const r = recs(waiter);
  assert.deepEqual(r[0], ['H', String(A.back), String(W('KTable')), '4', 'sit']);
  assert.ok(r.some(x => x[0] === 'h' && x[2] === '#1' && x[3] === 'PENDING'), 'a round heading names its status for Rust to word');
  assert.ok(r.some(x => x[0] === 'i' && x[1] === '1× Miso' && x[3] === String(W('Comped'))), 'a comped line says so');
  const t = tours(waiter);
  for (const a of ['round.less', 'round.more', 'round.remove', 'round.add', 'round.tableField', 'guest.confirm', 'guest.reject', 'round.transfer', 'sitting.move']) assert.ok(t.includes(a), a);
  assert.ok(!t.includes('round.pay'), 'no take_payment, no pay button');
  assert.ok(!t.includes('round.comp'), 'no void, no comp');
  const cashier = tours(rows(T(), ctx(['take_orders', 'take_payment', 'void'])));
  assert.ok(cashier.includes('round.pay') && cashier.includes('round.comp'));
  assert.equal(rows(T({ id: 'gone' }), ctx(['take_orders'])), '', 'a table that left the room closes its sheet');
});

test('remove asks why with the five reasons; other opens a text field', () => {
  const ask = { round: 'r1', op: 'remove', line: 0, kind: 'other' };
  const t = rows(T({ ask }), ctx(['take_orders']));
  const reasons = acts(t).filter(a => a[0] === A.reason).map(a => a[1]);
  assert.deepEqual(reasons, ['mistake', 'guest_changed', 'unavailable', 'dropped', 'other']);
  assert.ok(recs(t).some(r => r[0] === 'f' && r[1] === String(F.text) && r[4] === 'round.reasonText'));
});

test('pay: owed, currencies, seven methods, the foreign rate and the take button', () => {
  const pay = { currency: 'EUR', method: 'wallet', note: '' };
  const field = f => ({ [F.rate]: '97.50', [F.amount]: '10' })[f] || '';
  const t = rows(T({ view: 'pay', round: 'r1', pay }), ctx(['take_payment'], { field }));
  const r = recs(t);
  assert.deepEqual(acts(t).filter(a => a[0] === A.cur).map(a => a[1]), ['ALL', 'EUR']);
  assert.deepEqual(acts(t).filter(a => a[0] === A.method).map(a => a[1]), ['cash', 'card', 'cheque', 'transfer', 'gift_card', 'wallet', 'other']);
  for (const a of ['pay.rate', 'pay.amount', 'pay.tip', 'pay.wallet', 'pay.fill', 'pay.submit']) assert.ok(tours(t).includes(a), a);
  assert.ok(r.some(x => x[0] === 'k' && x[1] === String(W('OffTheBill')) && /975/.test(x[2])), 'the preview converts with the typed board rate');
  assert.ok(r.some(x => x[0] === 'B' && x[3] === String(W('TakeN')) && /10/.test(x[4])));
});

test('move lines: lines to pick, the rounds that may take them, send', () => {
  const t = rows(T({ view: 'move', round: 'r1', move: { lines: [0], to: 'r3' } }), ctx(['take_orders']));
  const a = acts(t);
  assert.deepEqual(a.filter(x => x[0] === A.line).map(x => x[1]), ['0', '1']);
  assert.deepEqual(a.filter(x => x[0] === A.to).map(x => x[1]), ['r2', 'r3'], 'every other round before the kitchen, any sitting');
  assert.ok(recs(t).some(r => r[0] === 'B' && r[1] === String(A.to) && r[5] === 's'), 'the picked target is pressed');
});

test('add: the menu, a search that filters, a basket and its send', () => {
  const menu = [{ name: 'Sushi', products: [{ id: 'p1', name: 'Salmon nigiri', price: 600 }, { id: 'p2', name: 'Tuna', price: 700, available: false }] },
    { name: 'Soup', products: [{ id: 'p3', name: 'Miso', price: 300 }] }];
  const t = rows(T({ view: 'add', round: 'r1', basket: { p1: 2 } }), ctx(['take_orders'], { menu, field: f => (f === F.text ? 'sal' : '') }));
  const a = acts(t);
  assert.deepEqual(a.filter(x => x[0] === A.pick).map(x => x[1]), ['p1'], 'the search keeps Salmon only');
  assert.ok(a.some(x => x[0] === A.unpick && x[1] === 'p1'));
  assert.ok(recs(t).some(r => r[0] === 'B' && r[1] === String(A.addSend) && r[4] === '2'));
  const loading = rows(T({ view: 'add', round: 'r1' }), ctx(['take_orders']));
  assert.ok(recs(loading).some(r => r[0] === 'p' && r[2] === String(W('Loading'))));
});

test('every computed word exists: reasons, methods, refusals, transfer errors', () => {
  const names = [...REASONS.map(r => word('R', r)), ...METHODS.map(methodWord),
    ...['changedReload', 'kitchenHasIt', 'roundPaid', 'alreadyThere', 'notAllLines', 'notHere', 'pickRound', 'pickLines'].map(k => word('', k))];
  for (const n of names) assert.ok(WORDS.includes(n), n);
  // Every key refusalKey can answer is in that list (logic.js is the one source of the keys).
  for (const [st, m] of [[409, 'changed while you were editing'], [409, 'the kitchen has the'], [409, 'is paid'], [400, 'is anywhere but table'], [409, 'no lines left'], [404, '']]) {
    assert.ok(WORDS.includes(word('', refusalKey(st, m))), m);
  }
});

test('every word the sheet names exists in Rust (no blank label)', async () => {
  const src = await readFile(new URL('./table.js', import.meta.url), 'utf8');
  const named = [...src.matchAll(/W\('([A-Za-z]+)'\)|'([A-Z][A-Za-z]+)'/g)].map(m => m[1] || m[2]).filter(n => /^[A-Z]/.test(n));
  for (const n of new Set(named)) assert.ok(WORDS.includes(n), `${n} is not a Str in lang.rs`);
});

test('the shipped module reads every row and keeps every control tappable', async () => {
  const bytes = await readFile(new URL('./board.wasm', import.meta.url));
  const { instance } = await WebAssembly.instantiate(bytes, { env: { txt_measure: (p, n, px) => n * px * 0.55 } });
  const ex = instance.exports;
  const put = s => new TextEncoder().encodeInto(s, new Uint8Array(ex.memory.buffer, ex.inbuf(), ex.inbuf_cap())).written;
  const stats = () => Array.from(new Uint32Array(ex.memory.buffer, ex.stats(), 12));
  ex.init(390, 844, 1, 1);
  ex.session(1, 0, 1, 1);
  const views = [
    rows(T({ ask: { round: 'r1', op: 'comp', line: 0, kind: 'other' } }), ctx(['take_orders', 'take_payment', 'void'])),
    rows(T({ view: 'pay', round: 'r1', pay: { currency: 'EUR', method: 'wallet', note: '1 000' } }), ctx(['take_payment'], { field: () => '97.5' })),
    rows(T({ view: 'move', round: 'r1', move: { lines: [], to: null } }), ctx(['take_orders'])),
    rows(T({ view: 'moveSit' }), ctx(['take_orders'])),
  ];
  for (const v of views) {
    const n = ex.sheet(put(v));
    ex.frame(0);
    const s = stats();
    assert.equal(n, v.trim().split('\n').length, 'every row read');
    assert.deepEqual({ rows: s[10], refused: s[11], small: s[7], overflow: s[3] }, { rows: n, refused: 0, small: 0, overflow: 0 });
  }
  ex.sheet(0);
  assert.equal(stats()[10], 0, 'closed');
});
