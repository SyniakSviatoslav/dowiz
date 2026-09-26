// The W-WIRE screens in node: every former orphan is CALLED by a control, with
// the body the hub reads. `node --test workers/api/public/admin/wire-dom.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
import { Document, Element, XSS, injected } from '../lib/ui/dom-shim.mjs';

// THE BROWSER SEAM (as kitchen-dom.test.mjs): the console's core is a fake
// reading globalThis.__wf; every other /admin/ or /lib/ path is the real file.
const PUBLIC = new URL('../', import.meta.url).href;
const FAKES = {
  '/admin/core.js': `const F = () => globalThis.__wf;
    import * as ui from '/lib/ui/index.js';
    import { check } from '/admin/parts.js';
    export const esc = ui.esc, store = { loc: 'V1', t: 'tok' }, LANGS = ['sq', 'en', 'uk'];
    export const $ = (s, r = document) => r.querySelector(s), $$ = (s, r = document) => [...r.querySelectorAll(s)];
    export const icon = n => '<i class="ti ti-' + n + '"></i>';
    export const t = k => '<' + k + '>';
    export const S = new Proxy({}, { get: (_, k) => F().S[k] });
    export const toast = m => F().toast.push(String(m));
    export const api = (path, opts = {}) => F().reply(opts.method || 'GET', path, opts.body);
    export const post = (path, body) => F().reply('POST', path, body);
    export const withLoc = (b = {}) => ({ location_id: 'V1', ...b });
    export const busy = (el, fn) => fn();
    export const confirm = async () => (F().yes ? { ok: true, reason: '' } : null);
    export const sheet = html => { document.querySelector('#sheetIn').innerHTML = html; };
    export const moneyEl = n => '<span data-money="' + n + '">' + n + '</span>';
    export const baseCurrency = () => 'ALL';
    export const clock = () => '12:00', day = () => '26 Sep';
    export const hydrate = () => {}, retranslate = () => {};
    export const switchEl = (id, on, key, hintKey) => check({ id, checked: on, key, hintKey });`,
  '/admin/i18n.js': `export const T = { sq: {}, en: {}, uk: {} };`,
};
register('data:text/javascript,' + encodeURIComponent(`const F = ${JSON.stringify(FAKES)}, P = ${JSON.stringify(PUBLIC)};
  export async function resolve(spec, ctx, next){
    if (F[spec]) return { url: 'data:text/javascript,' + encodeURIComponent(F[spec]), shortCircuit: true };
    if (/^\\/(admin|lib)\\//.test(spec)) return { url: P + spec.slice(1), shortCircuit: true };
    return next(spec, ctx);
  }`));

Object.defineProperty(Element.prototype, 'value', {
  get(){ return this.getAttribute('value') ?? ''; }, set(v){ this.setAttribute('value', v); }, configurable: true });
Object.defineProperty(Element.prototype, 'checked', {
  get(){ return this.hasAttribute('checked'); }, set(v){ v ? this.setAttribute('checked', '') : this.removeAttribute('checked'); }, configurable: true });

const tick = async (n = 4) => { for (let i = 0; i < n; i++) await new Promise(r => setTimeout(r, 0)); };

/// A page whose hub answers `routes[METHOD path-prefix]`; every call is kept.
function page(routes = {}, extra = {}){
  const doc = new Document();
  globalThis.document = doc;
  globalThis.location = { hostname: 'dubin.dowiz.org' };
  const box = doc.createElement('div'); box.setAttribute('id', 'sheetIn'); doc.body.appendChild(box);
  const f = { S: { orders: [], venue: { slug: 'dubin', default_locale: 'sq', tz: 'Europe/Tirane' }, ...(extra.S || {}) }, toast: [], calls: [], yes: extra.yes ?? true,
    reply: async (method, path, body) => {
      f.calls.push([method, path, body]);
      const key = Object.keys(routes).filter(k => `${method} ${path}`.startsWith(k)).sort((a, b) => b.length - a.length)[0];
      const a = key ? routes[key] : {};
      if (a instanceof Error) throw a;
      return typeof a === 'function' ? a(body, path) : a;
    } };
  globalThis.__wf = f;
  return { doc, box, f, host(){ const h = doc.createElement('div'); doc.body.appendChild(h); return h; } };
}
const sent = (f, method, prefix) => f.calls.filter(c => c[0] === method && c[1].startsWith(prefix));

