// The Telegram screen, in node: the drawing (telegram-view.js) against the real
// /lib/ui, and the controller (telegram.js) against a fake console seam, so
// every tap is shown to post what the hub reads.
// `node --test workers/api/public/admin/`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
import { readFileSync } from 'node:fs';
import * as V from './telegram-view.js';
import { WORDS } from './telegram-words.js';
import { useTranslator } from '../lib/ui/core.js';
import { Document, Element, render, injected, XSS } from '../lib/ui/dom-shim.mjs';

const HERE = new URL('./', import.meta.url).href;
const FAKES = {
  '/admin/core.js': 'export const { $, t, api, post, toast, sheet, store, clock, day, busy, confirm, retranslate } = globalThis.__tg;',
  '/admin/i18n.js': 'export const T = globalThis.__tgT; export const LANGS = Object.keys(globalThis.__tgT);',
};
register('data:text/javascript,' + encodeURIComponent(`const F = ${JSON.stringify(FAKES)}; const HERE = ${JSON.stringify(HERE)};
  export async function resolve(spec, ctx, next){
    if (F[spec]) return { url: 'data:text/javascript,' + encodeURIComponent(F[spec]), shortCircuit: true };
    if (spec === '/admin/telegram-view.js' || spec === '/admin/telegram-words.js') return { url: HERE + spec.slice(7), shortCircuit: true };
    return next(spec, ctx);
  }`));

useTranslator(k => `<${k}>`);
/// The shim's [attr="v"] takes word characters only; a cell id has a dot and a slash.
const cellOf = (root, id) => root.querySelectorAll('[data-cell]').find(e => e.getAttribute('data-cell') === id);
Object.defineProperty(Element.prototype, 'value', {
  get(){ return this.getAttribute('value') ?? ''; }, set(v){ this.setAttribute('value', v); }, configurable: true });

const EVENTS = [
  { key: 'order.placed', area: 'orders', live: true, scheduled: false, urgent: true },
  { key: 'stock.low', area: 'stock', live: false, scheduled: false, urgent: false },
  { key: 'digest.daily', area: 'analytics', live: true, scheduled: true, urgent: false },
];
const G = (id, over = {}) => ({ id, chat: '-1', title: 'T ' + id, kind: 'supergroup', lang: 'sq', pii: 'none', state: 'active', digest_at: 540,
  subs: { 'order.placed': 'now' }, health: {}, waiting: 0, ...over });
const DATA = (over = {}) => ({ tokenSet: true, bot: { username: 'dubinbot', hook_ms: 1 }, legacy: false, pending: null,
  groups: [G('kitchen', { thread: 42 }), G('owners', { lang: 'uk', pii: 'full', state: 'muted', subs: { 'digest.daily': 'now' } })],
  events: EVENTS, langs: ['en', 'sq', 'uk', 'ru'], ...over });

test('nextMode cycles now, summary, off; a summary row is now or off', () => {
  assert.deepEqual(['now', 'digest', 'off'].map(m => V.nextMode(m, false)), ['digest', 'off', 'now']);
  assert.deepEqual(['now', 'off'].map(m => V.nextMode(m, true)), ['off', 'now']);
  assert.equal(V.nextMode(undefined, false), 'now');
});

test('times: minutes and HH:MM round trip; nonsense is null', () => {
  assert.equal(V.hhmm(0), '00:00');
  assert.equal(V.hhmm(1380), '23:00');
  assert.equal(V.minutes('07:30'), 450);
  for (const bad of ['24:00', '7', '', null, 'ab:cd']) assert.equal(V.minutes(bad), null);
});

test('the matrix rows are the hub events, grouped by area, and every group is a column', () => {
  const { root } = render(V.matrix(DATA()));
  assert.equal(root.querySelectorAll('th[scope="col"]').length, 2);
  assert.deepEqual(root.querySelectorAll('.tg-area th').map(th => th.getAttribute('data-t')), ['tg_area_orders', 'tg_area_stock', 'tg_area_analytics']);
  const cells = root.querySelectorAll('[data-cell]');
  assert.equal(cells.length, EVENTS.length * 2);
  const c = cellOf(root, 'kitchen/order.placed');
  assert.equal(c.getAttribute('data-mode'), 'now');
  assert.equal(c.getAttribute('aria-pressed'), 'true');
  assert.ok(c.className.includes('ui-chip'), 'a design-system control');
  assert.equal(cellOf(root, 'kitchen/stock.low').hasAttribute('disabled'), true, 'a row with no producer yet is not a switch');
  assert.equal(cellOf(root, 'owners/digest.daily').getAttribute('data-scheduled'), '1');
  assert.equal(V.matrix(DATA({ groups: [] })), '', 'no group, no matrix');
});

