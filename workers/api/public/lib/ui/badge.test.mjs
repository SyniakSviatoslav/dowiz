// lib/ui/badge.js and lib/ui/chip.js
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { badge, status, STATUSES } from './badge.js';
import { chip } from './chip.js';
import { useTranslator } from './core.js';
import { XSS, injected, render } from './dom-shim.mjs';

const one = html => render(html).root.children[0];

test('badge: tone class, dot, icon, escaped label', () => {
  const b = one(badge({ label: '3 unsent', tone: 'warning', dot: true, icon: 'cloud-upload' }));
  assert.ok(b.classList.contains('ui-badge--warning'));
  assert.ok(b.querySelector('.ui-dot'));
  assert.equal(b.textContent, '3 unsent');
  assert.deepEqual(injected(badge({ label: XSS })), []);
  assert.throws(() => badge({ label: 'x', tone: 'red' }), /tone/);
});

test('badge live region only when asked', () => {
  assert.equal(one(badge({ label: 'x', live: true })).getAttribute('aria-live'), 'polite');
  assert.equal(one(badge({ label: 'x' })).getAttribute('role'), null);
});

test('status: known FSM state carries data-status; unknown renders UNKNOWN, not a throw', () => {
  for (const s of STATUSES) assert.equal(one(status({ label: s, status: s })).getAttribute('data-status'), s);
  assert.equal(one(status({ label: 'x', status: 'TELEPORTED' })).getAttribute('data-status'), 'UNKNOWN');
  assert.ok(one(status({ label: 'x', status: 'READY', pulse: true })).classList.contains('ui-status--pulse'));
  assert.deepEqual(injected(status({ label: XSS, status: XSS })), []);
});

test('chip: span readout vs button toggle', () => {
  const s = one(chip({ label: 'offline', dot: true, floating: true, id: 'shiftTag' }));
  assert.equal(s.tagName, 'SPAN');
  assert.ok(s.classList.contains('ui-chip--float'));
  const b = one(chip({ label: 'SQ', as: 'button', selected: true }));
  assert.equal(b.tagName, 'BUTTON');
  assert.equal(b.getAttribute('type'), 'button');
  assert.equal(b.getAttribute('aria-pressed'), 'true');
  assert.ok(one(chip({ label: 'x', hidden: true })).hidden);
  assert.equal(one(chip({ label: 'x', live: true })).getAttribute('role'), 'status');
  assert.deepEqual(injected(chip({ label: XSS, ariaLabel: XSS })), []);
});

test('chip i18n key hook', () => {
  useTranslator(k => ({ offline: 'офлайн' }[k] ?? k));
  try {
    const c = one(chip({ label: { t: 'offline' } }));
    assert.equal(c.textContent, 'офлайн');
    assert.equal(c.querySelector('[data-t]').getAttribute('data-t'), 'offline');
  } finally { useTranslator(null); }
});