test('messages: every thread listed with its unread number; a reply and a read mark go back as the venue', async () => {
  const th = { threads: [{ id: 'ord_abcdefgh1', unread: 2, atMs: 5, count: 2, last: { from: 'CUSTOMER', body: XSS } }], unread: 2 };
  const convo = { messages: [{ from: 'CUSTOMER', kind: 'TEXT', seq: 1, body: 'no wasabi', sentAtMs: 1 }, { from: 'CUSTOMER', kind: 'TEXT', seq: 2, body: 'thanks', sentAtMs: 2 }],
    unread: { VENUE: 2 } };
  const p = page({ 'GET /owner/threads': th, 'GET /public/locations/dubin/threads/': convo, 'POST /public/locations/dubin/threads/': { id: 'm' } });
  const M = await import('./threads.js');
  assert.equal(await M.unread(), 2);
  M.openMessages(); await tick();
  const row = p.box.querySelector('[data-thread="ord_abcdefgh1"]');
  assert.ok(row, 'the thread is a tappable row');
  assert.match(row.outerHTML, /2/);
  assert.deepEqual(injected(p.box.innerHTML), [], 'a customer message is text, never markup');
  row.onclick(); await tick();
  const read = sent(p.f, 'POST', '/public/locations/dubin/threads/ord_abcdefgh1/messages');
  assert.equal(read.length, 1);
  assert.deepEqual({ kind: read[0][2].kind, readThrough: read[0][2].readThrough }, { kind: 'READ', readThrough: 2 });
  p.box.querySelector('#w-reply').value = ' on its way ';
  await p.box.querySelector('#wSend').onclick(); await tick();
  const all = sent(p.f, 'POST', '/public/locations/dubin/threads/ord_abcdefgh1/messages');
  assert.equal(all[1][2].kind, 'TEXT');
  assert.equal(all[1][2].body, 'on its way');
  assert.ok(all[1][2].clientId, 'idempotent');
  // TWIN: an empty reply sends nothing.
  p.box.querySelector('#w-reply').value = '  ';
  await p.box.querySelector('#wSend').onclick();
  assert.equal(sent(p.f, 'POST', '/public/locations/dubin/threads/').filter(c => c[2].kind === 'TEXT').length, 1);
});

test('messages: nothing yet says so, a refusal is shown, and unread is 0 when the hub is away', async () => {
  const p = page({ 'GET /owner/threads': { threads: [], unread: 0 } });
  const M = await import('./threads.js');
  const h = p.host(); await M.mount(h);
  assert.match(h.innerHTML, /w_noThreads/);
  page({ 'GET /owner/threads': new Error('forbidden') });
  const h2 = globalThis.document.body.appendChild(globalThis.document.createElement('div'));
  await M.mount(h2);
  assert.match(h2.innerHTML, /forbidden/);
  assert.equal(await M.unread(), 0);
});

