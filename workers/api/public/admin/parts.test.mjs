// The console's shared pieces, rendered in node against the real /lib/ui.
// `node --test workers/api/public/admin/`
//
// What these hold: every id and data-* a handler binds passes through; the
// anchor lands as data-tour; words given as a key carry data-t (so the
// console's retranslate() keeps them current); every server string is
// escaped; the ui-adoption gate sees a `ui-` class on every control built here
// except the one checkbox, which is the console's single definition.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as P from './parts.js';
import { useTranslator } from '../lib/ui/core.js';
import { XSS, injected, render } from '../lib/ui/dom-shim.mjs';

useTranslator(k => `«${k}»`);
const one = (html, sel) => render(html).root.querySelector(sel);

test('k: a key reference', () => {
  assert.deepEqual(P.k('save'), { t: 'save' });
});

test('dataOf: data and tour merge into attrs; nothing given is nothing', () => {
  assert.equal(P.dataOf({}), undefined);
  assert.deepEqual(P.dataOf({ data: { o: '1' }, tour: 'orders.accept' }), { data: { o: '1', tour: 'orders.accept' } });
  assert.deepEqual(P.dataOf({ attrs: { name: 'x', data: { a: 1 } }, tour: 't.x' }), { name: 'x', data: { a: 1, tour: 't.x' } });
  assert.deepEqual(P.dataOf({ attrs: { name: 'x' } }), { name: 'x' });
});

test('btn: key words, id, data and anchor pass through; default secondary', () => {
  const h = P.btn({ id: 'go', key: 'save', icon: 'check', data: { o: 'ord1' }, tour: 'orders.accept' });
  const b = one(h, 'button');
  assert.equal(b.id, 'go');
  assert.equal(b.dataset.o, 'ord1');
  assert.equal(b.dataset.tour, 'orders.accept');
  assert.match(b.getAttribute('class'), /ui-btn--secondary/);
  assert.equal(one(h, '[data-t]').dataset.t, 'save');
  assert.match(P.btn({ variant: 'primary', label: 'Hi' }), /ui-btn--primary/);
  assert.match(P.btn({ href: '/x', label: 'Go' }), /^<a /);
  assert.equal(one(P.btn({ icon: 'trash', ariaKey: 'remove' }), 'button').getAttribute('aria-label'), '«remove»');
  assert.deepEqual(injected(P.btn({ label: XSS, data: { o: XSS } })), []);
});

test('iconBtn: named by a key or words; refuses a nameless one', () => {
  const b = one(P.iconBtn({ icon: 'trash', ariaKey: 'remove', data: { del: 'X1' }, tour: 'promos.delete' }), 'button');
  assert.equal(b.getAttribute('aria-label'), '«remove»');
  assert.equal(b.dataset.del, 'X1');
  assert.equal(b.dataset.tour, 'promos.delete');
  assert.match(P.iconBtn({ icon: 'x', ariaLabel: 'close' }), /aria-label="close"/);
  assert.throws(() => P.iconBtn({ icon: 'x' }));
});

test('field: label for the control, key placeholder and hint, anchor on the control', () => {
  const h = P.field({ id: 'pr-code', key: 'promo', phKey: 'findOrder', hintKey: 'minOrder', tour: 'promos.code' });
  const { root } = render(h);
  assert.equal(root.querySelector('label').getAttribute('for'), 'pr-code');
  assert.equal(root.querySelector('label [data-t]').dataset.t, 'promo');
  const i = root.querySelector('#pr-code');
  assert.equal(i.dataset.tour, 'promos.code');
  assert.equal(i.getAttribute('placeholder'), '«findOrder»');
  assert.match(h, /data-t-attr="placeholder:findOrder"/);
  assert.ok(root.querySelector('.ui-hint'));
  assert.match(P.field({ id: 'n', label: 'Name', rows: 3 }), /<textarea/);
  assert.match(P.field({ id: 'n', placeholder: 'x' }), /placeholder="x"/);
  assert.deepEqual(injected(P.field({ id: 'v', label: XSS, value: XSS })), []);
});

