// The delivery-area editor, in node: the body it posts is the one the hub's
// `set_zones` reads, and the editor round-trips what the storefront sends.
// `node --test workers/api/public/admin/`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as Z from './zones.js';
import { useTranslator } from '../lib/ui/core.js';
import { Element, render, injected, XSS } from '../lib/ui/dom-shim.mjs';
import { register } from 'node:module';

// THE BROWSER SEAM, in node: open() with no arguments imports the console's
// real /admin/core.js, /admin/i18n.js and /admin/app.js by absolute path.
// These hooks answer those three paths with modules reading globalThis.__zfake,
// so the default wiring runs here too (it is the path production takes).
const FAKES = {
  '/admin/core.js': 'export const { S, store, t, sheet, $, toast, closeSheet, busy, post } = globalThis.__zfake;',
  '/admin/i18n.js': 'export const T = { sq: {}, en: {}, uk: {} }; export const LANGS = ["sq", "en", "uk"];',
  '/admin/app.js': 'export const loadVenue = async () => { globalThis.__zfake.reloaded++; };',
};
register('data:text/javascript,' + encodeURIComponent(`const F = ${JSON.stringify(FAKES)};
  export async function resolve(spec, ctx, next){
    return F[spec] ? { url: 'data:text/javascript,' + encodeURIComponent(F[spec]), shortCircuit: true } : next(spec, ctx);
  }`));

useTranslator(k => `<${k}>`);
// The shim has no `.value`; an input's value is its attribute, as on first paint.
Object.defineProperty(Element.prototype, 'value', {
  get(){ return this.getAttribute('value') ?? ''; }, set(v){ this.setAttribute('value', v); }, configurable: true });

const DUBIN = { lat: 41.3231, lng: 19.4412 };
const CIRCLE = { kind: 'circle', lat: 41323100, lon: 19441200, radius_m: 3000 };
const POLY = { kind: 'polygon', points: [[1, 1], [1, 2], [2, 2]] };

test('circleZone: degrees and km become integer micro-degrees and metres, kind first', () => {
  const { zone } = Z.circleZone({ lat: '41.3231', lng: '19,4412', km: '3' });
  assert.deepEqual(zone, CIRCLE);
  assert.equal(Object.keys(zone)[0], 'kind');
  // The hub's reader splits on this exact prefix.
  assert.ok(JSON.stringify({ zones: [zone] }).includes('{"kind":"circle"'));
});

test('circleZone refuses a centre off the globe or missing, and a radius out of range', () => {
  for (const r of [{ lat: '', lng: '19', km: '3' }, { lng: '19', km: '3' }, { lat: '91', lng: '19', km: '3' }, { lat: '41', lng: '-181', km: '3' }, { lat: 'x', lng: '1', km: '1' }])
    assert.deepEqual(Z.circleZone(r), { err: 'zBadCentre' });
  for (const km of ['0', '', '-2', '50.5', '0.0001', 'abc'])
    assert.deepEqual(Z.circleZone({ lat: '41', lng: '19', km }), { err: 'zBadKm' });
  assert.equal(Z.circleZone({ lat: '-90', lng: '180', km: String(Z.MAX_KM) }).zone.radius_m, 50000);
});

test('rowsOf reads circles back as degrees and km, keeps polygons whole, drops the rest', () => {
  assert.deepEqual(Z.rowsOf([CIRCLE, POLY, { kind: 'blob' }, null]),
    [{ kind: 'circle', lat: '41.3231', lng: '19.4412', km: '3' }, { kind: 'polygon', zone: POLY }]);
  assert.deepEqual(Z.rowsOf(undefined), []);
});

test('bodyOf: round trip is the stored list; the first bad row refuses the lot', () => {
  assert.deepEqual(Z.bodyOf(Z.rowsOf([CIRCLE, POLY])), { body: { zones: [CIRCLE, POLY] } });
  assert.deepEqual(Z.bodyOf([]), { body: { zones: [] } });
  assert.deepEqual(Z.bodyOf([{ kind: 'circle', lat: '41', lng: '19', km: '99' }]), { err: 'zBadKm' });
});

test('the body carries nothing but zones; the venue rides in the query', () => {
  assert.deepEqual(Object.keys(Z.bodyOf([]).body), ['zones']);
  assert.equal(Z.zonesPath('dubin & sushi'), '/owner/zones?location_id=dubin%20%26%20sushi');
  assert.equal(Z.zonesPath(null), '/owner/zones?location_id=');
});

test('centreOf and newRow: the venue position when it has one, blank when not', () => {
  assert.deepEqual(Z.centreOf(DUBIN), DUBIN);
  for (const v of [null, {}, { lat: 41 }, { lat: 'x', lng: 1 }]) assert.equal(Z.centreOf(v), null);
  assert.deepEqual(Z.newRow(DUBIN), { kind: 'circle', lat: '41.3231', lng: '19.4412', km: String(Z.DEFAULT_KM) });
  assert.deepEqual(Z.newRow(null), { kind: 'circle', lat: '', lng: '', km: String(Z.DEFAULT_KM) });
});

test('install merges the three languages into the console table', () => {
  const T = { sq: { save: 'Ruaj' }, en: {}, uk: {} };
  Z.install(T, ['sq', 'en', 'uk']);
  assert.equal(T.sq.save, 'Ruaj');
  for (const l of ['sq', 'en', 'uk']) assert.deepEqual(Object.keys(T[l]).filter(k => k !== 'save').sort(), Object.keys(Z.WORDS.en).sort());
});

