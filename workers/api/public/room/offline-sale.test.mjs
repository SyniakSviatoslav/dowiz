// The offline cash sale's tablet half, called for real (W-OFFSALE).
// `node --test workers/api/public/room/offline-sale.test.mjs`
//
// THE PARITY TEST reads the SAME fixture the Rust pricer's test reads
// (`services/orders/offline_sale/parity.json`): the tablet's offline price and
// the Worker's online price agree on every basket, refusals included.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { register } from 'node:module';
import { priceOffline, productsOf, basketLines, saleBody, receiptLines, deadlineOf, DEADLINE_MS, MAX_QTY } from './offline-sale.js';
import { LANGS } from '../lib/langs.js';
import { renderRoom } from './screens.js';
import { parseCaps } from './logic.js';
import { useTranslator } from '../lib/ui/core.js';
import { render } from '../lib/ui/dom-shim.mjs';

// The room's dictionary imports the console's, which imports `/store/...` by
// its site path: answered from the files beside this one (pass.test.mjs).
const PUBLIC = new URL('../', import.meta.url).href;
register('data:text/javascript,' + encodeURIComponent(`const P = ${JSON.stringify(PUBLIC)};
  export async function resolve(spec, ctx, next){
    if (/^\\/(admin|lib|store)\\//.test(spec)) return { url: P + spec.slice(1), shortCircuit: true };
    return next(spec, ctx);
  }`));
const { WORDS } = await import('./sale-i18n.js');
const { renderSellConfirm, renderReceipt, renderSell } = await import('./sell.js');

const FIXTURE = JSON.parse(fs.readFileSync(new URL('../../src/services/orders/offline_sale/parity.json', import.meta.url), 'utf8'));

test('parity: the tablet prices every basket of the shared fixture as the Worker does', () => {
  assert.ok(FIXTURE.baskets.length >= 10, 'the fixture lost its baskets');
  for (const b of FIXTURE.baskets) {
    const got = priceOffline(FIXTURE.products, b.lines);
    const shown = got.ok ? { ok: true, units: got.units, total: got.total } : { ok: false, refusal: got.refusal };
    assert.deepEqual(shown, b.expect, b.name);
  }
});

test('the menu read becomes one product map; the basket a stable list', () => {
  const m = productsOf([{ products: [{ id: 'a', price: 1 }] }, { products: [{ id: 'b', price: 2 }, null] }]);
  assert.deepEqual(Object.keys(m), ['a', 'b']);
  assert.deepEqual(basketLines({ b: 2, a: 1, z: 0 }), [['a', 1], ['b', 2]]);
});

test('integer money only: an unsafe product refuses rather than rounds', () => {
  const r = priceOffline({ x: { id: 'x', price: Number.MAX_SAFE_INTEGER, available: true } }, [['x', 2]]);
  assert.deepEqual([r.ok, r.refusal], [false, 'quantity']);
  assert.equal(priceOffline({ x: { id: 'x', price: 12.5, available: true } }, [['x', 1]]).refusal, 'no_price');
  assert.equal(MAX_QTY, 99);
  assert.equal(priceOffline({}, [['constructor', 1]]).refusal, 'unknown', 'an inherited key is not a dish');
  const many = Object.fromEntries(Array.from({ length: 51 }, (_, i) => [`d${i}`, { id: `d${i}`, price: 1, available: true }]));
  assert.equal(priceOffline(many, Object.keys(many).map(k => [k, 1])).refusal, 'quantity', 'more lines than the server takes');
  assert.equal(priceOffline(many, Object.keys(many).slice(0, 50).map(k => [k, 1])).total, 50, 'fifty is the bound, not past it');
  assert.equal(priceOffline({ x: { id: 'x', price: 1, available: true, name: 'n'.repeat(130) } }, [['x', 1]]).lines[0].name.length, 120);
});

test("the body is the server SaleIn, field for field (deny_unknown_fields)", () => {
  const priced = priceOffline(FIXTURE.products, [['p_water', 2]]);
  const b = saleBody({ loc: 'qa-durres', key: 'k-0001-abcd', soldAt: 1000, currency: 'ALL', priced, menuVersion: 7 });
  assert.deepEqual(Object.keys(b).sort(), ['currency', 'lines', 'location_id', 'menu_version', 'method', 'sale_key', 'sold_at_ms', 'total']);
  assert.deepEqual(b.lines, [{ product_id: 'p_water', quantity: 2, unit_price: 150, name: 'QA Water' }]);
  assert.equal(b.total, 300); assert.equal(b.method, 'cash');
  assert.equal('menu_version' in saleBody({ loc: 'l', key: 'k-00000001', soldAt: 1, currency: 'ALL', priced, menuVersion: null }), false);
});

test('the deadline is 48 hours from the sale, to the millisecond', () => {
  assert.equal(DEADLINE_MS, 172_800_000);
  assert.equal(deadlineOf(1_000), 172_801_000);
});

const words = l => ({ title: 'QA', total: WORDS[l].saleTotal, cash: WORDS[l].saleCash, ref: WORDS[l].saleRef, noNivf: WORDS[l].saleNoNivf, noNivfSq: WORDS[l].saleNoNivfSq });
const SALE = { sale_key: 'abcdef12-3456', sold_at_ms: 5, currency: 'ALL', total: 300, lines: [{ product_id: 'p', quantity: 2, unit_price: 150, name: 'QA Water' }] };

