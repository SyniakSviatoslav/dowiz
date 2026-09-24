// The room's binders on the migrated markup: a tap on a /lib/ui control still
// reaches the same handler with the same `data-*`, and the write that leaves is
// the one it always was. Rendered with the real components into the node shim,
// tapped by calling the binder's own handler with the element as the target.
// `node --test workers/api/public/room/*.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { renderRound, bindRound } from './sheet.js';
import { renderAdd, bindAdd } from './menu.js';
import { renderPay, bindPay } from './pay.js';
import { renderTransfer, bindTransfer } from './transfer.js';
import { parseCaps } from './logic.js';
import { useTranslator } from '../lib/ui/core.js';
import { render } from '../lib/ui/dom-shim.mjs';

const t = k => `«${k}»`;
useTranslator(t);

/// An app context whose wire is a recorder: every write lands.
function app(S, answer = { landed: true, data: {} }) {
  const writes = [], toasts = [];
  const c = {
    S: { caps: parseCaps('take_orders,take_payment,void,transfer'), sittings: [], currency: 'ALL', loc: 'v1', ...S },
    t, locale: () => 'en', tableOf: () => '4', sitting: () => null, statusWord: s => s,
    write: async (path, body, tag) => { writes.push({ path, body, tag }); return answer; },
    toast: m => toasts.push(m), reload: async () => {}, render: () => { c.rendered = (c.rendered || 0) + 1; },
  };
  return { c, writes, toasts };
}
const tap = (root, sel) => root.onclick({ target: root.querySelector(sel) });
const round = (o = {}) => ({ id: 'r1', seq: 7, status: 'CONFIRMED', subtotal: 1000, total: 1000, payment_status: 'unpaid',
  items: [{ name: 'Maki', product_id: 'p1', quantity: 2, unit_price: 500 }], ...o });

test('round: "more" sends set_qty on the line the waiter saw, quoting the version', async () => {
  const { c, writes } = app();
  const r = round();
  const { root } = render(renderRound(c, r));
  bindRound(c, root, r);
  await tap(root, '[data-act="qty"][data-q="3"]');
  assert.deepEqual(writes[0].body, { location_id: 'v1', base_seq: 7, ops: [{ op: 'set_qty', line: 0, qty: 3 }] });
  assert.equal(writes[0].tag, 'amend:r1');
});

test('round: a comp opens the reason chips; a reason sends it with the words', async () => {
  const { c, writes } = app();
  const r = round();
  let { root } = render(renderRound(c, r));
  bindRound(c, root, r);
  await tap(root, '[data-act="ask"][data-op="comp"]');
  assert.deepEqual(c.S.ask, { op: 'comp', line: 0, kind: null });
  ({ root } = render(renderRound(c, r)));
  bindRound(c, root, r);
  await tap(root, '[data-act="reason"][data-r="mistake"]');
  assert.equal(writes[0].body.ops[0].op, 'comp');
  assert.ok(writes[0].body.reason, 'the reason goes with it');
  assert.equal(c.S.ask, null);
});

test('add: taps build the basket; send adds exactly what was tapped, never a price', async () => {
  const { c, writes } = app({ menu: [{ name: 'S', products: [{ id: 'p1', name: 'Maki', price: 500 }] }] });
  const r = round();
  const { amend } = await import('./sheet.js');
  const draw = () => { const { root } = render(renderAdd(c)); bindAdd(c, root, r, amend); return root; };
  await tap(draw(), '[data-act="pick"][data-id="p1"]');
  await tap(draw(), '[data-act="pick"][data-id="p1"]');
  await tap(draw(), '[data-act="unpick"][data-id="p1"]');
  assert.deepEqual(c.S.basket, { p1: 1 });
  await tap(draw(), '[data-act="send"]');
  assert.deepEqual(writes[0].body.ops, [{ op: 'add', product_id: 'p1', modifier_ids: [], quantity: 1 }]);
  assert.equal(JSON.stringify(writes[0].body).includes('price'), false);
});

test('pay: the card chip and a typed tip reach the payment body', async () => {
  const { c, writes } = app();
  const r = round({ status: 'READY' });
  const { root } = render(renderPay(c, r));
  bindPay(c, root, r);
  tap(root, '[data-act="method"][data-v="card"]');
  assert.equal(c.S.payForm.method, 'card');
  root.oninput({ target: { dataset: { in: 'tip' }, value: '50', selectionStart: 2 } });
  await root.onsubmit({ preventDefault() {} });
  assert.deepEqual(writes[0].body, { location_id: 'v1', amount: 1000, method: 'card', base_seq: 7, tip: 50 });
  assert.equal(writes[0].path, '/staff/orders/r1/pay');
});

test('pay: a refused amount is said, and nothing is sent', async () => {
  const { c, writes, toasts } = app();
  const r = round();
  const { root } = render(renderPay(c, r));
  bindPay(c, root, r);
  root.oninput({ target: { dataset: { in: 'amount' }, value: 'x', selectionStart: 1 } });
  await root.onsubmit({ preventDefault() {} });
  assert.equal(writes.length, 0);
  assert.deepEqual(toasts, ['«badAmount»']);
});

test('transfer: pick a line and a destination row, send moves exactly those', async () => {
  const src = round({ id: 'a', items: [{ name: 'x', quantity: 1, unit_price: 1 }, { name: 'y', quantity: 1, unit_price: 2 }] });
  const dst = round({ id: 'b', seq: 2 });
  const { c, writes } = app({ sittings: [{ sitting_id: 's1', table: '4', rounds: [src] }, { sitting_id: 's2', table: '9', rounds: [dst] }] });
  const draw = () => { const { root } = render(renderTransfer(c, src)); bindTransfer(c, root, src); return root; };
  await tap(draw(), '[data-act="line"][data-line="1"]');
  await tap(draw(), '[data-act="to"][data-id="b"]');
  await tap(draw(), '[data-act="send"]');
  assert.equal(writes[0].path, '/staff/orders/a/transfer');
  assert.deepEqual(writes[0].body.lines, [1]);
});
