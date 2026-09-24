// The room's sub-views on /lib/ui -- add from the menu, open a table, take a
// payment, move lines or a sitting, the till -- rendered in node against the
// real components, plus two checks across every room file: each icon it names
// is drawn by /lib/icons.css, and each learning anchor is listed in
// docs/learn/anchors-room.txt (and each listed one still exists).
// `node --test workers/api/public/room/*.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { renderAdd } from './menu.js';
import { renderOpen } from './open.js';
import { renderPay, paidNote } from './pay.js';
import { renderTransfer, renderMoveSitting } from './transfer.js';
import { renderTillScreen, bindTill, moveChoice } from './till.js';
import { parseCaps } from './logic.js';
import { useTranslator } from '../lib/ui/core.js';
import { XSS, injected, render, fire } from '../lib/ui/dom-shim.mjs';

const t = k => `«${k}»`;
useTranslator(t);
const ctx = (S = {}) => ({ S: { caps: parseCaps(''), sittings: [], currency: 'ALL', loc: 'v1', ...S }, t, locale: () => 'en',
  tableOf: r => r?.fulfilment?.table || '', sitting: () => null, api: () => new Promise(() => {}) });
const $ = (h, s) => render(h).root.querySelector(s);
const $$ = (h, s) => render(h).root.querySelectorAll(s);
const primaries = h => $$(h, '.ui-btn--primary,.ui-btn--success').length;
const MENU = [{ name: 'Sushi', products: [{ id: 'p1', name: 'Maki', price: 500 }, { id: 'p2', name: 'Nigiri', price: 700, available: false }] }];

test('add: before the menu a skeleton, a failure announced, a miss said as such', () => {
  assert.ok($(renderAdd(ctx({ menu: null })), '.ui-skel-wrap'));
  const failed = renderAdd(ctx({ menu: null, menuError: 'noSlug' }));
  assert.ok($(failed, '.ui-empty[role="alert"] [data-t="noSlug"]'));
  assert.ok($(renderAdd(ctx({ menu: MENU, menuQ: 'zzz' })), '.ui-empty [data-t="noMatch"]'));
});

test('add: every dish is a tap, a sold-out one is off, the basket counts and sends', () => {
  const h = renderAdd(ctx({ menu: MENU, basket: { p1: 2 } }));
  const picks = $$(h, '[data-act="pick"]');
  assert.deepEqual(picks.map(p => [p.getAttribute('data-id'), p.disabled]), [['p1', false], ['p2', true]]);
  assert.equal($(h, '[data-act="pick"][data-id="p1"] .count').textContent, '2');
  assert.ok($(h, '[data-act="unpick"][data-id="p1"]'));
  assert.equal($(h, '[data-act="unpick"][data-id="p2"]'), null);
  assert.equal($(h, 'input[data-in="q"]').getAttribute('type'), 'search');
  const send = $(h, '[data-act="send"]');
  assert.ok(!send.disabled);
  assert.ok($(renderAdd(ctx({ menu: MENU })), '[data-act="send"]').disabled, 'nothing picked, nothing to send');
  assert.equal(primaries(h), 1);
  assert.deepEqual(injected(renderAdd(ctx({ menu: [{ name: XSS, products: [{ id: XSS, name: XSS, price: 1 }] }], menuQ: XSS }))), [], 'names and the query escaped');
});

test('open: the table field sits under the heading, the picker below it', () => {
  const h = renderOpen(ctx({ menu: MENU, openTable: 'T4' }));
  const table = $(h, 'input[data-in="table"]');
  assert.equal(table.getAttribute('value'), 'T4');
  assert.equal(table.getAttribute('maxlength'), '24');
  assert.equal(table.getAttribute('data-tour'), 'open.table');
  assert.ok(h.indexOf('data-in="table"') > h.indexOf('«openTable»') && h.indexOf('data-in="table"') < h.indexOf('data-in="q"'));
  assert.deepEqual(injected(renderOpen(ctx({ menu: MENU, openTable: XSS }))), []);
});

const round = (o = {}) => ({ id: 'r1', status: 'READY', seq: 2, total: 2500, payment_status: 'unpaid', fulfilment: { table: '7' }, ...o });

