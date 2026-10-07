// The dish sheet's RECIPE in node (W-FLOWS, 2026-09-30): what the owner changes
// is what the save sends. `node --test workers/api/public/admin/menu-dom.test.mjs`
//
// THE DEFECT, measured live on qa-durres before this test existed (an
// instrumented menu.js routed into Chromium): type 70 into the tuna line, tap
// the x on the rice line, Save -> "Saved", and the body carried
// bom [tuna 70, rice 120]. The qty box redrew every line on `change`, which a
// browser fires on BLUR -- i.e. on the pointer-down of the very tap that
// follows -- so the x the finger pressed was gone before its click landed.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
import { Document, Element } from '../lib/ui/dom-shim.mjs';

// THE BROWSER SEAM (as wire-dom.test.mjs): core, i18n and app are fakes
// reading globalThis.__md; every other /admin/, /lib/ or /store/ path is the real file
// (admin/sense.js imports /store/sense.js, served beside it: W-PITR2 found the test red on it).
const PUBLIC = new URL('../', import.meta.url).href;
const FAKES = {
  '/admin/core.js': `const F = () => globalThis.__md;
    import * as ui from '/lib/ui/index.js';
    import { check } from '/admin/parts.js';
    export const esc = ui.esc, store = { loc: 'V1', t: 'tok' }, LANGS = ['sq', 'en', 'uk', 'ru'];
    export const $ = (s, r = document) => r.querySelector(s), $$ = (s, r = document) => [...r.querySelectorAll(s)];
    export const icon = n => '<i class="ti ti-' + n + '"></i>';
    export const t = k => '<' + k + '>';
    export const S = new Proxy({}, { get: (_, k) => F().S[k] });
    export const toast = m => F().toast.push(String(m));
    export const api = (path, opts = {}) => F().reply(opts.method || 'GET', path, opts.body);
    export const post = (path, body) => F().reply('POST', path, body);
    export const withLoc = (b = {}) => ({ location_id: 'V1', ...b });
    export const busy = (el, fn) => fn();
    export const confirm = async () => ({ ok: true, reason: '' });
    export const sheet = html => { document.querySelector('#sheetIn').innerHTML = html; };
    export const closeSheet = () => { F().closed++; };
    export const money = n => String(n), moneyEl = n => '<span>' + n + '</span>';
    export const hydrate = () => {}, retranslate = () => {}, repaintMoney = () => {};
    export const switchEl = (id, on, key, hintKey) => check({ id, checked: on, key, hintKey });`,
  '/admin/i18n.js': `export const T = { sq: {}, en: {}, uk: {}, ru: {} }; export const lang = 'en'; export const LANGS = ['sq', 'en', 'uk', 'ru']; export const st = k => k;`,
  '/admin/app.js': `export const loadVenue = async () => {}, rerender = async () => {}, me = () => ({}), show = async () => {};`,
};
register('data:text/javascript,' + encodeURIComponent(`const F = ${JSON.stringify(FAKES)}, P = ${JSON.stringify(PUBLIC)};
  export async function resolve(spec, ctx, next){
    if (F[spec]) return { url: 'data:text/javascript,' + encodeURIComponent(F[spec]), shortCircuit: true };
    if (/^\\/(admin|lib|store)\\//.test(spec)) return { url: P + spec.slice(1), shortCircuit: true };
    return next(spec, ctx);
  }`));

Object.defineProperty(Element.prototype, 'value', {
  get(){ return this.getAttribute('value') ?? ''; }, set(v){ this.setAttribute('value', v); }, configurable: true });
Object.defineProperty(Element.prototype, 'checked', {
  get(){ return this.hasAttribute('checked'); }, set(v){ v ? this.setAttribute('checked', '') : this.removeAttribute('checked'); }, configurable: true });
const tick = async (n = 6) => { for (let i = 0; i < n; i++) await new Promise(r => setTimeout(r, 0)); };

const SUPPLIES = [{ id: 'tuna', name: 'Tuna', unit: 'g', kind: 'food_ingredient' }, { id: 'rice', name: 'Rice', unit: 'g', kind: 'food_ingredient' }];
// Declared "none of the 14" (W-MR0): an UNDECLARED dish on sale is refused by the hub where the
// allergen filter is on, and now by the console before the round trip (tests at the end).
const dish = (id, name, allergens = []) => ({ id, name, price: 1000, categoryId: 'c1', categoryName: 'Rolls', available: true, translations: {}, ...(allergens ? { allergens } : {}) });