test('a group card shows every choice at once, in the hub languages (four, not three)', () => {
  const d = DATA();
  const { root } = render(V.groupCard(d.groups[0], d, ms => 'at ' + ms));
  assert.deepEqual(root.querySelectorAll('[data-glang]').map(b => b.getAttribute('data-glang')), ['en', 'sq', 'uk', 'ru']);
  assert.equal(root.querySelector('[data-glang="sq"]').getAttribute('aria-pressed'), 'true');
  assert.equal(root.querySelector('[data-gpii="none"]').getAttribute('aria-pressed'), 'true');
  assert.ok(root.querySelector('[data-gtest="kitchen"]') && root.querySelector('[data-gmute="kitchen"]') && root.querySelector('[data-gunlink="kitchen"]'));
  assert.equal(root.querySelector('[data-gdig="kitchen"]').getAttribute('value'), '09:00');
  assert.ok(root.textContent.includes('tg_topic') || root.innerHTML.includes('42'), 'the topic is shown');
  assert.equal(root.querySelector('[data-gtest="kitchen"]').getAttribute('data-tour'), 'notify.test');
});

test('health says the last error, or the last send, or that nothing went yet', () => {
  const d = DATA();
  const bad = V.groupCard(G('x', { health: { failing: true, err_ms: 5, err: 'Forbidden: bot was kicked' } }), d, ms => 'at ' + ms);
  assert.ok(bad.includes('Forbidden: bot was kicked') && bad.includes('at 5'));
  assert.ok(V.groupCard(G('x', { health: { ok_ms: 7 } }), d, ms => 'at ' + ms).includes('at 7'));
  assert.ok(V.groupCard(G('x'), d).includes('tg_never'));
  assert.ok(V.groupCard(G('x', { state: 'left' }), d).includes('tg_state_left'));
});

test('everything the hub or Telegram wrote is escaped', () => {
  const d = DATA({ groups: [G('x', { title: XSS, kind: XSS, health: { failing: true, err_ms: 1, err: XSS } })] });
  assert.deepEqual(injected(V.page(d, { code: XSS, exp_ms: 10_000, command: XSS, deepLink: 'https://t.me/x' }, 0, () => XSS)), []);
});

test('the wizard shows the command, the link and the time left, then says the code expired', () => {
  const p = { code: 'ABCD89A9', exp_ms: 125_000, command: '/link@dubinbot ABCD89A9', deepLink: 'https://t.me/dubinbot?startgroup=ABCD89A9' };
  const { root } = render(V.wizard(p, 125_000));
  assert.equal(root.querySelector('#tgCmd').textContent, '/link@dubinbot ABCD89A9');
  assert.equal(root.querySelector('#tgLeft').textContent, '2:05');
  assert.ok(root.innerHTML.includes('startgroup=ABCD89A9'));
  assert.ok(V.wizard(p, 0).includes('tg_expired'));
  assert.equal(V.wizard(null, 5), '');
  assert.ok(V.wizard({ code: 'X' }, 5).includes('/link X'), 'a code minted elsewhere still says what to send');
});

test('the page: no bot means no linking yet; no group means the empty state; legacy is explained', () => {
  const none = render(V.page(DATA({ bot: null, groups: [] }), null, 0)).root;
  assert.equal(none.querySelector('#tgAdd').hasAttribute('disabled'), true);
  assert.ok(none.innerHTML.includes('tg_noGroups'));
  assert.ok(render(V.page(DATA({ legacy: true }), null, 0)).root.innerHTML.includes('tg_legacy'));
  const anchors = render(V.page(DATA(), null, 0)).root.querySelectorAll('[data-tour]').map(e => e.getAttribute('data-tour')).sort();
  assert.deepEqual(anchors, ['notify.telegramState', 'notify.test', 'notify.tgChat', 'notify.tgToken'], 'the lesson anchors live on');
});

test('the words: every English key in every language, ASCII quotes only', () => {
  for (const l of Object.keys(WORDS)) assert.deepEqual(Object.keys(WORDS[l]).sort(), Object.keys(WORDS.en).sort(), l);
  for (const e of ['order.placed', 'stock.low', 'digest.daily', 'system.alert', 'stocktake.variance']) assert.ok(WORDS.en['tgev_' + e], e);
  const src = readFileSync(new URL('./telegram-words.js', import.meta.url), 'utf8') + readFileSync(new URL('./telegram.js', import.meta.url), 'utf8');
  assert.ok(!/[‘’“”]/.test(src), 'no typographic quote');
});

// ── the controller, against a fake console ─────────────────────────────────

