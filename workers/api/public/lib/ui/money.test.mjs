// lib/ui/money.js, lib/ui/card.js, lib/ui/time.js
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { amount, stat } from './money.js';
import { card, section, para } from './card.js';
import { when, mmss } from './time.js';
import { formatter } from '../money.js';
import { useTranslator } from './core.js';
import { XSS, injected, render } from './dom-shim.mjs';

const fmt = formatter({ base: 'ALL', display: 'ALL', rates: null, locale: 'sq' });

test('amount wears .money, takes the FORMATTED string from /lib/money.js, and refuses a raw number', () => {
  const a = render(amount(fmt(1500), { size: 'lg', strong: true })).root.children[0];
  assert.ok(a.classList.contains('money'));
  assert.ok(a.classList.contains('ui-amount--lg'));
  assert.equal(a.textContent, fmt(1500));
  assert.throws(() => amount(1500), /formatted string/);
  assert.throws(() => amount('1', { size: 'huge' }), /size/);
  assert.equal(render(amount('200 L', { sign: '+', tone: 'success' })).root.textContent, '+200 L');
  assert.equal(render(amount('200 L', { sign: '-' })).root.textContent, '−200 L');
  assert.deepEqual(injected(amount(XSS)), []);
});

test('stat: label text + value markup + optional hint', () => {
  const s = render(stat({ label: 'Cash in hand', value: amount('4 500 L', { size: 'xl' }), emphasis: true })).root.children[0];
  assert.ok(s.classList.contains('ui-stat--hero'));
  assert.equal(s.querySelector('.ui-stat-k').textContent, 'Cash in hand');
  assert.ok(s.querySelector('.ui-stat-v .money'));
  assert.deepEqual(injected(stat({ label: XSS, hint: XSS })), []);
});

test('card and section', () => {
  const c = render(card({ eyebrow: 'Offered', title: '#a1', tone: 'accent', body: '<p>x</p>' })).root.children[0];
  assert.ok(c.classList.contains('ui-card--accent'));
  assert.equal(c.querySelector('.ui-card-title').tagName, 'H3');
  assert.throws(() => card({ tone: 'gold' }), /tone/);
  const s = render(section({ title: 'History', back: { id: 'pback', label: 'Back' } })).root.children[0];
  assert.equal(s.querySelector('#pback').getAttribute('aria-label'), 'Back');
  assert.equal(s.querySelector('h2').textContent, 'History');
  assert.equal(render(section({ title: 'x', level: 3 })).root.querySelector('h3').textContent, 'x');
  assert.deepEqual(injected(card({ eyebrow: XSS, title: XSS }) + section({ title: XSS, sub: XSS, back: { label: XSS } }) + para(XSS)), []);
});

test('para: body and hint voices, i18n key', () => {
  useTranslator(k => ({ h: 'Ndihmë' }[k] ?? k));
  try {
    const p = render(para({ t: 'h' }, { hint: true, center: true })).root.children[0];
    assert.equal(p.tagName, 'P');
    assert.ok(p.classList.contains('ui-hint') && p.classList.contains('ui-center'));
    assert.equal(p.getAttribute('data-t'), 'h');
  } finally { useTranslator(null); }
});

test('when: a <time> with the ISO instant, formatted in the given locale and zone', () => {
  const at = Date.UTC(2026, 8, 24, 12, 5);
  const t = render(when(at, { locale: 'uk', style: 'time', timeZone: 'UTC' })).root.children[0];
  assert.equal(t.tagName, 'TIME');
  assert.equal(t.getAttribute('datetime'), '2026-09-24T12:05:00.000Z');
  assert.equal(t.textContent, '12:05');
  assert.match(render(when(at, { locale: 'uk', style: 'date', timeZone: 'UTC' })).root.textContent, /24/);
  assert.throws(() => when('soon'), /instant/);
  assert.throws(() => when(at, { style: 'ago' }), /style/);
});

test('mmss', () => {
  assert.equal(mmss(0), '0:00'); assert.equal(mmss(65), '1:05'); assert.equal(mmss(-3), '0:00'); assert.equal(mmss(299.9), '4:59');
});