test('render: empty state and no clear button when there is no area; rows and clear when there is', () => {
  const none = render(Z.render([])).root;
  assert.ok(none.querySelector('.ui-empty, [class*="ui-empty"]'));
  assert.equal(none.querySelector('#zClear'), null);
  assert.ok(none.querySelector('#zAdd') && none.querySelector('#zSave'));
  const some = render(Z.render(Z.rowsOf([CIRCLE, POLY]))).root;
  assert.equal(some.querySelectorAll('.zone-row').length, 2);
  assert.ok(some.querySelector('#zClear'));
  assert.equal(some.querySelector('[data-f="km"]').value, '3');
  assert.match(some.querySelectorAll('.zone-row')[1].textContent, /3 /);
  assert.deepEqual(injected(Z.render([{ kind: 'circle', lat: XSS, lng: XSS, km: XSS }])), []);
  // A polygon with no points list still draws (0 points) rather than throwing.
  assert.match(render(Z.render([{ kind: 'polygon', zone: { kind: 'polygon' } }])).root.textContent, /0 /);
});

test('readRows re-reads what was typed, polygons as held', () => {
  const rows = Z.rowsOf([CIRCLE, POLY]);
  const { root } = render(Z.render(rows));
  root.querySelector('[data-f="km"]').value = '5';
  assert.deepEqual(Z.readRows(root, rows), [{ kind: 'circle', lat: '41.3231', lng: '19.4412', km: '5' }, { kind: 'polygon', zone: POLY }]);
});

// A fake console: the sheet renders into a shim document; every post and toast is recorded.
function fakeCore(venue, { postFails = false } = {}){
  let root = null;
  const c = { posts: [], toasts: [], closed: 0, sheets: 0,
    S: { venue }, store: { loc: 'dubin' }, t: k => 'T:' + k,
    sheet(html){ c.sheets++; root = render(html).root; },
    $: s => root.querySelector(s), get root(){ return root; },
    toast: m => c.toasts.push(m), closeSheet: () => { c.closed++; },
    busy: async (_el, fn) => fn(),
    post: async (path, body) => { if (postFails) throw new Error('HTTP 400'); c.posts.push([path, body]); return { ok: true }; } };
  return c;
}
const deps = (c, reloads) => ({ core: c, i18n: { T: { sq: {}, en: {}, uk: {} }, LANGS: ['sq', 'en', 'uk'] }, reload: async () => { reloads.n++; } });

test('open: a venue with no area adds a circle at its own position and saves it', async () => {
  const c = fakeCore({ ...DUBIN }), reloads = { n: 0 };
  await Z.open(deps(c, reloads));
  assert.equal(c.root.querySelectorAll('.zone-row').length, 0);
  c.$('#zAdd').onclick();
  assert.equal(c.root.querySelectorAll('.zone-row').length, 1);
  await c.$('#zSave').onclick();
  assert.deepEqual(c.posts, [['/owner/zones?location_id=dubin', { zones: [CIRCLE] }]]);
  assert.deepEqual(c.toasts, ['T:saved']);
  assert.equal(reloads.n, 1);
  assert.equal(c.closed, 1);
});

test('open: clear posts the empty list, which is how the area is removed', async () => {
  const c = fakeCore({ ...DUBIN, deliveryZones: [CIRCLE] }), reloads = { n: 0 };
  await Z.open(deps(c, reloads));
  await c.$('#zClear').onclick();
  assert.deepEqual(c.posts, [['/owner/zones?location_id=dubin', { zones: [] }]]);
});

test('open: removing a row keeps the others; a bad radius is refused before any request', async () => {
  const c = fakeCore({ deliveryZones: [CIRCLE, POLY] }), reloads = { n: 0 };
  await Z.open(deps(c, reloads));
  c.root.querySelector('[data-zdel="1"]').onclick();
  assert.equal(c.root.querySelectorAll('.zone-row').length, 1);
  c.$('[data-f="km"]').value = '400';
  await c.$('#zSave').onclick();
  assert.deepEqual(c.posts, []);
  assert.deepEqual(c.toasts, ['T:zBadKm']);
});

test('open: the hub refusing is shown in its own words and the sheet stays open', async () => {
  const c = fakeCore({ deliveryZones: [CIRCLE] }, { postFails: true }), reloads = { n: 0 };
  await Z.open(deps(c, reloads));
  await c.$('#zSave').onclick();
  assert.deepEqual(c.toasts, ['HTTP 400']);
  assert.equal(c.closed, 0);
  assert.equal(reloads.n, 0);
});

test('open: a console with no venue loaded yet opens on the empty state', async () => {
  const c = fakeCore(undefined), reloads = { n: 0 };
  await Z.open(deps(c, reloads));
  assert.equal(c.$('#zClear'), null);
});

test('open: a refusal that is not an Error is still shown', async () => {
  const c = fakeCore({ deliveryZones: [CIRCLE] }), reloads = { n: 0 };
  c.post = async () => { throw 'offline'; };
  await Z.open(deps(c, reloads));
  await c.$('#zSave').onclick();
  assert.deepEqual(c.toasts, ['offline']);
});

test('open() with no arguments wires the real console modules and reloads the venue after a save', async () => {
  const c = fakeCore({ deliveryZones: [CIRCLE] });
  c.reloaded = 0;
  globalThis.__zfake = c;
  await Z.open();
  await c.$('#zSave').onclick();
  assert.deepEqual(c.posts, [['/owner/zones?location_id=dubin', { zones: [CIRCLE] }]]);
  assert.equal(c.reloaded, 1);
  assert.equal(c.closed, 1);
});