test('wallet: the owner records a top-up with its payment reference, in minor units', async () => {
  const p = page({ 'GET /owner/customers': { customers: [{ key: 'k1', name: 'Arben' }, { key: 'k2', phone: '069' }] },
    'GET /public/locations/dubin/wallet?': { balanceMinor: 500, currency: 'ALL', conserved: true },
    'GET /public/locations/dubin/wallet/statement': { statement: [{ kind: 'TOP_UP', memo: 'rcpt 1', minor: 500 }] },
    'POST /public/locations/dubin/wallet/topup': { id: 'tx', replayed: false } });
  const W = await import('./wallet.js');
  W.openWallets(); await tick();
  p.box.querySelector('#w-find').value = 'arb';
  p.box.querySelector('#w-find').oninput();
  const rows = p.box.querySelectorAll('[data-i]');
  assert.equal(rows.length, 1, 'the search narrows the list');
  rows[0].onclick(); await tick();
  assert.match(p.box.innerHTML, /rcpt 1/);
  // A top-up without a reference is refused here, before the hub.
  p.box.querySelector('#w-amt').value = '1 500';
  await p.box.querySelector('#wTop').onclick();
  assert.equal(sent(p.f, 'POST', '/public/locations/dubin/wallet/topup').length, 0);
  assert.match(p.f.toast.at(-1), /w_needRef/);
  p.box.querySelector('#w-amt').value = 'lots';
  p.box.querySelector('#w-ref').value = 'R-7';
  await p.box.querySelector('#wTop').onclick();
  assert.match(p.f.toast.at(-1), /w_badAmount/);
  p.box.querySelector('#w-amt').value = '1 500';
  p.box.querySelector('#w-ref').value = 'R-7';
  await p.box.querySelector('#wTop').onclick(); await tick();
  const [call] = sent(p.f, 'POST', '/public/locations/dubin/wallet/topup');
  assert.deepEqual({ user: call[2].user, amountMinor: call[2].amountMinor, currency: call[2].currency, providerRef: call[2].providerRef },
    { user: 'k1', amountMinor: 1500, currency: 'ALL', providerRef: 'R-7' });
});