test('input: date/time/color with the field classes; other types refused', () => {
  const h = P.input({ id: 'pr-from', type: 'date', key: 'when', value: '2026-09-24', tour: 'promos.from' });
  const i = one(h, 'input');
  assert.equal(i.getAttribute('type'), 'date');
  assert.match(i.getAttribute('class'), /^ui-input/);
  assert.equal(i.dataset.tour, 'promos.from');
  assert.equal(one(h, 'label').getAttribute('for'), 'pr-from');
  assert.match(P.input({ type: 'color', controlCls: 'swatch' }), /class="ui-input swatch"/);
  assert.doesNotMatch(P.input({ id: 'x', type: 'time' }), /<label/);
  assert.throws(() => P.input({ type: 'text' }), /field\(\)/);
  const f = one(P.input({ id: 'bkFile', type: 'file', accept: '.csv', hidden: true, tour: 'bulk.file' }), 'input');
  assert.equal(f.getAttribute('accept'), '.csv');
  assert.equal(f.hasAttribute('hidden'), true);
  assert.equal(f.hasAttribute('multiple'), false);
});

test('select: the chosen option, key options carry data-t, anchor and label', () => {
  const h = P.select({ id: 'role', key: 'role', value: 'waiter', tour: 'staff.role',
    options: [{ value: 'waiter', key: 'waiter' }, { value: 'kitchen', label: XSS }] });
  const { root } = render(h);
  const s = root.querySelector('select');
  assert.match(s.getAttribute('class'), /^ui-input/);
  assert.equal(s.dataset.tour, 'staff.role');
  const opts = root.querySelectorAll('option');
  assert.equal(opts[0].hasAttribute('selected'), true);
  assert.equal(opts[0].dataset.t, 'waiter');
  assert.equal(opts[1].hasAttribute('selected'), false);
  assert.deepEqual(injected(h), []);
  assert.match(P.select({ ariaLabel: 'pick', options: [] }), /aria-label="pick"/);
  assert.doesNotMatch(P.select({ key: 'x', ariaLabel: 'pick', options: [] }), /aria-label/);
  assert.equal(render(P.select()).root.querySelectorAll('option').length, 0);
});

test('check: the one checkbox -- id, state, words, hint, anchor', () => {
  const h = P.check({ id: 'pauseD', checked: true, key: 'paused', hintKey: 'pausedHint', tour: 'state.pause' });
  const i = one(h, 'input');
  assert.equal(i.getAttribute('type'), 'checkbox');
  assert.equal(i.hasAttribute('checked'), true);
  assert.equal(i.dataset.tour, 'state.pause');
  assert.ok(one(h, 'small[data-t="pausedHint"]'));
  assert.equal(one(P.check({ id: 'x' }), 'input').hasAttribute('checked'), false);
  assert.deepEqual(injected(P.check({ id: 'x', label: XSS })), []);
});

test('pill: the three old tones map onto badges; unknown is neutral', () => {
  assert.match(P.pill('ok', { key: 'on' }), /ui-badge--success/);
  assert.match(P.pill('warn', { label: 'x' }), /ui-badge--warning/);
  assert.match(P.pill('bad', { label: 'x' }), /ui-badge--danger/);
  assert.match(P.pill('', { label: 'x' }), /ui-badge--neutral/);
  assert.equal(one(P.pill('ok', { key: 'x', tour: 'a.b' }), '.ui-badge').dataset.tour, 'a.b');
});

test('pillBtn: a tappable pill is a chip button with its data', () => {
  const b = one(P.pillBtn('ok', { key: 'promo_active', data: { flip: 'CODE' }, tour: 'promos.flip' }), 'button');
  assert.match(b.getAttribute('class'), /ui-chip--success/);
  assert.equal(b.dataset.flip, 'CODE');
  assert.match(P.pillBtn('', { label: 'x', ariaKey: 'toggle' }), /aria-label="«toggle»"/);
});

test('empty: key title, body, failure is an alert', () => {
  const h = P.empty('ticket', { key: 'none', bodyKey: 'noneHint' });
  assert.ok(one(h, '.ui-empty-title[data-t="none"]'));
  assert.ok(one(h, '.ui-empty-body[data-t="noneHint"]'));
  assert.equal(one(P.empty('alert-triangle', { title: XSS, alert: true }), '.ui-empty').getAttribute('role'), 'alert');
  assert.deepEqual(injected(P.empty('x', { title: XSS, body: XSS })), []);
});

