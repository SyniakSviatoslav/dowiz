// The storefront's pieces on /lib/ui, rendered in node against the real
// components, and two checks across every storefront file: each icon it names
// is drawn by /lib/icons.css, and each learning anchor is listed in
// docs/learn/anchors-store.txt (and each listed one still exists).
// `node --test workers/api/public/store/*.test.mjs`
//
// The screens themselves import '/store/...' by absolute URL and touch the DOM
// when they load, so node cannot import them; what they draw with is here.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { cta, ghost, seg, stepper, emptySheet, rows, choiceList, k } from './parts.js';
import { useTranslator } from '../lib/ui/core.js';
import { XSS, injected, render } from '../lib/ui/dom-shim.mjs';

const t = key => `«${key}»`;
useTranslator(t);
const $ = (h, s) => render(h).root.querySelector(s);
const el = h => render(h).root.children[0];

test('cta: the component in the venue\'s foil, one per sheet, with its anchor', () => {
  const b = el(cta({ id: 'place', label: k('place'), iconEnd: 'chevron-right', tour: 'checkout.place', cls: 'mb-2' }));
  assert.equal(b.tagName, 'BUTTON');
  for (const c of ['ui-btn', 'ui-btn--primary', 'ui-btn--lg', 'ui-btn--block', 'btn', 'mb-2']) assert.ok(b.classList.contains(c), c);
  assert.equal(b.getAttribute('data-tour'), 'checkout.place');
  assert.equal(b.getAttribute('type'), 'button');
  assert.equal($(cta({ label: k('place') }), '[data-t="place"]').textContent, '«place»', 'the label carries the i18n hook');
  assert.ok(el(cta({ id: 'bkGo', label: 'x', disabled: true })).disabled);
  const link = el(ghost({ href: 'tel:+355', label: k('callUs') }));
  assert.equal(link.tagName, 'A');
  assert.ok(link.classList.contains('btn-ghost') && link.classList.contains('ui-btn--secondary'));
});

test('ghost: glass secondary; hidden until the binder shows it; attrs pass through', () => {
  const b = el(ghost({ id: 'dar', label: k('onTable'), attrs: { hidden: true, data: { share: 'b1' } }, tour: 'dish.onTable' }));
  assert.ok(b.hidden);
  assert.equal(b.getAttribute('data-share'), 'b1');
  assert.equal(b.getAttribute('data-tour'), 'dish.onTable', 'the anchor merges with the other data');
  assert.ok(!el(ghost({ label: 'x', block: false })).classList.contains('ui-btn--block'));
});

test('seg: a pressed chip in the segment skin, the chosen one "on"', () => {
  const on = el(seg({ on: true, icon: 'bike', label: k('toDoor'), attrs: { data: { how: 'delivery' } }, tour: 'checkout.delivery' }));
  const off = el(seg({ on: false, label: 'x', attrs: { data: { how: 'pickup' } } }));
  assert.equal(on.getAttribute('aria-pressed'), 'true');
  assert.equal(off.getAttribute('aria-pressed'), 'false');
  assert.ok(on.classList.contains('seg-b') && on.classList.contains('on') && !off.classList.contains('on'));
  assert.equal(on.getAttribute('data-how'), 'delivery');
});

test('stepper: minus, the count, plus -- ids and data the binders use', () => {
  const h = stepper({ value: 1, valueId: 'dq', minus: { id: 'dm' }, plus: { id: 'dp' }, tour: 'dish.qty' });
  const { root } = render(h);
  assert.equal(root.querySelector('.qty').getAttribute('data-tour'), 'dish.qty');
  assert.equal(root.querySelector('#dq').textContent, '1');
  assert.equal(root.querySelector('#dm').getAttribute('aria-label'), '−');
  assert.equal(root.querySelector('#dp').getAttribute('aria-label'), '+');
  const line = render(stepper({ value: 3, minus: { attrs: { data: { m: 'k1' } } }, plus: { attrs: { data: { a: 'k1' } } } })).root;
  assert.ok(line.querySelector('[data-m="k1"]') && line.querySelector('[data-a="k1"]'));
  assert.deepEqual(injected(stepper({ value: XSS, valueId: 'q' })), []);
});

