// lib/ui/button.js
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { button, iconButton, setBusy, VARIANTS } from './button.js';
import { useTranslator } from './core.js';
import { XSS, injected, render } from './dom-shim.mjs';

const one = html => render(html).root.children[0];

test('button renders a real <button type=button> with variant and size classes', () => {
  const b = one(button({ label: 'Take', variant: 'primary', size: 'lg', block: true, icon: 'package', id: 'take' }));
  assert.equal(b.tagName, 'BUTTON');
  assert.equal(b.getAttribute('type'), 'button');
  assert.equal(b.id, 'take');
  for (const c of ['ui-btn', 'ui-btn--primary', 'ui-btn--lg', 'ui-btn--block']) assert.ok(b.classList.contains(c), c);
  assert.equal(b.textContent, 'Take');
  assert.ok(b.querySelector('.ti-package'));
  assert.equal(one(button({ label: 'Go', type: 'submit' })).getAttribute('type'), 'submit');
});

test('every variant renders; an unknown variant or a size under --tap throws', () => {
  for (const v of VARIANTS) assert.ok(one(button({ label: v, variant: v })).classList.contains(`ui-btn--${v}`));
  assert.throws(() => button({ label: 'x', variant: 'link' }), /variant/);
  assert.throws(() => button({ label: 'x', size: 'sm' }), /--tap/);
});

test('button escapes its label and attributes', () => {
  const html = button({ label: XSS, ariaLabel: XSS, title: XSS, id: 'a' });
  assert.deepEqual(injected(html), []);
  const b = one(html);
  assert.equal(b.textContent, XSS);
  assert.equal(b.getAttribute('aria-label'), XSS);
});

test('i18n: a key label carries data-t, a key aria-label carries data-t-attr', () => {
  useTranslator(k => ({ signIn: 'Hyni', back: 'Prapa' }[k] ?? k));
  try {
    const b = one(button({ label: { t: 'signIn' }, ariaLabel: { t: 'back' } }));
    assert.equal(b.querySelector('[data-t]').getAttribute('data-t'), 'signIn');
    assert.equal(b.textContent, 'Hyni');
    assert.equal(b.getAttribute('data-t-attr'), 'aria-label:back');
  } finally { useTranslator(null); }
});

test('states: disabled, pressed, busy (disabled + aria-busy + spinner)', () => {
  assert.ok(one(button({ label: 'x', disabled: true })).disabled);
  assert.equal(one(button({ label: 'x', pressed: true })).getAttribute('aria-pressed'), 'true');
  assert.equal(one(button({ label: 'x' })).getAttribute('aria-pressed'), null);
  const busy = one(button({ label: 'Save', busy: true, busyLabel: 'Saving…' }));
  assert.ok(busy.disabled);
  assert.equal(busy.getAttribute('aria-busy'), 'true');
  assert.ok(busy.querySelector('.ti-loader-2.ui-spin'));
  assert.equal(busy.textContent, 'Saving…');
});

test('href renders a link that looks like a button; _blank gets noopener', () => {
  const a = one(button({ label: 'Maps', href: 'https://x.test/?q=a&b', target: '_blank' }));
  assert.equal(a.tagName, 'A');
  assert.equal(a.getAttribute('href'), 'https://x.test/?q=a&b');
  assert.equal(a.getAttribute('rel'), 'noopener');
  assert.equal(a.getAttribute('type'), null);
});

test('iconButton requires an accessible name; renders round control', () => {
  assert.throws(() => iconButton({ icon: 'x' }), /ariaLabel/);
  const b = one(iconButton({ icon: 'sun-moon', ariaLabel: 'Theme', id: 't', pressed: false }));
  assert.equal(b.getAttribute('aria-label'), 'Theme');
  assert.equal(b.getAttribute('aria-pressed'), 'false');
  assert.ok(b.classList.contains('ui-iconbtn'));
  assert.equal(one(iconButton({ text: 'SQ', ariaLabel: 'Language' })).textContent, 'SQ');
});

test('setBusy disables, labels, and restores exactly', () => {
  const b = one(button({ label: 'Picked up', icon: 'package' }));
  const before = b.innerHTML;
  const undo = setBusy(b, 'Saving <now>');
  assert.ok(b.disabled);
  assert.equal(b.getAttribute('aria-busy'), 'true');
  assert.equal(b.textContent, 'Saving <now>');
  undo();
  assert.equal(b.disabled, false);
  assert.equal(b.getAttribute('aria-busy'), null);
  assert.equal(b.innerHTML, before);
});
