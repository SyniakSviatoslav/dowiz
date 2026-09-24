// lib/ui/toast.js and lib/ui/sheet.js: the two behaviours with timing and focus
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { toastHost, createToaster } from './toast.js';
import { sheet, openSheet, confirmSheet } from './sheet.js';
import { XSS, injected, render, fire, Document } from './dom-shim.mjs';

function fakeTimers(){
  let n = 0; const q = new Map();
  return { setTimeout: (f, ms) => { q.set(++n, { f, ms }); return n; }, clearTimeout: id => q.delete(id),
           flush(){ for (const [id, { f }] of [...q]) { q.delete(id); f(); } }, get pending(){ return [...q.values()].map(x => x.ms); } };
}

test('toast host exists hidden, as a polite atomic live region', () => {
  const h = render(toastHost()).root.children[0];
  assert.equal(h.id, 'toast');
  assert.ok(h.hidden);
  assert.equal(h.getAttribute('role'), 'status');
  assert.equal(h.getAttribute('aria-live'), 'polite');
  assert.equal(h.getAttribute('aria-atomic'), 'true');
});

test('toaster shows escaped text with an icon, then hides after its time', () => {
  const { root } = render(toastHost());
  const host = root.children[0], timers = fakeTimers();
  const t = createToaster(host, { timers });
  t.show(XSS, { icon: 'alert-circle', tone: 'danger' });
  assert.equal(host.hidden, false);
  assert.deepEqual(injected(host.innerHTML), []);
  assert.equal(host.textContent, XSS);
  assert.ok(host.classList.contains('ui-toast--danger'));
  assert.ok(host.querySelector('.ti-alert-circle'));
  assert.deepEqual(timers.pending, [3000]);
  // a second toast replaces the first and restarts the clock -- one timer, not two
  t.show('again', { ms: 5000 });
  assert.deepEqual(timers.pending, [5000]);
  assert.equal(host.classList.contains('ui-toast--danger'), false);
  timers.flush();
  assert.equal(host.hidden, true);
  assert.throws(() => t.show('x', { tone: 'rainbow' }), /tone/);
});

test('sheet markup: dialog, modal, labelled by its title; close control has a name', () => {
  const s = render(sheet({ id: 's', title: 'Refund', closeLabel: 'Close', body: '<p>b</p>' })).root.children[0];
  assert.equal(s.getAttribute('role'), 'dialog');
  assert.equal(s.getAttribute('aria-modal'), 'true');
  assert.equal(s.getAttribute('aria-labelledby'), 's-title');
  assert.equal(s.querySelector('#s-title').textContent, 'Refund');
  assert.equal(s.querySelector('[data-ui-close]').getAttribute('aria-label'), 'Close');
  assert.deepEqual(injected(sheet({ title: XSS, closeLabel: XSS })), []);
});

test('openSheet: focus moves in, Tab is trapped, Escape closes and returns focus', () => {
  const doc = new Document();
  const opener = doc.createElement('button'); doc.body.appendChild(opener); opener.focus();
  const seen = [];
  const s = openSheet({ title: 'T', body: '<button id="a">a</button><button id="b">b</button>', onClose: v => seen.push(v) }, doc);
  const a = s.el.querySelector('#a'), b = s.el.querySelector('#b');
  assert.equal(doc.activeElement, a);
  const e = fire(b, 'keydown', { key: 'Tab' }); b.focus();
  const e2 = fire(doc.body, 'keydown', { key: 'Tab' });
  assert.ok(e2.defaultPrevented, 'Tab from the last control wraps');
  assert.equal(doc.activeElement, a);
  assert.ok(e);
  fire(doc.body, 'keydown', { key: 'Escape' });
  assert.deepEqual(seen, ['escape']);
  assert.equal(doc.activeElement, opener);
  assert.equal(doc.querySelector('.ui-scrim'), null);
});

test('openSheet: data-ui-close reports its value; the scrim closes; close is idempotent', () => {
  const doc = new Document();
  const seen = [];
  const s = openSheet({ title: 'T', body: '<button data-ui-close="keep">k</button>', onClose: v => seen.push(v) }, doc);
  fire(s.el.querySelector('[data-ui-close="keep"]'), 'click');
  s.close('again');
  assert.deepEqual(seen, ['keep']);
  const s2 = openSheet({ title: 'T', onClose: v => seen.push(v) }, doc);
  fire(doc.querySelector('.ui-scrim'), 'click');
  assert.deepEqual(seen, ['keep', 'scrim']);
  assert.ok(s2);
});

test('confirmSheet resolves true only on confirm', async () => {
  const doc = new Document();
  const p = confirmSheet({ title: 'Deliver?', confirmLabel: 'Yes', cancelLabel: 'No' }, doc);
  fire(doc.querySelector('[data-ui-close="yes"]'), 'click');
  assert.equal(await p, true);
  const q = confirmSheet({ title: 'Deliver?' }, doc);
  fire(doc.querySelector('[data-ui-close="no"]'), 'click');
  assert.equal(await q, false);
  const r = confirmSheet({ title: 'x' }, doc);
  fire(doc.body, 'keydown', { key: 'Escape' });
  assert.equal(await r, false);
});
