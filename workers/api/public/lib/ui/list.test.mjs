// lib/ui/list.js, lib/ui/empty.js, lib/ui/skeleton.js
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { row, list } from './list.js';
import { emptyState } from './empty.js';
import { skeleton } from './skeleton.js';
import { amount } from './money.js';
import { useTranslator } from './core.js';
import { XSS, injected, render } from './dom-shim.mjs';

test('a plain row is a listitem inside role=list', () => {
  const { root } = render(list([row({ title: 'Today', sub: '3 deliveries', trailing: amount('4 500 L') })], { label: 'Takings' }));
  const l = root.querySelector('.ui-list');
  assert.equal(l.getAttribute('role'), 'list');
  assert.equal(l.getAttribute('aria-label'), 'Takings');
  const r = l.querySelector('.ui-row');
  assert.equal(r.getAttribute('role'), 'listitem');
  assert.equal(r.querySelector('.ui-row-title').textContent, 'Today');
  assert.ok(r.querySelector('.ui-row-trail .money'));
});

test('a selectable row is a button with aria-pressed and a drawn radio; its list is a group', () => {
  const rows = [row({ title: '#a1', select: true, pressed: true, data: { sel: 'a1' } }), row({ title: '#b2', select: true, pressed: false, data: { sel: 'b2' } })];
  const { root } = render(list(rows, { label: 'Ready' }));
  const l = root.querySelector('.ui-list');
  assert.equal(l.getAttribute('role'), 'group');
  const [a, b] = l.querySelectorAll('button');
  assert.equal(a.getAttribute('type'), 'button');
  assert.equal(a.getAttribute('aria-pressed'), 'true');
  assert.equal(b.getAttribute('aria-pressed'), 'false');
  assert.equal(a.dataset.sel, 'a1');
  assert.ok(a.querySelector('.ui-radio[aria-hidden="true"]'));
  // no icon is involved in the mark: the icons it used to need never existed
  assert.equal(a.querySelector('.ti'), null);
});

test('a row with href is a link', () => {
  const { root } = render(row({ title: 'x', href: '/courier/' }));
  assert.equal(root.children[0].tagName, 'A');
});

test('rows escape title, sub and data', () => {
  assert.deepEqual(injected(list([row({ title: XSS, sub: XSS, data: { sel: XSS } }), row({ title: XSS, select: true, data: { sel: XSS } })], { label: XSS })), []);
});

test('emptyState: icon, title, body, optional action; role only when asked', () => {
  const html = emptyState({ icon: 'radar-2', title: 'No free orders', body: 'It appears here', action: '<button id="retry">r</button>' });
  const e = render(html).root.children[0];
  assert.equal(e.getAttribute('role'), null);
  assert.ok(e.querySelector('.ti-radar-2'));
  assert.equal(e.querySelector('.ui-empty-title').textContent, 'No free orders');
  assert.ok(e.querySelector('#retry'));
  assert.equal(render(emptyState({ title: 'x', alert: true })).root.children[0].getAttribute('role'), 'alert');
  assert.equal(render(emptyState({ title: 'x', status: true })).root.children[0].getAttribute('role'), 'status');
  assert.deepEqual(injected(emptyState({ title: XSS, body: XSS, reason: XSS })), []);
  assert.throws(() => emptyState({ title: 'x', tone: 'grey' }), /tone/);
});

test('emptyState i18n keys', () => {
  useTranslator(k => ({ none: 'Asgjë' }[k] ?? k));
  try { assert.equal(render(emptyState({ title: { t: 'none' } })).root.querySelector('[data-t]').getAttribute('data-t'), 'none'); }
  finally { useTranslator(null); }
});

test('skeleton: busy, labelled, the shapes asked for; unknown shape throws', () => {
  const s = render(skeleton({ shapes: ['title', 'block', 'button'], label: 'Loading' })).root.children[0];
  assert.equal(s.getAttribute('aria-busy'), 'true');
  assert.equal(s.getAttribute('aria-label'), 'Loading');
  assert.deepEqual(s.children.map(c => c.className), ['ui-skel ui-skel--title', 'ui-skel ui-skel--block', 'ui-skel ui-skel--button']);
  assert.equal(render(skeleton({ shape: 'row', count: 3 })).root.querySelectorAll('.ui-skel--row').length, 3);
  assert.throws(() => skeleton({ shapes: ['blob'] }), /unknown shape/);
  assert.deepEqual(injected(skeleton({ label: XSS })), []);
});

test('an acting row is a plain button: no aria-pressed, no radio, its data kept', () => {
  const r = row({ title: 'O1a', act: true, data: { learn: 'O1a' } });
  assert.match(r, /^<button\b[^>]*type="button"/);
  assert.match(r, /ui-row--action/);
  assert.match(r, /data-learn="O1a"/);
  assert.doesNotMatch(r, /aria-pressed|ui-radio/);
  assert.doesNotMatch(row({ title: 'x' }), /^<button/);
});