test('empty and loading: an empty sheet says what and why; a skeleton is labelled', () => {
  const { root } = render(emptySheet({ icon: 'shopping-bag', title: 'empty', body: 'emptyHint' }));
  assert.equal(root.querySelector('.ui-empty-title').getAttribute('data-t'), 'empty');
  assert.equal(root.querySelector('.ui-empty-body').getAttribute('data-t'), 'emptyHint');
  assert.equal(render(emptySheet({ icon: 'receipt', title: 'noOrders' })).root.querySelector('.ui-empty-body'), null);
  const sk = render(rows(3, 'Loading')).root.children[0];
  assert.equal(sk.getAttribute('aria-busy'), 'true');
  assert.equal(sk.getAttribute('aria-label'), 'Loading');
  assert.equal(sk.children.length, 3);
});

test('choiceList: pick-one rows, the chosen pressed and lit, the binder\'s key kept, names escaped', () => {
  const h = choiceList([{ on: false, code: 'SQ', title: 'Shqip', data: { l: 'sq' } }, { on: true, code: 'EUR', money: true, title: XSS, data: { c: 'EUR' } }], { label: 'Choose' });
  const { root } = render(h);
  const list = root.children[0];
  assert.equal(list.getAttribute('role'), 'group');
  assert.equal(list.getAttribute('aria-label'), 'Choose');
  assert.ok(list.classList.contains('choices'));
  assert.equal(root.querySelector('[data-l="sq"]').getAttribute('aria-pressed'), 'false');
  assert.equal(root.querySelector('[data-c="EUR"]').getAttribute('aria-pressed'), 'true');
  assert.ok(root.querySelector('[data-c="EUR"] .choice-code').classList.contains('money'));
  assert.deepEqual(injected(h), []);
});

const STORE = new URL('.', import.meta.url);
const sources = readdirSync(STORE).filter(f => /\.(js|html)$/.test(f)).map(f => [f, readFileSync(new URL(f, STORE), 'utf8')]);

test('icons: every icon name the storefront asks for is drawn by /lib/icons.css', () => {
  const drawn = new Set([...readFileSync(new URL('../lib/icons.css', STORE), 'utf8').matchAll(/\.ti-([a-z0-9-]+)/g)].map(m => m[1]));
  const used = new Set();
  for (const [, s] of sources) for (const m of s.matchAll(/\b(?:icon|iconEnd):\s*'([a-z0-9-]+)'|\bicon\('([a-z0-9-]+)'|\bti-([a-z0-9-]+)/g)) used.add(m[1] || m[2] || m[3]);
  assert.ok(used.size > 20, 'found only ' + used.size);
  assert.deepEqual([...used].filter(n => !drawn.has(n)), [], 'icons that would draw nothing');
});

test('anchors: every data-tour in the storefront is listed with its file:line, and every listed one exists', () => {
  const listed = readFileSync(new URL('../../../../docs/learn/anchors-store.txt', STORE), 'utf8')
    .split('\n').filter(l => l.trim() && !l.startsWith('#')).map(l => l.trim().split(/\s+/));
  // An anchor is a LITERAL '<module>.<control>' string with one of these module names.
  const ANCHOR = /["'](top|cart|menu|dish|checkout|booking|nav|lang|currency|orders|map|voice|track)\.([a-zA-Z]+)["']/g;
  const inTree = new Set();
  for (const [, s] of sources) for (const line of s.split('\n')) {
    if (/^export const [A-Z_]+ = /.test(line)) continue;
    for (const m of line.matchAll(ANCHOR)) inTree.add(`${m[1]}.${m[2]}`);
  }
  const ids = listed.map(([id]) => id);
  assert.equal(new Set(ids).size, ids.length, 'an anchor is listed twice');
  for (const [id, at] of listed) {
    assert.ok(inTree.has(id), `${id} is listed but no storefront file writes it`);
    const [file, line] = at.split(':');
    assert.ok(readFileSync(new URL('../../../../' + file, STORE), 'utf8').split('\n')[Number(line) - 1].includes(`'${id}'`) ||
      readFileSync(new URL('../../../../' + file, STORE), 'utf8').split('\n')[Number(line) - 1].includes(`"${id}"`), `${id} is not on ${at}`);
  }
  for (const id of inTree) assert.ok(ids.includes(id), `${id} is in the tree and not listed`);
  for (const want of ['menu.category', 'menu.dish', 'dish.add', 'cart.open', 'checkout.place', 'booking.submit'])
    assert.ok(ids.includes(want), `the guest track needs ${want}`);
});
