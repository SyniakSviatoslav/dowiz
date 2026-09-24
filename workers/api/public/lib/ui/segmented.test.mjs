// lib/ui/segmented.js and lib/ui/tabs.js: markup + keyboard behaviour
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { segmented, bindSegmented } from './segmented.js';
import { tabs, bindTabs } from './tabs.js';
import { XSS, injected, render, fire } from './dom-shim.mjs';

const LANGS = [{ value: 'sq', label: 'SQ' }, { value: 'en', label: 'EN' }, { value: 'uk', label: 'UK' }];

test('segmented is a radiogroup; only the checked radio is tabbable', () => {
  const { root } = render(segmented({ label: 'Language', options: LANGS, value: 'en' }));
  const g = root.querySelector('[role="radiogroup"]');
  assert.equal(g.getAttribute('aria-label'), 'Language');
  const r = g.querySelectorAll('[role="radio"]');
  assert.deepEqual(r.map(x => x.getAttribute('aria-checked')), ['false', 'true', 'false']);
  assert.deepEqual(r.map(x => x.getAttribute('tabindex')), ['-1', '0', '-1']);
  // an unknown value falls back to the first option rather than none checked
  const f = render(segmented({ options: LANGS, value: 'de' })).root.querySelectorAll('[aria-checked="true"]');
  assert.equal(f.length, 1); assert.equal(f[0].dataset.value, 'sq');
  assert.throws(() => segmented({ options: [] }), /options/);
  assert.deepEqual(injected(segmented({ label: XSS, options: [{ value: XSS, label: XSS }] })), []);
});

test('bindSegmented: click selects and reports; clicking the checked one reports nothing', () => {
  const { root } = render(segmented({ options: LANGS, value: 'sq' }));
  const seen = [];
  bindSegmented(root.children[0], v => seen.push(v));
  const [sq, en] = root.querySelectorAll('[role="radio"]');
  fire(en, 'click');
  fire(en, 'click');
  fire(sq, 'click');
  assert.deepEqual(seen, ['en', 'sq']);
  assert.equal(sq.getAttribute('aria-checked'), 'true');
});

test('bindSegmented: arrows move and wrap, Home/End jump, focus follows', () => {
  const { root, doc } = render(segmented({ options: LANGS, value: 'sq' }));
  const seen = [];
  bindSegmented(root.children[0], v => seen.push(v));
  const [sq, , uk] = root.querySelectorAll('[role="radio"]');
  const e = fire(sq, 'keydown', { key: 'ArrowLeft' });
  assert.ok(e.defaultPrevented);
  assert.equal(doc.activeElement, uk);
  fire(uk, 'keydown', { key: 'Home' });
  fire(sq, 'keydown', { key: 'End' });
  assert.deepEqual(seen, ['uk', 'sq', 'uk']);
  assert.equal(fire(sq, 'keydown', { key: 'a' }).defaultPrevented, false);
});

test('tabs: tablist, aria-selected, aria-controls; bindTabs toggles panels', () => {
  const html = tabs({ label: 'Sections', items: [{ id: 'q', label: 'Queue', badge: 3 }, { id: 'h', label: 'History' }], active: 'q' })
    + '<div id="q-panel">Q</div><div id="h-panel" hidden>H</div>';
  const { root, doc } = render(html);
  const list = root.querySelector('[role="tablist"]');
  const [q, h] = list.querySelectorAll('[role="tab"]');
  assert.equal(q.getAttribute('aria-selected'), 'true');
  assert.equal(q.getAttribute('aria-controls'), 'q-panel');
  assert.equal(q.querySelector('.ui-tab-badge').textContent, '3');
  const seen = [];
  bindTabs(list, id => seen.push(id));
  fire(q, 'keydown', { key: 'ArrowRight' });
  assert.deepEqual(seen, ['h']);
  assert.equal(doc.getElementById('h-panel').hidden, false);
  assert.equal(doc.getElementById('q-panel').hidden, true);
  assert.equal(doc.activeElement, h);
  assert.throws(() => tabs({ items: [] }), /items/);
  assert.deepEqual(injected(tabs({ label: XSS, items: [{ id: 'x', label: XSS, badge: XSS }] })), []);
});
