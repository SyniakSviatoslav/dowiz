// lib/ui/field.js: field, inputRow, alert
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { field, inputRow, alert } from './field.js';
import { iconButton } from './button.js';
import { useTranslator } from './core.js';
import { XSS, injected, render } from './dom-shim.mjs';

test('field wires label[for] to the control, and hint + error into aria-describedby', () => {
  const { root } = render(field({ id: 'em', label: 'Email', hint: 'We never share it', error: 'Wrong', autocomplete: 'username' }));
  const input = root.querySelector('input');
  assert.equal(root.querySelector('label').getAttribute('for'), 'em');
  assert.equal(input.id, 'em');
  assert.equal(input.getAttribute('aria-describedby'), 'em-hint em-err');
  assert.equal(input.getAttribute('aria-invalid'), 'true');
  assert.equal(root.querySelector('#em-err').getAttribute('role'), 'alert');
  assert.equal(input.getAttribute('autocomplete'), 'username');
  assert.ok(root.querySelector('.ui-field--invalid'));
});

test('field without error or hint has no describedby and is not invalid (positive twin)', () => {
  const { root } = render(field({ id: 'pw', label: 'Password', type: 'password' }));
  const input = root.querySelector('input');
  assert.equal(input.getAttribute('aria-describedby'), null);
  assert.equal(input.getAttribute('aria-invalid'), null);
  assert.equal(input.getAttribute('type'), 'password');
});

test('field generates an id when none is given, and they stay paired', () => {
  const { root } = render(field({ label: 'X' }));
  assert.equal(root.querySelector('label').getAttribute('for'), root.querySelector('input').id);
});

test('field escapes label, value, placeholder, hint and error', () => {
  const html = field({ id: 'a', label: XSS, value: XSS, placeholder: XSS, hint: XSS, error: XSS });
  assert.deepEqual(injected(html), []);
  const { root } = render(html);
  assert.equal(root.querySelector('input').getAttribute('value'), XSS);
  assert.equal(root.querySelector('label').textContent, XSS);
});

test('textarea when rows is set; money field is numeric and wears .money', () => {
  const { root } = render(field({ id: 'n', label: 'Note', rows: 3, maxlength: 280, value: 'a<b' }));
  const ta = root.querySelector('textarea');
  assert.equal(ta.getAttribute('rows'), '3');
  assert.equal(ta.getAttribute('maxlength'), '280');
  assert.equal(ta.textContent, 'a<b');
  const m = render(field({ id: 'got', label: 'Cash', money: true, value: 1500 })).root.querySelector('input');
  assert.ok(m.classList.contains('money'));
  assert.equal(m.getAttribute('inputmode'), 'numeric');
  assert.equal(m.getAttribute('pattern'), '[0-9]*');
  assert.equal(m.getAttribute('value'), '1500');
  assert.throws(() => field({ type: 'colour' }), /unknown type/);
});

test('i18n keys on label and placeholder', () => {
  useTranslator(k => ({ pw: 'Fjalëkalimi', ph: 'p.sh.' }[k] ?? k));
  try {
    const { root } = render(field({ id: 'p', label: { t: 'pw' }, placeholder: { t: 'ph' } }));
    assert.equal(root.querySelector('label [data-t]').getAttribute('data-t'), 'pw');
    assert.equal(root.querySelector('input').getAttribute('placeholder'), 'p.sh.');
    assert.equal(root.querySelector('input').getAttribute('data-t-attr'), 'placeholder:ph');
  } finally { useTranslator(null); }
});

test('inputRow: the input is named by aria-label and the action sits beside it', () => {
  const { root } = render(inputRow({ id: 'askBox', label: 'Ask', placeholder: XSS, action: iconButton({ id: 'askGo', icon: 'send', ariaLabel: 'Send' }) }));
  assert.equal(root.querySelector('#askBox').getAttribute('aria-label'), 'Ask');
  assert.ok(root.querySelector('#askGo'));
  assert.deepEqual(injected(inputRow({ label: XSS, placeholder: XSS })), []);
});

test('alert: danger is announced as alert, others as status', () => {
  assert.equal(render(alert({ label: 'no' })).root.children[0].getAttribute('role'), 'alert');
  assert.equal(render(alert({ label: 'ok', tone: 'success' })).root.children[0].getAttribute('role'), 'status');
  assert.deepEqual(injected(alert({ label: XSS })), []);
  assert.throws(() => alert({ label: 'x', tone: 'loud' }), /tone/);
});
