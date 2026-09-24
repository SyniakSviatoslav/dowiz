// The courier's screens, rendered in node against the real /lib/ui components.
// `node --test workers/api/public/courier/`
//
// What these hold: every id app.js binds and the e2e journeys drive is present
// on the screen that needs it; every server-supplied string (an address, a
// phone number, a note, an error) is escaped; money goes through the formatter
// it is handed and wears `.money`; ONE main action per screen.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as screens from './screens.js';
import { useTranslator } from '../lib/ui/core.js';
import { XSS, injected, render } from '../lib/ui/dom-shim.mjs';

const t = (k, v) => (v ? `${k}:${JSON.stringify(v)}` : `«${k}»`);
useTranslator(t);
const money = n => `${n} L`;
const ctx = { t, money, lang: 'en', langs: ['sq', 'en', 'uk'] };
const ids = html => render(html).root.querySelectorAll('[id]').map(e => e.id);
const has = (html, ...want) => { const got = ids(html); for (const w of want) assert.ok(got.includes(w), `#${w} missing`); };
const primaries = html => render(html).root.querySelectorAll('.ui-btn--primary,.ui-btn--success,.ui-btn--danger').length;

const order = (o = {}) => ({ id: 'ord_0123456789', total: 1500, payment: 'cash', items: 3, status: 'READY',
  address: { line: 'Rruga Tregtare 5', note: 'kati 2', lat_udeg: 41315347, lon_udeg: 19444996 }, contact: { phone: '+355691234567' }, ...o });
const evil = () => order({ id: XSS, items: XSS, address: { line: XSS, note: XSS }, contact: { phone: XSS } });

test('login: the fields and buttons the journeys drive, language switch, one primary', () => {
  const h = screens.login(null, ctx);
  has(h, 'loginForm', 'em', 'pw', 'go', 'toClaim', 'langSeg');
  const { root } = render(h);
  assert.equal(root.querySelector('#go').getAttribute('type'), 'submit');
  assert.equal(root.querySelector('label[for="em"]').textContent, '«emailOrPhone»');
  assert.equal(root.querySelector('[role="radio"][aria-checked="true"]').dataset.value, 'en');
  assert.equal(primaries(h), 1);
  assert.equal(root.querySelector('[role="alert"]'), null);
  // a failed sign-in is announced, and the server's words are escaped
  assert.deepEqual(injected(screens.login(XSS, ctx)), []);
  assert.ok(render(screens.login('bad password', ctx)).root.querySelector('[role="alert"]'));
});

test('claim: code, phone, password with its hint wired to the field', () => {
  const h = screens.claim(null);
  has(h, 'claimForm', 'cph', 'cod', 'cpw', 'cgo', 'toLogin');
  const { root } = render(h);
  assert.equal(root.querySelector('#cod').getAttribute('maxlength'), '16');
  assert.equal(root.querySelector('#cpw').getAttribute('aria-describedby'), 'cpw-hint');
  assert.deepEqual(injected(screens.claim(XSS)), []);
});

test('first paint: a labelled skeleton, never a claim about the shift', () => {
  const { root } = render(screens.loading(ctx));
  assert.equal(root.children[0].getAttribute('aria-busy'), 'true');
  assert.equal(root.children[0].getAttribute('aria-label'), '«loading»');
  assert.equal(root.textContent.trim(), '');
});

test('failed first load: announced, with the reason escaped, and a retry', () => {
  const h = screens.failed(XSS);
  has(h, 'retry');
  assert.ok(render(h).root.querySelector('[role="alert"] .ti'));
  assert.deepEqual(injected(h), []);
});

test('off shift and waiting', () => {
  has(screens.offShift(), 'openShift');
  assert.equal(primaries(screens.offShift()), 1);
  const w = screens.waiting();
  has(w, 'askBox', 'askGo', 'answer', 'earn', 'hist', 'endShift');
  const { root } = render(w);
  assert.equal(root.querySelector('#askGo').getAttribute('aria-label'), '«send»');
  assert.ok(root.querySelector('#answer').hidden);
  // the empty state keeps an icon inside it (e2e/_probe_courier4 measures one)
  assert.ok(root.querySelector('.ui-empty .ti'));
});

test('pick list: rows select, ONE button takes, a queued tap disables it', () => {
  const avail = [order({ id: 'aaaaaaaa1' }), order({ id: 'bbbbbbbb2', total: 900 })];
  const h = screens.pickList(avail, 'bbbbbbbb2', () => null, ctx);
  has(h, 'take', 'endShift');
  const { root } = render(h);
  const rows = root.querySelectorAll('[data-sel]');
  assert.deepEqual(rows.map(r => r.getAttribute('aria-pressed')), ['false', 'true']);
  assert.equal(root.querySelector('#take').textContent, '«take» #bbbbbbbb');
  assert.equal(root.querySelector('#take').disabled, false);
  assert.equal(root.querySelectorAll('.money').map(m => m.textContent).join('|'), '1500 L|900 L');
  assert.equal(primaries(h), 1);
  // an unknown selection falls back to the first row rather than none
  assert.equal(render(screens.pickList(avail, 'gone', () => null, ctx)).root.querySelector('[aria-pressed="true"]').dataset.sel, 'aaaaaaaa1');
  const q = render(screens.pickList(avail, 'aaaaaaaa1', id => id === 'aaaaaaaa1' ? { tag: 'accept:x' } : null, ctx)).root.querySelector('#take');
  assert.ok(q.disabled);
  assert.match(q.textContent, /«queued»/);
  assert.deepEqual(injected(screens.pickList([evil()], XSS, () => null, ctx)), []);
});