test("the receipt says pa NIVF in Albanian always, and in the reader language beside it", () => {
  const sq = receiptLines(SALE, words('sq'), a => `${a} L`, () => 'T');
  assert.ok(sq.includes('pa NIVF — do të fiskalizohet brenda 48 orëve'));
  assert.equal(sq.filter(l => l.includes('NIVF')).length, 1, 'Albanian once, not twice');
  assert.ok(sq.includes('2 x QA Water  300 L') && sq.some(l => l.endsWith('300 L')) && sq.includes('Ref. abcdef12'));
  for (const l of ['en', 'uk', 'ru']) {
    const r = receiptLines(SALE, words(l), a => `${a}`, () => 'T');
    assert.equal(r.filter(x => x.includes('NIVF')).length, 2, `${l}: the law in sq and the reader's line`);
    assert.ok(r.some(x => x.includes('48')), l);
  }
});

test('the words: every language of the set, the same keys, no typographic quotes', () => {
  assert.deepEqual(Object.keys(WORDS).sort(), [...LANGS].sort());
  const keys = Object.keys(WORDS.en).sort();
  for (const l of LANGS) assert.deepEqual(Object.keys(WORDS[l]).sort(), keys, l);
  const src = fs.readFileSync(new URL('./sale-i18n.js', import.meta.url), 'utf8') + fs.readFileSync(new URL('./sell.js', import.meta.url), 'utf8');
  assert.ok(!/[‘’“”]/.test(src), 'a typographic quote takes the room down (DOWIZ-COMMON-RULES 11)');
});

// ── the screens, rendered in node against the real components ─────────────
const t = k => `«${k}»`;
useTranslator(t);
const ctx = S => ({ S: { caps: parseCaps('take_orders,take_payment'), sittings: [], currency: 'ALL', at: 1, ...S }, t, statusWord: s => s, locale: () => 'en' });
const $$ = (h, s) => render(h).root.querySelectorAll(s);

test('the room offers the cash sale only while offline, and only to a payment holder', () => {
  const sell = h => $$(h, '[data-act="sell"]').length;
  assert.equal(sell(renderRoom(ctx({ offline: true }))), 1);
  assert.equal($$(renderRoom(ctx({ offline: true })), '.ui-btn--primary').length, 1, 'offline: the cash sale is the one main action');
  assert.equal(sell(renderRoom(ctx({ offline: false }))), 0, 'online: a table is opened and paid as before');
  assert.equal(sell(renderRoom(ctx({ offline: true, caps: parseCaps('take_orders') }))), 0, 'no take_payment, no sale');
});

test('the picker without a saved menu says so; with one it draws the dishes and the cash-only line', () => {
  assert.equal($$(renderSell(ctx({ menu: null })), '.ui-empty').length, 1);
  const h = renderSell(ctx({ menu: [{ name: 'Drinks', products: [{ id: 'p_water', name: 'QA Water', price: 150, available: true }] }], menuAt: Date.now() }));
  assert.equal($$(h, '[data-act="pick"][data-id="p_water"]').length, 1);
  assert.ok(h.includes('«cashOnly»') || h.includes('cashOnly'));
});

test('the confirmation: each line, the total, ONE primary action that takes the cash', () => {
  const priced = priceOffline(FIXTURE.products, [['p_water', 2], ['p_miso', 1]]);
  const h = renderSellConfirm(ctx({ sellDraft: priced }));
  assert.equal($$(h, '.sale-line').length, 2);
  assert.equal($$(h, '[data-act="sellCommit"]').length, 1);
  assert.equal($$(h, '.ui-btn--primary').length, 1);
  assert.ok($$(h, '[data-tour]').map(e => e.getAttribute('data-tour')).includes('sale.total'));
});

test('the receipt screen prints the receipt and is escaped', () => {
  const h = renderReceipt(ctx({ lastSale: { ...SALE, lines: [{ ...SALE.lines[0], name: '<img src=x>' }] } }));
  assert.equal($$(h, 'pre.receipt').length, 1);
  assert.ok(!h.includes('<img src=x>'), 'a dish name is escaped');
  assert.equal($$(h, '[data-act="salePrint"]').length + $$(h, '[data-act="saleDone"]').length, 2);
});

test('the drawer is not closed while unsent offline sales are in it; with none it is', async () => {
  const { renderTillScreen, bindTill } = await import('./till.js');
  const run = async unsent => {
    const writes = [], toasts = [];
    const c = { ...ctx({ caps: parseCaps('open_till'), till: null, loc: '', unsentSales: unsent }), api: () => new Promise(() => {}),
      toast: m => toasts.push(m), render: () => {}, write: async (path, body) => { writes.push(path); return { landed: false, queued: true }; } };
    const { root } = render(renderTillScreen(c));
    bindTill(c, root);
    const btn = root.querySelector('form[data-form="close"] button[type="submit"]');
    const form = { dataset: { form: 'close' }, elements: { sure: { checked: true } }, querySelector: () => btn };
    await root.onsubmit({ preventDefault() {}, target: form });
    return { writes, toasts };
  };
  const held = await run(1);
  assert.deepEqual(held.writes, [], 'no close was sent');
  assert.ok(held.toasts.some(m => String(m).includes('saleUnsentClose')), held.toasts.join('|'));
  assert.deepEqual((await run(0)).writes, ['/staff/till/close'], 'the positive twin: with nothing unsent the close goes');
});