test('pay: the owed figure, currency and method chips, amount and tip, one take', () => {
  const h = renderPay(ctx(), round());
  assert.ok($(h, '.ui-stat--hero .money'));
  assert.equal($(h, '[data-act="currency"][aria-pressed="true"]').getAttribute('data-v'), 'ALL');
  assert.equal($(h, '[data-act="method"][aria-pressed="true"]').getAttribute('data-v'), 'cash');
  assert.ok($(h, '[data-act="method"][data-v="card"]'), 'the walk taps card');
  for (const k of ['amount', 'tip']) assert.equal($(h, `input[data-in="${k}"]`).getAttribute('name'), k);
  assert.equal($(h, 'input[data-in="amount"]').getAttribute('value'), '2500');
  assert.equal($(h, 'input[data-in="rate"]'), null, 'the bill\'s own currency needs no rate');
  assert.ok($(h, 'form[data-form="pay"] button[type="submit"]'));
  assert.equal(primaries(h), 1);
});

test('pay: another currency asks the board rate and previews; wallet asks its code', () => {
  const S = { payForm: { id: 'r1', currency: 'EUR', method: 'wallet', amount: '10', rate: '100', tip: '', wallet: '' } };
  const h = renderPay(ctx(S), round());
  assert.ok($(h, 'input[data-in="rate"]'));
  assert.ok($(h, 'input[data-in="wallet"][required]'));
  assert.ok($(h, '.preview [data-act="fill"]'));
  assert.ok(/≈/.test($(h, '.preview').textContent));
  const noRate = renderPay(ctx({ payForm: { ...S.payForm, rate: '' } }), round());
  assert.ok($(noRate, '.preview [data-t]') === null && /«rateNeeded»/.test($(noRate, '.preview').textContent));
  assert.equal($(noRate, '[data-act="fill"]'), null);
});

test('pay: no currency is a stop; a paid round has no form; the note is escaped once', () => {
  assert.ok($(renderPay(ctx({ currency: null }), round()), '.ui-empty[role="alert"]'));
  const paid = renderPay(ctx({ payNote: XSS }), round({ payment_status: 'paid' }));
  assert.equal($(paid, 'form[data-form="pay"]'), null);
  assert.deepEqual(injected(paid), []);
  assert.ok(render(paid).root.textContent.includes('<img'), 'the note reads as text, not markup');
  const c = ctx();
  const note = paidNote(c, { order: { payment_status: 'paid', payments: [{ amount_in_order_currency: 1950, tip: 50 }] } }, 'EUR', 2000);
  assert.ok(note.startsWith('«taken»: ') && note.includes('«offTheBill»') && note.endsWith('«paidInFull»'));
  assert.ok(!/&[a-z]+;/.test(note), 'plain text: the alert escapes it');
});

test('transfer: lines and destinations are pressed rows; send needs a destination', () => {
  const src = round({ id: 'a', status: 'CONFIRMED', items: [{ name: 'Maki', quantity: 1, unit_price: 500 }, { name: XSS, quantity: 2, unit_price: 100 }] });
  const dst = round({ id: 'b', status: 'CONFIRMED', fulfilment: { table: '9' } });
  const S = { caps: parseCaps('take_orders,transfer'), sittings: [{ sitting_id: 's1', table: '7', rounds: [src] }, { sitting_id: 's2', table: '9', rounds: [dst] }],
    moveForm: { id: 'a', lines: [1], to: 'b' } };
  const h = renderTransfer(ctx(S), src);
  assert.deepEqual($$(h, '[data-act="line"]').map(b => [b.getAttribute('data-line'), b.getAttribute('aria-pressed')]), [['0', 'false'], ['1', 'true']]);
  assert.equal($(h, '[data-act="to"][data-id="b"]').getAttribute('aria-pressed'), 'true');
  assert.ok(!$(h, '[data-act="send"]').disabled);
  assert.deepEqual(injected(h), []);
  const alone = renderTransfer(ctx({ ...S, sittings: [S.sittings[0]] }), src);
  assert.ok($(alone, '[data-act="send"]').disabled && $(alone, '.ui-empty [data-t="noTargets"]'));
});

test('move a sitting: one table field and its submit', () => {
  const h = renderMoveSitting(ctx(), { sitting_id: 's1', table: XSS });
  assert.ok($(h, 'form[data-form="moveSit"] input[name="table"][required]'));
  assert.ok($(h, 'form[data-form="moveSit"] button[type="submit"]'));
  assert.deepEqual(injected(h), []);
});