test('older orders: the archives are listed newest first and a date range narrows one', async () => {
  const at = Date.UTC(2026, 8, 10, 12);  // noon UTC = 14:00 in Tirana
  const p = page({ 'GET /owner/history?location_id=V1&archive=': { orders: [{ id: 'o1', created_at_ms: at, total: 700, status: 'DELIVERED' }] },
    'GET /owner/history': { archives: ['log@3', 'log@9'], keepMs: 30 * 86_400_000 } });
  const H = await import('./history.js');
  H.openHistory(); await tick(6);
  assert.match(sent(p.f, 'GET', '/owner/history?location_id=V1&archive=')[0][1], /archive=log%409/, 'newest first');
  assert.match(p.box.innerHTML, /#o1/);
  p.box.querySelector('#w-from').value = '2026-09-11';
  p.box.querySelector('#w-from').onchange();
  assert.doesNotMatch(p.box.querySelector('#wOrders').innerHTML, /#o1/);
  const chip = p.box.querySelector('[data-arch="log@3"]');
  chip.onclick(); await tick();
  assert.match(sent(p.f, 'GET', '/owner/history?location_id=V1&archive=').at(-1)[1], /log%403/);
  // TWIN: no archives says so.
  const q = page({ 'GET /owner/history': { archives: [], keepMs: 0 } });
  const h = q.host(); await H.mount(h);
  assert.match(h.innerHTML, /w_noArchives/);
});

test('tax: a percentage is saved as ppm, the schedule as json, the fiscal start as ms; untouched keys are not written', async () => {
  const p = page({ 'GET /owner/settings': { values: { 'tax.default_ppm': '200000' }, known: [{ key: 'tax.prices_include', default: 'true' },
    { key: 'notify.whatsapp.status', default: 'off' }, { key: 'notify.order.late_min', default: '20' }] }, 'GET /owner/fiscal': { sendEnabled: false } });
  const X = await import('./tax.js');
  X.openTax(); await tick(6);
  assert.match(p.box.innerHTML, /w_sendOff/, 'the platform switch is said');
  assert.equal(p.box.querySelector('#w-rate').value, '20');
  p.box.querySelector('#w-rate').value = '6,5';
  p.box.querySelector('#wAddRow').onclick();
  p.box.querySelector('[data-sd="0"]').value = '2027-01-01';
  p.box.querySelector('[data-sp="0"]').value = '10';
  p.box.querySelector('#w-since').value = '2026-10-01';
  await p.box.querySelector('#wTaxSave').onclick(); await tick();
  const w = Object.fromEntries(sent(p.f, 'POST', '/owner/settings').map(c => [c[2].key, c[2].value]));
  assert.equal(w['tax.default_ppm'], '65000');
  assert.equal(JSON.parse(w['tax.schedule'])[0].ppm, 100000);
  assert.equal(Number(w['fiscal.since_ms']) > 0, true);
  assert.deepEqual(Object.keys(w).sort(), ['fiscal.since_ms', 'tax.default_ppm', 'tax.schedule'], 'defaults are not written');
  // TWIN: a bad rate writes nothing.
  const before = p.f.calls.length;
  p.box.querySelector('#w-rate').value = '120';
  await p.box.querySelector('#wTaxSave').onclick();
  assert.equal(p.f.calls.length, before);
  assert.match(p.f.toast.at(-1), /w_badRate/);
});

test('tax writes: each refusal named, each value exact', async () => {
  const { writes } = await import('./tax.js');
  const ok = { rate: '20', fee: '', incl: true, rows: [], since: '', wa: true, late: '15', tz: 'Europe/Tirane' };
  assert.deepEqual(writes(ok).out.find(x => x[0] === 'notify.whatsapp.status'), ['notify.whatsapp.status', 'on']);
  assert.equal(writes({ ...ok, fee: 'x' }).bad, 'w_badRate');
  assert.equal(writes({ ...ok, since: '2026-02-31' }).bad, 'w_badDate');
  assert.equal(writes({ ...ok, late: '-3' }).bad, 'w_badMinutes');
  assert.deepEqual(writes({ ...ok, rows: [{ day: 'x', pct: '5' }] }), { bad: 'w_badSchedule', rows: [1] });
  assert.deepEqual(writes({ ...ok, rate: '', incl: false, wa: false }).out.slice(0, 2), [['tax.default_ppm', ''], ['tax.prices_include', 'false']]);
});

test('category names: only the changed fields are sent, per language, through /owner/i18n', async () => {
  const menu = l => ({ categories: [{ id: 'c1', name: l === 'en' ? 'Rolls' : 'Role' }] });
  const p = page({ 'GET /public/locations/dubin/menu?locale=sq': menu('sq'), 'GET /public/locations/dubin/menu?locale=en': menu('en'),
    'GET /public/locations/dubin/menu?locale=uk': menu('uk'), 'POST /owner/i18n': { ok: true } });
  const C = await import('./cat-i18n.js');
  C.openCategoryWords(); await tick(6);
  assert.equal(p.box.querySelector('[data-cw="c1"][data-lang="en"]').value, 'Rolls');
  assert.equal(p.box.querySelector('[data-cw="c1"][data-lang="uk"]').value, '', 'the base name is no translation');
  await p.box.querySelector('#cwSave').onclick();
  assert.match(p.f.toast.at(-1), /w_nothingChanged/);
  p.box.querySelector('[data-cw="c1"][data-lang="uk"]').value = 'Роли';
  await p.box.querySelector('#cwSave').onclick(); await tick();
  const [call] = sent(p.f, 'POST', '/owner/i18n');
  assert.deepEqual(call[2], { location_id: 'V1', entries: [{ entity: 'category', id: 'c1', locale: 'uk', field: 'name', value: 'Роли' }] });
  assert.deepEqual(C.changed([{ id: 'c', locale: 'en', value: '' }], { c: { en: 'x' } }), [{ entity: 'category', id: 'c', locale: 'en', field: 'name', value: '' }], 'clearing is a change');
});

test('look: a preset is applied only on a yes, and the logo removal asks first', async () => {
  const p = page({ 'GET /owner/branding': { brand: { paper: '#000000', typePair: 'classic' }, presets: [{ id: 'sea', label: 'Sea', primary: '#123456', paper: '#ffffff' }] },
    'POST /owner/branding/preset': { ok: true }, 'POST /owner/logo/clear': { ok: true } }, { yes: false });
  const B = await import('./brand-extra.js');
  B.openBrandExtra(); await tick(6);
  await p.box.querySelector('[data-preset="sea"]').onclick(); await tick();
  assert.equal(sent(p.f, 'POST', '/owner/branding/preset').length, 0, 'no yes, no change');
  p.f.yes = true;
  await p.box.querySelector('[data-preset="sea"]').onclick(); await tick();
  assert.deepEqual(sent(p.f, 'POST', '/owner/branding/preset')[0][2], { preset: 'sea' });
  await tick(4);
  await p.box.querySelector('#wLogoClear').onclick(); await tick();
  assert.equal(sent(p.f, 'POST', '/owner/logo/clear').length, 1);
});

test('data & safety: each repair asks, runs once, and shows the hub its answer', async () => {
  const p = page({ 'POST /owner/customers/reforget': { reforgotten: { declared: 0 } }, 'POST /owner/customers/rekey': { changed: 3 },
    'POST /owner/hub/rotate': new Error('nothing to archive') });
  const D = await import('./safety.js');
  D.openSafety();
  await p.box.querySelector('#w-rekey').onclick(); await tick();
  assert.equal(sent(p.f, 'POST', '/owner/customers/rekey?location_id=V1').length, 1);
  assert.match(p.box.querySelector('#wSaid').textContent, /changed/);
  await p.box.querySelector('#w-rotate').onclick(); await tick();
  assert.match(p.box.querySelector('#wSaid').textContent, /nothing to archive/, 'a refusal is shown, not swallowed');
  await p.box.querySelector('#w-reforget').onclick(); await tick();
  assert.equal(sent(p.f, 'POST', '/owner/customers/reforget').length, 1);
  p.f.yes = false;
  await p.box.querySelector('#w-rekey').onclick(); await tick();
  assert.equal(sent(p.f, 'POST', '/owner/customers/rekey').length, 1, 'no yes, no run');
  assert.equal(D.said(null), '');
  assert.equal(D.said('x'), 'x');
});

test('assistant sources: a word shows the facts and their links', async () => {
  const p = page({ 'GET /owner/graph': { nodes: 9, relations: 4, found: [{ kind: 'dish', label: 'Maki', related: [{ direction: 'to', how: 'uses', label: 'Rice' }] }] } });
  const G = await import('./assist-sources.js');
  G.openAssistSources();
  p.box.querySelector('#w-gq').value = '';
  await p.box.querySelector('#wGo').onclick();
  assert.equal(p.f.calls.length, 0, 'no word, no question');
  p.box.querySelector('#w-gq').value = 'maki';
  await p.box.querySelector('#wGo').onclick(); await tick();
  assert.match(sent(p.f, 'GET', '/owner/graph')[0][1], /q=maki/);
  assert.match(p.box.innerHTML, /uses Rice/);
  assert.equal(G.relations({ related: [{ direction: 'from', how: 'in', label: 'Order' }] }), '← in Order');
});

test('tiles: every W-WIRE screen has one, and the messages tile carries the unread count', async () => {
  const p = page({ 'GET /owner/threads': { threads: [], unread: 4 } });
  const T = await import('./wire.js');
  assert.equal(new Set(T.TILES.map(x => x.id)).size, T.TILES.length);
  const h = p.host(); await T.mountTiles(h); await tick();
  assert.equal(h.querySelectorAll('[data-wtile]').length, T.TILES.length);
  assert.match(h.querySelector('[data-wunread]').innerHTML, /4/);
  await T.openTile('graph');
  assert.ok(p.box.querySelector('#w-gq'), 'a tile opens its screen');
  await T.openTile('nope');
});
