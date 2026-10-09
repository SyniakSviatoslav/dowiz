// The queue's lists in node (W-LOOPC, R-LOOPS row 7): history and search
// give the SAME orders, in the SAME order, as the code they replaced.
// `node --test workers/api/public/admin/orders-list.test.mjs`
//
// THE ORACLE is the old `matching()` of admin/orders.js copied verbatim
// (base 6ccca6ef, lines 92-98): `liveOrders()` rebuilt inside the history
// filter + a linear `includes`, and the hay string normalised per order per
// keystroke. The new code keeps one Set per call and a hay per order object.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';

// THE BROWSER SEAM (as kitchen-dom.test.mjs): core, i18n and app are fakes
// reading globalThis.__ol; every other /admin/ or /lib/ path is the real file.
const PUBLIC = new URL('../', import.meta.url).href;
const FAKES = {
  '/admin/core.js': `const F = () => globalThis.__ol;
    import * as ui from '/lib/ui/index.js';
    export const esc = ui.esc, ORDER_ID_SHOWN = 8;
    export const $ = () => null, $$ = () => [];
    export const icon = n => '<i class="ti ti-' + n + '"></i>';
    export const t = k => k;
    export const S = new Proxy({}, { get: (_, k) => F().S[k] });
    export const api = async () => ({}), post = async () => ({}), withLoc = (b = {}) => b;
    export const toast = () => {}, sheet = () => {}, closeSheet = () => {}, busy = (el, fn) => fn();
    export const confirm = async () => ({ ok: true });
    export const money = n => String(n), moneyEl = n => String(n), ago = () => '', clock = () => '', day = () => '';`,
  '/admin/i18n.js': `const F = () => globalThis.__ol;
    export const T = { sq: {}, en: {}, uk: {}, ru: {} }, LANGS = ['sq', 'en', 'uk', 'ru'];
    export const st = s => F().words[s] || s;
    export const payName = p => p || '', intlLocale = () => 'en';`,
  '/admin/app.js': `const F = () => globalThis.__ol;
    const LIVE = ['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY'];
    export const liveOrders = () => F().S.orders.filter(o => LIVE.includes(o.status));
    export const loadOrders = async () => {}, rerender = async () => {};`,
};
register('data:text/javascript,' + encodeURIComponent(`const F = ${JSON.stringify(FAKES)}, P = ${JSON.stringify(PUBLIC)};
  export async function resolve(spec, ctx, next){
    if (F[spec]) return { url: 'data:text/javascript,' + encodeURIComponent(F[spec]), shortCircuit: true };
    if (/^\\/(admin|lib|store)\\//.test(spec)) return { url: P + spec.slice(1), shortCircuit: true };
    return next(spec, ctx);
  }`));

const EN = { PENDING: 'New', CONFIRMED: 'Accepted', PREPARING: 'Cooking', READY: 'Ready', IN_DELIVERY: 'On the way', DELIVERED: 'Delivered', CANCELLED: 'Cancelled', REJECTED: 'Rejected' };
const UK = { PENDING: 'Нове', CONFIRMED: 'Прийнято', PREPARING: 'Готується', READY: 'Готово', IN_DELIVERY: 'В дорозі', DELIVERED: 'Доставлено', CANCELLED: 'Скасовано', REJECTED: 'Відхилено' };
globalThis.__ol = { S: { orders: [] }, words: EN };
const O = await import('./orders.js');

// ── the oracle: base orders.js:90-99, verbatim but for `view` -> arguments ──
const ORDER_ID_SHOWN = 8;
const st = s => globalThis.__ol.words[s] || s;
const LIVE = ['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY'];
const liveOrders = () => globalThis.__ol.S.orders.filter(o => LIVE.includes(o.status));
const norm = s => String(s ?? '').toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '');
function oracleMatching(view){
  const S = globalThis.__ol.S;
  const all = view.mode === 'history' ? S.orders.filter(o => !liveOrders().includes(o)) : liveOrders();
  const q = norm(view.q).trim(); if (!q) return all;
  const terms = q.split(/\s+/).filter(Boolean);
  return all.filter(o => { const hay = norm([o.id.slice(0, ORDER_ID_SHOWN), o.contact?.name, o.contact?.phone, o.fulfilment?.address?.line, o.status, st(o.status), o.promo?.code, ...(o.items || []).map(i => i.name)].join(' ')); return terms.every(tm => hay.includes(tm)); });
}
const oracleHistoryAll = () => globalThis.__ol.S.orders.filter(o => !liveOrders().includes(o));