/// A console whose hub answers `routes[METHOD path-prefix]`; every call is kept.
function page(routes){
  const doc = new Document();
  globalThis.document = doc;
  const box = doc.createElement('div'); box.setAttribute('id', 'sheetIn'); doc.body.appendChild(box);
  const f = { S: { products: [dish('d1', 'Tuna roll'), dish('d2', 'Rice roll')], venue: { defaultLocale: 'en' }, categories: [] }, toast: [], calls: [], closed: 0,
    reply: async (method, path, body) => {
      f.calls.push([method, path, body]);
      const key = Object.keys(routes).filter(k => `${method} ${path}`.startsWith(k)).sort((a, b) => b.length - a.length)[0];
      const a = key ? routes[key] : {};
      if (a instanceof Error) throw a;
      return typeof a === 'function' ? a(body, path) : a;
    } };
  globalThis.__md = f;
  return { box, f };
}
const BASE = { 'GET /owner/categories': { categories: [{ id: 'c1', name: 'Rolls' }] }, 'GET /owner/stock': { supplies: SUPPLIES, noRecipe: [] },
  'GET /owner/products?id=d1': { products: [{ id: 'd1', bom: [{ supply: 'tuna', qty: 80 }, { supply: 'rice', qty: 120 }] }] },
  'GET /owner/products?id=d2': { products: [{ id: 'd2', bom: [{ supply: 'rice', qty: 150 }] }] }, 'POST /owner/products/': {} };
const saved = (f, id) => f.calls.filter(c => c[0] === 'POST' && c[1] === `/owner/products/${id}`).map(c => c[2]);
const bomOf = b => b.bom?.map(l => [l.supply, l.qty]);
const M = await import('./menu.js');

test('a qty typed, then the x of ANOTHER line tapped: the x still lands, and the save carries both changes', async () => {
  const { box, f } = page(BASE);
  await M.openDish('d1'); await tick();
  const qty = box.querySelector('[data-rq="0"]');
  qty.value = '70'; qty.oninput?.();
  const x = box.querySelector('[data-rx="1"]');
  // The finger goes down on the x: the browser blurs the qty box first, which fires its `change`.
  qty.onchange?.();
  // (the shim's innerHTML leaves a replaced node's parentNode set, so "still in
  // the sheet" is asked of the sheet's own query, not of contains())
  assert.ok(box.querySelectorAll('[data-rx]').includes(x), 'the x the finger pressed was redrawn away by the qty box\'s blur (`change`) -- the tap is lost and the rice line is saved again');
  x.onclick();
  await box.querySelector('#dSave').onclick(); await tick();
  assert.deepEqual(bomOf(saved(f, 'd1')[0]), [['tuna', 70]], 'the save sends the removal AND the new quantity');
});

test('twin: a save that changed nothing sends the stored recipe as it was, and no name', async () => {
  const { box, f } = page(BASE);
  await M.openDish('d1'); await tick();
  await box.querySelector('#dSave').onclick(); await tick();
  const b = saved(f, 'd1')[0];
  assert.deepEqual(bomOf(b), [['tuna', 80], ['rice', 120]]);
  assert.equal('name' in b, false, 'an unchanged name is not sent');
});

test('a qty typed as 0 removes the line on save, without redrawing under the finger', async () => {
  const { box, f } = page(BASE);
  await M.openDish('d1'); await tick();
  const qty = box.querySelector('[data-rq="1"]');
  const save = box.querySelector('#dSave');
  qty.value = '0'; qty.oninput?.(); qty.onchange?.();
  assert.ok(box.querySelectorAll('button').includes(save));
  await save.onclick(); await tick();
  assert.deepEqual(bomOf(saved(f, 'd1')[0]), [['tuna', 80]]);
});

test('a recipe read that answers LATE, for a dish whose sheet is already gone, does not take over the next dish', async () => {
  let release; const late = new Promise(r => { release = r; });
  const { box, f } = page({ ...BASE, 'GET /owner/products?id=d1': () => late.then(() => BASE['GET /owner/products?id=d1']) });
  const first = M.openDish('d1'); await tick();
  await first;
  await M.openDish('d2'); await tick();
  release(); await tick();
  const names = box.querySelectorAll('.rc-line b').map(b => b.textContent);
  assert.deepEqual(names, ['Rice'], 'the Rice roll sheet shows ITS recipe, not the Tuna roll\'s');
  await box.querySelector('#dSave').onclick(); await tick();
  assert.deepEqual(bomOf(saved(f, 'd2')[0]), [['rice', 150]], 'and its save sends its own recipe');
});

test('a recipe that could not be read cannot be edited, and says so; the save sends no recipe', async () => {
  const { box, f } = page({ ...BASE, 'GET /owner/products?id=d1': new Error('HTTP 503') });
  await M.openDish('d1'); await tick();
  assert.equal(box.querySelectorAll('[data-rx]').length, 0, 'no x to tap on a recipe nobody read');
  assert.notEqual(typeof box.querySelector('#rcAdd')?.onclick, 'function', 'the add-component button works on a recipe nobody read -- and what it adds is dropped by the save with "Saved"');
  assert.ok(box.querySelector('#rcLines [data-t="recipeLoadFail"]'), 'the sheet says the recipe could not be read');
  await box.querySelector('#dSave').onclick(); await tick();
  assert.equal('bom' in saved(f, 'd1')[0], false);
});

// ── W-MR0: the menu's claims ────────────────────────────────────────────────
const undeclaredPage = (features) => {
  const p = page(BASE);
  p.f.S.products = [dish('d1', 'Tuna roll', null), dish('d2', 'Rice roll')];
  if (features) p.f.S.venue.features = features;
  return p;
};