test('active run: pick before pickup; the swipe after; a held tap replaces both', () => {
  const ready = screens.active(order(), { waiting: null, eta: '≈ 6 min' }, ctx);
  has(ready, 'pick', 'etaLine');
  assert.equal(ids(ready).includes('done'), false);
  assert.equal(ids(ready).includes('refused'), false);
  const going = screens.active(order({ status: 'IN_DELIVERY' }), { waiting: null, eta: '' }, ctx);
  has(going, 'slide', 'slideFill', 'done', 'refused');
  const { root } = render(going);
  assert.equal(root.querySelector('#done').getAttribute('aria-label'), '«deliveredAria»');
  assert.equal(root.querySelector('[data-status]').getAttribute('data-status'), 'IN_DELIVERY');
  assert.ok(root.querySelector('.cash .money'));
  assert.equal(root.querySelector('a[href^="tel:"]').getAttribute('href'), 'tel:+355691234567');
  assert.equal(root.querySelector('a[target="_blank"]').getAttribute('rel'), 'noopener');
  const held = screens.active(order({ status: 'IN_DELIVERY' }), { waiting: { tag: 'deliver:x' }, eta: '' }, ctx);
  for (const id of ['pick', 'done', 'refused']) assert.equal(ids(held).includes(id), false, id);
  assert.ok(render(held).root.querySelector('[role="status"]'));
  // paid online: no cash chip, a badge instead
  assert.equal(render(screens.active(order({ payment: 'card' }), { eta: '' }, ctx)).root.querySelector('.cash'), null);
  assert.deepEqual(injected(screens.active(evil(), { eta: XSS }, ctx)), []);
});

test('cash handover and refused-at-door', () => {
  const c = screens.cash(order(), '', ctx);
  has(c, 'got', 'confirm', 'back');
  const got = render(c).root.querySelector('#got');
  assert.equal(got.getAttribute('value'), '1500');
  assert.equal(got.getAttribute('inputmode'), 'numeric');
  const r = screens.refused(order(), '', ctx);
  has(r, 'rnote', 'rgo', 'back');
  assert.equal(render(r).root.querySelector('#rnote').getAttribute('maxlength'), String(screens.NOTE_MAX));
  assert.ok(render(r).root.querySelector('#rgo.ui-btn--danger'));
  assert.deepEqual(injected(screens.cash(evil(), XSS, ctx) + screens.refused(evil(), XSS, ctx)), []);
});

test('offer: time left, or lapsed and still takeable', () => {
  const h = screens.offer(order(), 125, ctx);
  has(h, 'takeOffer', 'endShift', 'offerClock', 'offerLeft');
  assert.equal(render(h).root.querySelector('#offerClock').textContent, '2:05');
  const lapsed = screens.offer(order(), 0, ctx);
  assert.equal(ids(lapsed).includes('offerClock'), false);
  has(lapsed, 'takeOffer');
  assert.deepEqual(injected(screens.offer(evil(), 3, ctx)), []);
});

test('panels: back button, earnings keep tips apart from the float, history empty state', () => {
  const p = screens.panel('History', screens.panelLoading(ctx));
  has(p, 'pback');
  const d = { cashInHand: 4500, expectedCash: 1200,
    today: { deliveries: 3, cash: 4500, tips: 200 }, week: { deliveries: 9, cash: 9000, tips: 0 }, month: { deliveries: 30, cash: 30000 } };
  const { root } = render(screens.earnings(d, ctx));
  assert.equal(root.querySelector('.ui-stat--hero .money').textContent, '4500 L');
  const tips = root.querySelectorAll('.ui-amount--success');
  assert.equal(tips.length, 1);
  assert.equal(tips[0].textContent, '+200 L');
  assert.equal(render(screens.history([], () => '', ctx)).root.querySelector('.ui-empty-title').textContent, '«emptyHistory»');
  const hist = render(screens.history([{ street: 'Rr. A', at: 0, status: 'DELIVERED', cashCollected: 700 }], () => '24 Sep', ctx)).root;
  assert.equal(hist.querySelector('.ui-row-sub').textContent, '24 Sep · DELIVERED');
  assert.deepEqual(injected(screens.history([{ street: XSS, status: XSS }], () => XSS, ctx) + screens.panelError(XSS) + screens.panel(XSS, '')), []);
});