function fakeConsole(d){
  const doc = new Document();
  doc.head = doc.createElement('head');
  doc.body.appendChild(doc.head);
  const sheetIn = doc.createElement('div');
  sheetIn.setAttribute('id', 'sheetIn');
  doc.body.appendChild(sheetIn);
  globalThis.document = doc;
  const calls = [], toasts = [];
  const answers = { group: p => ({ ok: true, group: { ...d.groups.find(g => g.id === p.id), ...(p.patch.subs ? { subs: { ...d.groups.find(g => g.id === p.id).subs, ...p.patch.subs } } : {}), ...(p.patch.lang ? { lang: p.patch.lang } : {}) } }),
    test: () => ({ ok: false, error: 'Forbidden: bot was kicked' }), unlink: () => ({ ok: true }), connect: () => ({ ok: true, bot: 'dubinbot' }),
    link: () => ({ code: 'ABCD89A9', exp_ms: Date.now() + 600_000, command: '/link@dubinbot ABCD89A9', deepLink: 'https://t.me/dubinbot?startgroup=ABCD89A9' }) };
  globalThis.__tgT = { en: { on: 'on' }, sq: {}, uk: {}, ru: {} };
  globalThis.__tg = {
    $: (s, r = doc) => r.querySelector(s), t: k => k, store: { loc: 'dubin' }, clock: String, day: String, retranslate(){},
    toast: m => toasts.push(m), busy: (_, f) => f(), confirm: async () => ({ ok: true }),
    sheet: html => { sheetIn.innerHTML = html; },
    api: async path => { calls.push(['GET', path]); return structuredClone(d); },
    post: async (path, body) => { calls.push(['POST', path, body]); return answers[path.split('?')[0].split('/').pop()](body); },
  };
  return { doc, sheetIn, calls, toasts };
}

test('the controller: open, tap a cell, choose a language, test, pause, unlink, link, connect', async () => {
  const d = DATA();
  const f = fakeConsole(d);
  const C = await import('./telegram.js?c=' + Date.now());
  await C.open();
  assert.equal(f.calls[0][1], '/owner/telegram?location_id=dubin', 'the venue rides in the query');
  assert.ok(f.doc.getElementById('tgCss'), 'its stylesheet is loaded once');
  assert.equal(globalThis.__tgT.ru.tg_title, 'Telegram', 'a language with no row reads English');
  const click = async el => { await f.sheetIn.onclick({ target: el }); await new Promise(r => setTimeout(r, 0)); };

  await click(cellOf(f.sheetIn, 'kitchen/order.placed'));
  assert.deepEqual(f.calls.at(-1), ['POST', '/owner/telegram/group?location_id=dubin', { id: 'kitchen', patch: { subs: { 'order.placed': 'digest' } } }]);
  assert.equal(cellOf(f.sheetIn, 'kitchen/order.placed').getAttribute('data-mode'), 'digest', 'redrawn with the answer');

  await click(f.sheetIn.querySelector('[data-group="kitchen"] [data-glang="en"]'));
  assert.deepEqual(f.calls.at(-1)[2], { id: 'kitchen', patch: { lang: 'en' } });
  await click(f.sheetIn.querySelector('[data-group="kitchen"] [data-gpii="fulfil"]'));
  assert.deepEqual(f.calls.at(-1)[2], { id: 'kitchen', patch: { pii: 'fulfil' } });
  await click(f.sheetIn.querySelector('[data-gquiet="kitchen"]'));
  assert.deepEqual(f.calls.at(-1)[2], { id: 'kitchen', patch: { quiet: { from: 1380, to: 420 } } });

  await click(f.sheetIn.querySelector('[data-gtest="kitchen"]'));
  assert.equal(f.toasts.at(-1), 'Forbidden: bot was kicked', 'Telegram words, as the answer');
  await click(f.sheetIn.querySelector('[data-gmute="owners"]'));
  assert.deepEqual(f.calls.at(-1)[2], { id: 'owners', patch: { muted: false } }, 'a paused group resumes');
  await click(f.sheetIn.querySelector('[data-gunlink="owners"]'));
  assert.deepEqual(f.calls.at(-2), ['POST', '/owner/telegram/unlink?location_id=dubin', { id: 'owners' }]);

  const dig = f.sheetIn.querySelector('[data-gdig="kitchen"]');
  dig.value = '10:30';
  await f.sheetIn.onchange({ target: dig });
  assert.deepEqual(f.calls.at(-1)[2], { id: 'kitchen', patch: { digest_at: 630 } });

  await f.doc.getElementById('tgAdd').onclick();
  assert.equal(f.calls.at(-1)[1], '/owner/telegram/link?location_id=dubin');
  assert.equal(f.doc.getElementById('tgCmd').textContent, '/link@dubinbot ABCD89A9');

  f.doc.getElementById('tgToken').value = ' 123:abc ';
  await f.doc.getElementById('tgConnect').onclick();
  assert.deepEqual(f.calls.find(c => c[1].startsWith('/owner/telegram/connect')), ['POST', '/owner/telegram/connect?location_id=dubin', { token: '123:abc' }]);
  assert.ok(f.toasts.some(m => m.startsWith('@dubinbot')));
});