/// A deterministic venue: statuses, diacritics, missing fields, promos.
const STATUSES = ['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED', 'CANCELLED', 'REJECTED', 'DELIVERED', 'DELIVERED'];
const NAMES = ['Ardit Hoxha', 'Ëndrit Çela', 'Олена Коваль', 'Zoë Brontë', null, 'Guest'];
const DISHES = ['Salmon roll', 'Tuna nigiri', 'Gyoza', 'Miso soup', 'Edamame'];
function venue(n, seed = 7){
  let x = seed; const rnd = m => (x = (x * 1103515245 + 12345) & 0x7fffffff) % m;
  return Array.from({ length: n }, (_, i) => {
    const name = NAMES[rnd(NAMES.length)];
    return {
      id: `ord_${String(i).padStart(6, '0')}`, status: STATUSES[rnd(STATUSES.length)], total: 1500 + rnd(20) * 100,
      ...(name || rnd(2) ? { contact: { ...(name ? { name } : {}), phone: `+35569${String(1000000 + i).slice(1)}` } } : {}),
      ...(rnd(3) ? { fulfilment: { kind: 'delivery', address: { line: `Rruga ${rnd(40)} Durrës` } } } : { fulfilment: { kind: 'pickup' } }),
      ...(rnd(5) === 0 ? { promo: { code: `SUMMER${rnd(9)}` } } : {}),
      ...(rnd(7) ? { items: Array.from({ length: 1 + rnd(3) }, () => ({ name: DISHES[rnd(DISHES.length)], quantity: 1 })) } : {}),
    };
  });
}
const QUERIES = ['', '   ', 'salmon', 'SALMON ROLL', 'endrit', 'Ëndrit', 'çela', 'zoe', 'олена', 'durres 1', 'ready', 'new',
  'cooking', 'on the way', 'delivered', 'summer3', '+35569', 'ord_0001', 'ord_00012', 'gyoza miso', 'nothing-matches', 'готово'];
const same = (a, b, why) => {
  assert.equal(a.length, b.length, `${why}: length ${a.length} vs oracle ${b.length}`);
  for (let i = 0; i < a.length; i++) assert.ok(a[i] === b[i], `${why}: row ${i} is ${a[i]?.id}, oracle ${b[i]?.id}`);
};
const sweep = why => {
  for (const mode of ['live', 'history']) for (const q of QUERIES) same(O.listFor(mode, q), oracleMatching({ mode, q }), `${why} ${mode} "${q}"`);
  same(O.historyOf(globalThis.__ol.S.orders, liveOrders()), oracleHistoryAll(), `${why} historyAll`);
};

test('300 orders: every mode x query gives the oracle\'s orders, same objects, same order', () => {
  globalThis.__ol.S.orders = venue(300); globalThis.__ol.words = EN;
  sweep('n=300');
  // A non-trivial sweep: both lists non-empty, and some queries narrow them.
  assert.ok(O.listFor('live', '').length > 50 && O.listFor('history', '').length > 50);
  assert.ok(O.listFor('history', 'salmon').length > 0 && O.listFor('history', 'salmon').length < O.listFor('history', '').length);
  // Twice in a row: the second pass reads the cached hays and still agrees.
  sweep('n=300 cached');
});

test('the words follow the language: a cached hay is rebuilt when the status word changes', () => {
  globalThis.__ol.S.orders = venue(120, 11); globalThis.__ol.words = EN;
  sweep('en');
  assert.ok(O.listFor('live', 'cooking').length > 0);
  globalThis.__ol.words = UK;
  sweep('uk');
  assert.equal(O.listFor('live', 'cooking').length, 0, 'the English word is gone after the switch');
  assert.ok(O.listFor('live', 'готується').length > 0, 'the Ukrainian word is found');
  globalThis.__ol.words = EN;
  sweep('en again');
});

test('an order that moves is a new object: its new name, status and dishes are found, the old ones are not', () => {
  globalThis.__ol.words = EN;
  const orders = venue(40, 3);
  orders[5] = { ...orders[5], status: 'PENDING', contact: { name: 'Besnik Krasniqi', phone: '+355690000005' }, items: [{ name: 'Gyoza', quantity: 1 }] };
  globalThis.__ol.S.orders = orders;
  sweep('before');
  const moved = { ...orders[5], status: 'DELIVERED', contact: { name: 'Teuta Mema', phone: '+355690000005' }, items: [{ name: 'Edamame', quantity: 2 }] };
  globalThis.__ol.S.orders = orders.map((o, i) => i === 5 ? moved : o);
  sweep('after');
  assert.deepEqual(O.listFor('history', 'teuta').map(o => o.id), [moved.id]);
  assert.equal(O.listFor('live', 'besnik').length, 0);
  // Even a status written IN PLACE (nothing does this today) is not served stale.
  moved.status = 'READY';
  sweep('in place');
  assert.deepEqual(O.listFor('live', 'ready teuta').map(o => o.id), [moved.id]);
});

test('history is by identity, as before: two objects with one id are told apart', () => {
  globalThis.__ol.words = EN;
  const a = { id: 'ord_twin', status: 'READY' }, b = { id: 'ord_twin', status: 'DELIVERED' };
  globalThis.__ol.S.orders = [a, b, { id: 'ord_x', status: 'CANCELLED' }];
  sweep('twins');
  same(O.listFor('history', ''), [b, globalThis.__ol.S.orders[2]], 'twins history');
  globalThis.__ol.S.orders = [];
  sweep('empty');
});
