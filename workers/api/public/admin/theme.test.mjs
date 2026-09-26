// theme.js: the three choices, what each writes on the document, and that a
// broken storage never breaks the page.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { THEMES, KEY, GROUND, pick, next, colorFor, apply, stored, choose } from './theme.js';

/// A document with an <html> and two theme-color metas, as index.html has.
function doc(){
  const attrs = new Map();
  const meta = content => ({ dataset: {}, _c: content, getAttribute(n){ return n === 'content' ? this._c : null; }, setAttribute(n, v){ if (n === 'content') this._c = v; } });
  const metas = [meta('#f2f1ec'), meta('#141416')];
  return {
    documentElement: { setAttribute: (n, v) => attrs.set(n, v), removeAttribute: n => attrs.delete(n), getAttribute: n => attrs.get(n) ?? null },
    querySelectorAll: sel => (sel === 'meta[name="theme-color"]' ? metas : []),
    metas,
  };
}

test('pick: a theme is itself, anything else is auto', () => {
  for (const v of THEMES) assert.equal(pick(v), v);
  assert.equal(pick('sepia'), 'auto');
  assert.equal(pick(null), 'auto');
  assert.equal(pick(undefined), 'auto');
});

test('next: cycles auto -> light -> dark -> auto, from any starting value', () => {
  assert.equal(next('auto'), 'light');
  assert.equal(next('light'), 'dark');
  assert.equal(next('dark'), 'auto');
  assert.equal(next('garbage'), 'light');
});

test('colorFor: a forced scheme names its ground, auto names none', () => {
  assert.equal(colorFor('light'), GROUND.light);
  assert.equal(colorFor('dark'), GROUND.dark);
  assert.equal(colorFor('auto'), null);
  assert.equal(colorFor('x'), null);
});

test('apply: dark sets data-theme and both metas; auto removes it and restores each meta', () => {
  const d = doc();
  assert.equal(apply(d, 'dark'), 'dark');
  assert.equal(d.documentElement.getAttribute('data-theme'), 'dark');
  assert.deepEqual(d.metas.map(m => m._c), [GROUND.dark, GROUND.dark]);
  assert.equal(apply(d, 'light'), 'light');
  assert.equal(d.documentElement.getAttribute('data-theme'), 'light');
  assert.deepEqual(d.metas.map(m => m._c), [GROUND.light, GROUND.light]);
  assert.equal(apply(d, 'auto'), 'auto');
  assert.equal(d.documentElement.getAttribute('data-theme'), null);
  assert.deepEqual(d.metas.map(m => m._c), ['#f2f1ec', '#141416']);
  assert.deepEqual(d.metas.map(m => m.dataset.auto), [undefined, undefined]);
});

test('apply: auto on a fresh document touches nothing (positive twin of the restore)', () => {
  const d = doc();
  apply(d, 'auto');
  assert.deepEqual(d.metas.map(m => m._c), ['#f2f1ec', '#141416']);
  assert.equal(d.documentElement.getAttribute('data-theme'), null);
});

test('stored: reads the key, defaults to auto, survives a storage that throws', () => {
  assert.equal(stored(k => (k === KEY ? 'dark' : null)), 'dark');
  assert.equal(stored(() => null), 'auto');
  assert.equal(stored(() => 'nope'), 'auto');
  assert.equal(stored(() => { throw new Error('private'); }), 'auto');
});

test('choose: remembers and applies; a storage that throws still applies', () => {
  const d = doc(); const mem = {};
  assert.equal(choose(d, 'dark', (k, v) => { mem[k] = v; }), 'dark');
  assert.equal(mem[KEY], 'dark');
  assert.equal(d.documentElement.getAttribute('data-theme'), 'dark');
  const d2 = doc();
  assert.equal(choose(d2, 'light', () => { throw new Error('quota'); }), 'light');
  assert.equal(d2.documentElement.getAttribute('data-theme'), 'light');
  assert.equal(choose(d2, 'bogus', () => {}), 'auto');
  assert.equal(d2.documentElement.getAttribute('data-theme'), null);
});
