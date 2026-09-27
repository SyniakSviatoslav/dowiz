import test from 'node:test';
import assert from 'node:assert/strict';
import { tree } from './fixture.mjs';
import { flatten, pickDict, completeness, stubBrowser, rootHook, collect } from './i18n.mjs';

const byId = xs => Object.fromEntries(xs.map(x => [x.id, x]));

test('flatten: nested keys dotted; arrays and functions are leaves', () => {
  assert.deepEqual(flatten({ a: 1, b: { c: [1], d: () => 1, e: { f: null } } }), ['a', 'b.c', 'b.d', 'b.e.f']);
});

test('pickDict prefers T, else any export with every language', () => {
  const d = { sq: {}, en: {}, uk: {}, ru: {} };
  assert.equal(pickDict({ T: d }), d);
  assert.equal(pickDict({ other: 1, dict: d }), d);
  assert.equal(pickDict({ half: { sq: {}, en: {} } }), null);
});

test('completeness: the union less each language, qtyWords excluded', () => {
  const c = completeness({ sq: { a: 1, qtyWords: { one: 'x' } }, en: { a: 1, b: 1 }, uk: { a: 1, b: 1 }, ru: { a: 1, b: 1 } });
  assert.equal(c.union, 2);
  assert.deepEqual(c.per.sq, { keys: 1, missing: ['b'] });
  assert.deepEqual(c.per.en, { keys: 2, missing: [] });
});

test('the browser stub is inert and never overwrites a real global', () => {
  const g = { navigator: 'mine' };
  stubBrowser(g);
  assert.equal(g.navigator, 'mine');
  assert.equal(g.window, g);
  g.localStorage.setItem('k', 1);
  assert.equal(g.localStorage.getItem('k'), '1');
  assert.equal(g.localStorage.getItem('none'), null);
  g.localStorage.removeItem('k');
  assert.equal(g.localStorage.getItem('k'), null);
  const el = g.document.createElement('div');
  el.setAttribute(); el.addEventListener(); el.classList.add(); el.classList.remove(); el.classList.toggle();
  g.document.addEventListener();
  assert.equal(g.document.querySelector('x'), null);
  assert.deepEqual(g.document.querySelectorAll('x'), []);
  const w = { window: 'w' };
  stubBrowser(w);
  assert.equal(w.window, 'w');
});

test('the resolve hook maps a served-root specifier only from inside the tree', async () => {
  const src = decodeURIComponent(rootHook('file:///pub/').slice('data:text/javascript,'.length));
  const mod = await import('data:text/javascript,' + encodeURIComponent(src));
  const next = (s) => s;
  assert.equal(await mod.resolve('/a.js', { parentURL: 'file:///pub/x.js' }, next), 'file:///pub/a.js');
  assert.equal(await mod.resolve('/a.js', { parentURL: 'file:///elsewhere/x.js' }, next), '/a.js');
  assert.equal(await mod.resolve('./b.js', {}, next), './b.js');
});

test('the collector imports each dictionary the way the page does', async () => {
  const r = byId(await collect({ root: tree() }));
  assert.equal(r['i18n.store.missing_sq'].value, 1);
  assert.equal(r['i18n.store.missing_sq'].note, 'b.c');
  assert.equal(r['i18n.store.missing_en'].value, 0);
  assert.equal(r['i18n.store.missing_en'].note, undefined);
  assert.equal(r['i18n.admin.keys'].value, 3);
  assert.equal(r['i18n.admin.missing_uk'].note, 'd');
  assert.equal(r['i18n.room.importable'].value, 0);
  assert.equal(r['i18n.room.importable'].note, 'no export holds sq, en, uk, ru');
  assert.equal(r['i18n.courier.importable'].note, 'boom at import');
});