test('the menu screen counts and lists the dishes nobody declared allergens for', async () => {
  const { f } = undeclaredPage();
  const MF = await import('/admin/menu-flags.js');
  const html = MF.undeclaredPanel(f.S.products);
  assert.match(html, /id="mfUndeclaredN">1</, 'one undeclared dish counted');
  assert.match(html, /data-p="d1"/, 'and listed, opening its sheet');
  assert.doesNotMatch(html, /data-p="d2"/, '"none of the 14" is a declaration');
  assert.match(MF.undeclaredPanel([dish('d2', 'Rice roll')]), /mf_allDeclared/);
});

test('an undeclared dish is not put on sale where the filter is on: the console refuses before the hub', async () => {
  const { box, f } = undeclaredPage();
  await M.openDish('d1'); await tick();
  await box.querySelector('#dSave').onclick(); await tick();
  assert.equal(saved(f, 'd1').length, 0, 'nothing sent');
  assert.deepEqual(f.toast, ['<mf_needDeclare>']);
});

test('"none of the 14" tapped: the save declares it and goes through', async () => {
  const { box, f } = undeclaredPage();
  await M.openDish('d1'); await tick();
  box.querySelector('#alNone').onclick();
  await box.querySelector('#dSave').onclick(); await tick();
  assert.deepEqual(saved(f, 'd1')[0].allergens, []);
});

test('chips tapped: the save carries exactly them; untapping all is NOT "none"', async () => {
  const { box, f } = undeclaredPage();
  await M.openDish('d1'); await tick();
  const chip = c => box.querySelector(`[data-al="${c}"]`);
  chip('fish').onclick(); chip('soy').onclick();
  await box.querySelector('#dSave').onclick(); await tick();
  assert.deepEqual(saved(f, 'd1')[0].allergens, ['fish', 'soy']);
  await M.openDish('d1'); await tick();
  chip('fish').onclick(); chip('fish').onclick();
  await box.querySelector('#dSave').onclick(); await tick();
  assert.equal(saved(f, 'd1').length, 1, 'emptied chips leave it undeclared, so the second save is refused');
});

test('with the storefront filter off the hub does not gate, and neither does the console; nothing is declared for the owner', async () => {
  const { box, f } = undeclaredPage({ allergen_filter: false });
  await M.openDish('d1'); await tick();
  await box.querySelector('#dSave').onclick(); await tick();
  assert.equal(saved(f, 'd1').length, 1);
  assert.equal('allergens' in saved(f, 'd1')[0], false, 'an untouched field sends no claim');
});

test("the owner's tag says what it is: Venue's pick, not Popular", async () => {
  const { box } = undeclaredPage();
  await import('/admin/menu-flags.js');
  await M.openDish('d2'); await tick();
  const pop = box.querySelector('[data-tag="popular"]');
  assert.ok(pop && /mtag_popular/.test(pop.innerHTML), pop?.innerHTML);
});

// ── W-MR0 MR8: the guest's taste on the customer card and the segment counts ──────────────────
test("the card shows the venue's taste profile, its segment and why, and never a price", async () => {
  const TA = await import('/admin/taste.js');
  const html = TA.tasteMarkup({ taste: { segment: 'regular', why: '3 orders, the last 2 days ago (at most 30)', orders: 3,
    tags: [{ key: 'salmon', w: 3000 }, { key: 'hot', w: 1000 }], cats: [{ key: 'rolls', w: 3000 }], device: { tags: {}, cats: {} } } });
  assert.match(html, /data-t="seg_regular"/);
  assert.match(TA.tasteMarkup({ taste: { segment: '<img onerror=x>', why: '', orders: 1 } }), /data-t="seg_new"/, 'an unknown segment never reaches the markup');
  assert.match(html, /3 orders, the last 2 days ago/);
  assert.match(html, /salmon, hot/);
  assert.match(html, /data-t="tasteFromDevice"/, 'what it was built from, including the phone summary');
  assert.match(html, /data-t="tasteNever"/);
  assert.doesNotMatch(html, /price|discount|lek/i);
  assert.match(TA.tasteMarkup({ taste: null }), /data-t="tasteNone"/, 'no consent: the card says so, nothing invented');
});

test('the segment counts draw all four, zero included, and fetch the venue route', async () => {
  const { box, f } = page({ 'GET /owner/customers/taste/segments': { segments: { new: 2, regular: 5, at_risk: 0, lapsed: 1 } } });
  box.innerHTML = '<div id="cuSegments"></div>';
  const TA = await import('/admin/taste.js');
  await TA.mountSegments();
  const html = box.innerHTML;
  for (const [k, n] of [['new', 2], ['regular', 5], ['at_risk', 0], ['lapsed', 1]]) assert.match(html, new RegExp(`data-t="seg_${k}"[^]*?</span> <b class="mono">${n}</b>`));
  assert.deepEqual(f.calls.map(c => c[1]), ['/owner/customers/taste/segments']);
});