test('loading: a labelled busy skeleton of rows', () => {
  const s = one(P.loading(3), '.ui-skel-wrap');
  assert.equal(s.getAttribute('aria-busy'), 'true');
  assert.equal(s.getAttribute('aria-label'), '«loading»');
  assert.equal(render(P.loading(3)).root.querySelectorAll('.ui-skel--row').length, 3);
  assert.equal(render(P.loading()).root.querySelectorAll('.ui-skel').length, 1);
  assert.ok(one(P.loading(1, 'card'), '.ui-skel--card'));
});

test('rowBtn / rowDiv: the row classes, markup slots, escaped title, anchor', () => {
  const h = P.rowBtn({ id: 'r1', title: XSS, sub: '<b>sub</b>', leading: 'L', trailing: 'T', data: { open: 'dish' }, tour: 'menu.dish', cls: 'off' });
  const b = one(h, 'button');
  assert.match(b.getAttribute('class'), /^ui-row ui-row--action off$/);
  assert.equal(b.dataset.open, 'dish');
  assert.equal(b.dataset.tour, 'menu.dish');
  assert.ok(one(h, '.ui-row-lead') && one(h, '.ui-row-trail') && one(h, '.ui-row-sub b'));
  assert.deepEqual(injected(P.rowBtn({ title: XSS })), []);
  assert.equal(one(P.rowBtn({ title: { t: 'x' }, ariaLabel: 'open' }), 'button').getAttribute('aria-label'), 'open');
  const d = one(P.rowDiv({ title: 'T', trailing: 'x', leading: 'y', sub: 's', data: { post: 'p1' }, tour: 'posts.row' }), '.ui-row');
  assert.equal(d.tagName, 'DIV');
  assert.equal(d.dataset.post, 'p1');
  assert.doesNotMatch(P.rowDiv({ title: 'T' }), /ui-row-sub|ui-row-lead|ui-row-trail/);
  assert.doesNotMatch(P.rowBtn({ title: 'T' }), /ui-row-sub|ui-row-lead|ui-row-trail/);
});

test('choice: a selectable row, pressed when chosen', () => {
  const b = one(P.choice({ key: 'open', pressed: true, data: { state: 'open' }, tour: 'state.open', subKey: 'openSub' }), 'button');
  assert.equal(b.getAttribute('aria-pressed'), 'true');
  assert.equal(b.dataset.state, 'open');
  assert.equal(b.dataset.tour, 'state.open');
  assert.equal(one(P.choice({ label: 'x' }), 'button').getAttribute('aria-pressed'), 'false');
  assert.match(P.choice({ label: 'x', sub: 'y' }), /ui-row-sub/);
});

test('chips: one pressed, the handler attribute and anchor on each; press() moves it', () => {
  const h = P.chips({ id: 'langPick', values: [{ value: 'sq', label: 'SQ' }, { value: 'en', key: 'en' }], value: 'en', attr: 'l', labelKey: 'language', tour: 'prefs.lang' });
  const { root } = render(h);
  const bs = root.querySelectorAll('button');
  assert.equal(bs[0].getAttribute('aria-pressed'), 'false');
  assert.equal(bs[1].getAttribute('aria-pressed'), 'true');
  assert.equal(bs[0].dataset.l, 'sq');
  assert.equal(bs[0].dataset.tour, 'prefs.lang');
  assert.equal(root.querySelector('.chips').getAttribute('aria-label'), '«language»');
  P.press(bs, bs[0]);
  assert.equal(bs[0].getAttribute('aria-pressed'), 'true');
  assert.equal(bs[1].getAttribute('aria-pressed'), 'false');
  assert.equal(one(P.chips({ values: [{ value: 1, label: '1' }], value: 1 }), 'button').dataset.v, '1');
  assert.doesNotMatch(P.chips({ values: [] }), /id=|aria-label/);
  assert.equal(render(P.chips()).root.querySelectorAll('button').length, 0);
});