test('till: the four forms, the cash move as two radio groups, the tips section', () => {
  const c = ctx({ caps: parseCaps('open_till'), till: null });
  const h = renderTillScreen(c);
  for (const f of ['open', 'move', 'count', 'close']) assert.ok($(h, `form[data-form="${f}"]`), f);
  assert.equal($$(h, 'input[name="f_ALL"],input[name="f_EUR"]').length, 2, 'a float per drawer currency');
  assert.equal($(h, '#tillDir [aria-checked="true"]').getAttribute('data-value'), 'pay_in');
  assert.equal($(h, '#tillCur [aria-checked="true"]').getAttribute('data-value'), 'ALL');
  assert.equal($(h, 'select'), null, 'the currency is a radio group now');
  assert.ok($(h, 'input[name="sure"][type="checkbox"][required]'));
  assert.ok($(h, 'form[data-form="close"] .ui-btn--danger'));
  assert.ok($(h, '[data-tips] .ui-skel-wrap'), 'the tips say they are coming');
  const open = renderTillScreen(ctx({ till: { kind: 'till.opened', open: true, opened_at: 1, float: {} } }));
  assert.equal($(open, 'form[data-form="open"]'), null, 'an open drawer is not opened twice');
});

test('till: the cash move choice lives in state and follows the radio groups', () => {
  const S = { caps: parseCaps('open_till'), till: null, loc: '' };
  const c = { ...ctx(S) };
  c.S = { ...c.S, ...S };
  const { root } = render(renderTillScreen(c));
  bindTill(c, root);
  fire(root.querySelector('#tillDir [data-value="pay_out"]'), 'click');
  fire(root.querySelector('#tillCur [data-value="EUR"]'), 'click');
  assert.deepEqual(moveChoice(c.S), { dir: 'pay_out', currency: 'EUR' });
  assert.deepEqual(moveChoice({}), { dir: 'pay_in', currency: 'ALL' }, 'a fresh screen starts at pay in, the drawer\'s first currency');
});

const ROOM = new URL('.', import.meta.url);
const sources = readdirSync(ROOM).filter(f => /\.(js|html)$/.test(f) && f !== 'sw.js').map(f => [f, readFileSync(new URL(f, ROOM), 'utf8')]);

test('icons: every icon name the room asks for is drawn by /lib/icons.css', () => {
  const drawn = new Set([...readFileSync(new URL('../lib/icons.css', ROOM), 'utf8').matchAll(/\.ti-([a-z0-9-]+)/g)].map(m => m[1]));
  const used = new Set();
  for (const [, s] of sources) {
    for (const m of s.matchAll(/\bicon:\s*'([a-z0-9-]+)'/g)) used.add(m[1]);
    for (const m of s.matchAll(/\bti-([a-z0-9-]+)/g)) used.add(m[1]);
  }
  assert.ok(used.size > 10, 'found only ' + used.size);
  assert.deepEqual([...used].filter(n => !drawn.has(n)), [], 'icons that would draw nothing');
});

test('anchors: every data-tour in the room is listed with its file:line, and every listed one exists', () => {
  const listed = readFileSync(new URL('../../../../docs/learn/anchors-room.txt', ROOM), 'utf8')
    .split('\n').filter(l => l.trim() && !l.startsWith('#')).map(l => l.trim().split(/\s+/));
  // An anchor is a LITERAL string in the source (never built from parts), so
  // grep finds it: '<module>.<control>' with one of the room's module names.
  const ANCHOR = /["'](login|hud|nav|room|sitting|round|menu|open|guest|pay|transfer|move|till|floor)\.([a-zA-Z]+)["']/g;
  const inTree = new Map();
  for (const [f, s] of sources) s.split('\n').forEach((line, i) => {
    if (/^export const [A-Z_]+ = /.test(line)) return; // a constant (till-view's CLOSED kind), not an anchor
    for (const m of line.matchAll(ANCHOR)) { const id = `${m[1]}.${m[2]}`; if (!inTree.has(id)) inTree.set(id, `room/${f}:${i + 1}`); }
  });
  const ids = listed.map(([id]) => id);
  assert.equal(new Set(ids).size, ids.length, 'an anchor is listed twice');
  for (const [id, at] of listed) {
    assert.ok(inTree.has(id), `${id} is listed but no room file writes it`);
    const [file, line] = at.split(':');
    assert.ok(readFileSync(new URL('../../../../' + file, ROOM), 'utf8').split('\n')[Number(line) - 1].includes(id), `${id} is not on ${at}`);
  }
  for (const id of inTree.keys()) assert.ok(ids.includes(id), `${id} is in the tree and not listed`);
});
