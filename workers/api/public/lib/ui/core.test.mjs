// lib/ui/core.js: the escaper, the attribute builder, icons and the i18n hook.
// `node --test workers/api/public/lib/ui/`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { esc, cx, attrs, merge, icon, label, text, i18nAttr, useTranslator, tone } from './core.js';
import { XSS, injected, render } from './dom-shim.mjs';

test('esc escapes the five characters and nothing else', () => {
  assert.equal(esc(`<a href="x">'&</a>`), '&lt;a href=&quot;x&quot;&gt;&#39;&amp;&lt;/a&gt;');
  assert.equal(esc('Durrës 1500 L'), 'Durrës 1500 L');
  assert.equal(esc(null), ''); assert.equal(esc(0), '0');
});

test('cx drops falsy parts', () => {
  assert.equal(cx('a', false, null, '', 'c', ['d', undefined]), 'a c d');
});

test('attrs escapes values, renders booleans, data-*, and refuses style= and on*=', () => {
  assert.equal(attrs({ id: 'x', title: XSS, disabled: true, hidden: false, n: null }),
    ` id="x" title="${esc(XSS)}" disabled`);
  assert.deepEqual(injected(`<b${attrs({ title: XSS })}>x</b>`), []);
  assert.equal(attrs({ data: { orderId: '7', on: true, off: false } }), ' data-order-id="7" data-on');
  assert.throws(() => attrs({ style: 'color:red' }), /CSP/);
  assert.throws(() => attrs({ onclick: 'x()' }), /CSP/);
  assert.throws(() => attrs({ 'a b': 1 }), /bad attribute/);
  // positive twin: an ordinary attribute with "on" inside the name is fine
  assert.equal(attrs({ 'aria-controls': 'p' }), ' aria-controls="p"');
});

test('icon validates the name and is hidden from assistive technology', () => {
  assert.equal(icon('check'), '<i class="ti ti-check ui-i" aria-hidden="true"></i>');
  assert.equal(icon(''), '');
  assert.throws(() => icon('x" onerror="y'), /bad icon/);
});

test('label: plain text is escaped; a key renders data-t for retranslate()', () => {
  useTranslator((k, v) => ({ hi: 'Përshëndetje', n: `N=${v && v.n}` }[k] ?? k));
  try {
    assert.equal(label('a<b'), '<span>a&lt;b</span>');
    assert.equal(label({ t: 'hi' }), '<span data-t="hi">Përshëndetje</span>');
    // a key with variables cannot be re-translated by data-t, so it carries none
    assert.equal(label({ t: 'n', vars: { n: 3 } }), '<span>N=3</span>');
    assert.equal(text({ t: 'hi' }), 'Përshëndetje');
    assert.deepEqual(i18nAttr('aria-label', { t: 'hi' }), { 'aria-label': 'Përshëndetje', 'data-t-attr': 'aria-label:hi' });
    assert.equal(label(''), ''); assert.equal(label(null), '');
    const { root } = render(label(XSS));
    assert.equal(root.textContent, XSS);
  } finally { useTranslator(null); }
});

test('merge concatenates data-t-attr so two translated attributes both survive', () => {
  const m = merge(i18nAttr('aria-label', { t: 'a' }), i18nAttr('title', { t: 'b' }));
  assert.equal(attrs(m), ' aria-label="a" title="b" data-t-attr="aria-label:a title:b"');
});

test('tone refuses an unknown tone (a typo is loud) and has a positive twin', () => {
  assert.throws(() => tone('sucess'), /unknown tone/);
  assert.equal(tone('success'), 'success');
  assert.equal(tone(undefined), 'neutral');
});
