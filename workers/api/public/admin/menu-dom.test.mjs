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
    let onClose = null; export const whenSheetCloses = fn => { onClose = fn; }; export const closeSheetFor = () => { const f = onClose; onClose = null; f?.(); };
    export const switchEl = (id, on, key, hintKey) => check({ id, checked: on, key, hintKey });`,
  '/admin/i18n.js': `export const T = { sq: {}, en: {}, uk: {}, ru: {} }; export const lang = 'en'; export const LANGS = ['sq', 'en', 'uk', 'ru']; export const st = k => k;`,
  '/admin/app.js': `const F = () => globalThis.__md; export const loadVenue = async () => { F().loads++; }, rerender = async () => { F().rerenders++; }, me = () => ({}), show = async () => {};`,
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
// The shim has no insertAdjacentHTML and no <head>: the list's badges and stylesheets need both.
Element.prototype.insertAdjacentHTML = function(where, html){ const tmp = this.ownerDocument.createElement('div'); tmp.innerHTML = html; const kids = [...tmp.childNodes];
  for (const k of kids) { if (where === 'afterbegin' && this.childNodes.length) { k.parentNode = this; this.childNodes.unshift(k); } else this.appendChild(k); } };

const SUPPLIES = [{ id: 'tuna', name: 'Tuna', unit: 'g', kind: 'food_ingredient' }, { id: 'rice', name: 'Rice', unit: 'g', kind: 'food_ingredient' }];
// Declared "none of the 14" (W-MR0): an UNDECLARED dish on sale is refused by the hub where the
// allergen filter is on, and now by the console before the round trip (tests at the end).
const dish = (id, name, allergens = []) => ({ id, name, price: 1000, categoryId: 'c1', categoryName: 'Rolls', available: true, translations: {}, ...(allergens ? { allergens } : {}) });

/// A console whose hub answers `routes[METHOD path-prefix]`; every call is kept.
function page(routes){
  const doc = new Document();
  globalThis.document = doc;
  doc.head = { append(){} };
  const box = doc.createElement('div'); box.setAttribute('id', 'sheetIn'); doc.body.appendChild(box);
  const f = { S: { products: [dish('d1', 'Tuna roll'), dish('d2', 'Rice roll')], venue: { defaultLocale: 'en' }, categories: [] }, toast: [], calls: [], closed: 0, loads: 0, rerenders: 0,
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

// TZ 2026-10-16 ("enter everything as is, no ingredient lost"): a line typed down to 0 is no
// longer dropped by the save with "Saved". The save stops, names the line, and the x takes it out.
test('a qty typed as 0 stops the save and names the line; the x takes it out', async () => {
  const { box, f } = page(BASE);
  await M.openDish('d1'); await tick();
  const qty = box.querySelector('[data-rq="1"]');
  const save = box.querySelector('#dSave');
  qty.value = '0'; qty.oninput?.(); qty.onchange?.();
  assert.ok(box.querySelectorAll('button').includes(save), 'typing did not redraw under the finger');
  await save.onclick(); await tick();
  assert.equal(saved(f, 'd1').length, 0, 'nothing was sent: the line would have been lost');
  assert.ok(f.toast.some(m => /rc_lineBad|Not saved|quantity/i.test(String(m))), 'the owner is told which line');
  assert.ok(qty.closest('.rc-line')?.classList.contains('lost'), 'and the line is outlined');
  box.querySelector('[data-rx="1"]').onclick(); await tick();
  await box.querySelector('#dSave').onclick(); await tick();
  assert.deepEqual(bomOf(saved(f, 'd1')[0]), [['tuna', 80]], 'taken out with x, the save goes');
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

// ── W-TKUI TZ 4 (operator 2026-10-12): the recipe picker's ticks survive the search box ─────────
const BOOK4 = [...SUPPLIES, { id: 'nori', name: 'Nori', unit: 'g', kind: 'food_ingredient' }, { id: 'salmon', name: 'Salmon', unit: 'g', kind: 'food_ingredient' }];
const tickBox = (box, id) => { const cb = box.querySelector(`[data-rs="${id}"]`); assert.ok(cb, `no checkbox for ${id}`); cb.checked = true; cb.onchange?.(); return cb; };
const typeSearch = (box, v) => { const q = box.querySelector('#rcQ'); q.value = v; q.oninput?.(); };
// menu.js keeps the shelf answer until a write lands (W-TZFRESH `fresh.cache`): a bump makes this page's book be read.
const { fresh } = await import('/admin/fresh-logic.js');
const freshBook = () => { fresh.bump('/test'); };

test('TZ4: a component ticked, a search typed and cleared: the tick is kept, and Add takes it', async () => {
  freshBook(); const { box } = page({ ...BASE, 'GET /owner/stock': { supplies: BOOK4, noRecipe: [] } });
  await M.openDish('d1'); await tick();
  tickBox(box, 'nori');
  typeSearch(box, 'sal'); typeSearch(box, '');
  assert.ok(box.querySelector('[data-rs="nori"]')?.checked, 'clearing the search box reset the tick on nori (TZ 4)');
  box.querySelector('#rcAdd').onclick();
  assert.deepEqual(box.querySelectorAll('.rc-line b').map(b => b.textContent), ['Tuna', 'Rice', 'Nori']);
});

test('TZ4 twin: a tick hidden by the search is still added, and the kind chips keep ticks too; an untick is honoured', async () => {
  freshBook(); const { box } = page({ ...BASE, 'GET /owner/stock': { supplies: BOOK4, noRecipe: [] } });
  await M.openDish('d1'); await tick();
  tickBox(box, 'nori');
  typeSearch(box, 'sal');
  assert.equal(box.querySelector('[data-rs="nori"]'), null, 'the search hides nori');
  tickBox(box, 'salmon');
  const chip = box.querySelectorAll('[data-rk]').find(b => b.dataset.rk === 'food_ingredient'); chip.onclick();
  const s = box.querySelector('[data-rs="salmon"]'); assert.ok(s?.checked, 'the kind chip kept the tick');
  s.checked = false; s.onchange?.();
  box.querySelector('#rcAdd').onclick();
  assert.deepEqual(box.querySelectorAll('.rc-line b').map(b => b.textContent), ['Tuna', 'Rice', 'Nori'], 'nori (hidden, ticked) added; salmon (unticked) not');
  assert.equal(box.querySelectorAll('[data-rs][checked]').length, 0, 'after Add the picker starts clean');
});

// ── W-TKUI TZ 2: taste, texture and aroma under ONE collapsed accordion, remembered per device ──
const SX_KEY = 'dw_fold_sxFold';
const fakeStorage = (init = {}) => { const m = new Map(Object.entries(init)); return { getItem: k => (m.has(k) ? m.get(k) : null), setItem: (k, v) => m.set(k, String(v)), removeItem: k => m.delete(k), m }; };

test('TZ2: the three sense blocks sit inside one <details> that opens closed, with a keyboard-reachable summary', async () => {
  globalThis.localStorage = fakeStorage();
  const { box } = page(BASE);
  await M.openDish('d1'); await tick();
  const fold = box.querySelector('details#sxFold');
  assert.ok(fold, 'no single accordion around taste/texture/aroma (TZ 2)');
  assert.equal(fold.hasAttribute('open'), false, 'collapsed by default: the form is shorter');
  const sum = fold.querySelector('summary');
  assert.ok(sum && /data-t="sxEdit"/.test(sum.innerHTML), 'the summary carries the heading');
  for (const id of ['sxBox', 'sxSuggest', 'sxNote', 'sxRecipe']) assert.ok(fold.querySelector('#' + id), `${id} is inside the accordion`);
  assert.equal(box.querySelectorAll('[data-t="sx_taste"], [data-t="sx_texture"], [data-t="sx_aroma"]').every(e => fold.querySelectorAll('[data-t]').includes(e)), true);
});

test('TZ2 twin: a device that opened it last time sees it open; a toggle is remembered; a broken storage is survived', async () => {
  globalThis.localStorage = fakeStorage({ [SX_KEY]: 'open' });
  let { box } = page(BASE);
  await M.openDish('d1'); await tick();
  const fold = box.querySelector('details#sxFold');
  assert.equal(fold.hasAttribute('open'), true, 'remembered open');
  fold.removeAttribute('open'); fold.dispatchEvent({ type: 'toggle', target: fold });
  assert.equal(globalThis.localStorage.getItem(SX_KEY), 'closed', 'the toggle is written');
  globalThis.localStorage = { getItem(){ throw new Error('blocked'); }, setItem(){ throw new Error('blocked'); } };
  ({ box } = page(BASE));
  await M.openDish('d1'); await tick();
  assert.ok(box.querySelector('details#sxFold'), 'drawn without storage');
});

// ── W-TKUI TZ 6: after a save the console re-reads ONE answer and redraws, without F5 ─────────
globalThis.location = { hostname: 'qa-durres.dowiz.org' };
const MENU_AFTER = { location: { id: 'V1' }, categories: [{ id: 'c1', name: 'Rolls', products: [{ id: 'd1', name: 'Tuna roll XL', description: 'big', price: 1200, available: true }, { id: 'd2', name: 'Rice roll', price: 1000, available: true }] }] };
const menuGets = f => f.calls.filter(c => c[0] === 'GET' && /\/public\/locations\/.*\/menu/.test(c[1])).map(c => c[1]);

test('TZ6: a save re-reads the menu ONCE, in the console language, folds the dish into the list and redraws it', async () => {
  const { box, f } = page({ ...BASE, 'GET /public/locations/qa-durres/menu': MENU_AFTER });
  f.S.categories = [{ id: 'c1', name: 'Rolls', products: f.S.products }];
  await M.openDish('d1'); await tick();
  box.querySelector('#d-price').value = '1200'; box.querySelector('#d-name-uk').value = 'Тунець XL';
  await box.querySelector('#dSave').onclick(); await tick();
  assert.equal(saved(f, 'd1').length, 1);
  assert.deepEqual(menuGets(f), ['/public/locations/qa-durres/menu?locale=en&fresh=1'], 'one read, the console language, fresh (not four languages)');
  assert.equal(f.loads, 0, 'no whole-venue reload');
  const row = f.S.products.find(p => p.id === 'd1');
  assert.equal(row.price, 1200); assert.equal(row.name, 'Tuna roll XL'); assert.equal(row.description, 'big');
  assert.equal(row.translations.uk.name, 'Тунець XL', 'the translation typed is kept on the row without a re-read');
  assert.equal(f.S.categories[0].products.find(p => p.id === 'd1').price, 1200, 'the category list the screen draws from is the same row');
  assert.ok(f.rerenders >= 1, 'the screen was redrawn');
});

test('TZ6 twin: a dish the fresh menu no longer carries falls back to the whole-venue reload; a failed read too', async () => {
  const { box, f } = page({ ...BASE, 'GET /public/locations/qa-durres/menu': { location: {}, categories: [] } });
  await M.openDish('d1'); await tick();
  await box.querySelector('#dSave').onclick(); await tick();
  assert.equal(f.loads, 1, 'the dish vanished from the answer: the whole venue is reloaded');
  const p2 = page({ ...BASE, 'GET /public/locations/qa-durres/menu': new Error('HTTP 503') });
  await M.openDish('d1'); await tick();
  await p2.box.querySelector('#dSave').onclick(); await tick();
  assert.equal(p2.f.loads, 1, 'a failed read: the whole venue is reloaded, never a stale row');
});

test('TZ6 twin: a nudge keeps the sheet open, re-reads once, and the list behind is redrawn when the sheet closes', async () => {
  const { box, f } = page({ ...BASE, 'GET /public/locations/qa-durres/menu': MENU_AFTER });
  await M.openDish('d1'); await tick();
  const before = f.rerenders;
  await box.querySelector('#dUp').onclick(); await tick();
  assert.deepEqual(saved(f, 'd1')[0], { location_id: 'V1', move: 'up' });
  assert.equal(menuGets(f).length, 1, 'one read');
  assert.equal(f.loads, 0);
  const core = await import('/admin/core.js');
  core.closeSheetFor();
  assert.ok(f.rerenders > before, 'the screen behind the sheet is redrawn on close');
});

// ── W-TKUI TZ 1 (CONTRACT-QTY section 1): a recipe line takes a decimal, exactly ───────────────
test('TZ1: "0,5" typed on a line is sent as "0.5"; a stored 40.5 draws as 40.5 and goes back as it came; a stepper moves whole grams', async () => {
  freshBook();
  const { box, f } = page({ ...BASE, 'GET /owner/stock': { supplies: BOOK4, noRecipe: [] },
    'GET /owner/products?id=d1': { products: [{ id: 'd1', bom: [{ supply: 'nori', qty: 0.5, qtyMilli: 500 }, { supply: 'rice', qty: 120, qtyMilli: 120000 }] }] } });
  await M.openDish('d1'); await tick();
  const q0 = box.querySelector('[data-rq="0"]');
  assert.equal(q0.value, '0.5', 'a fractional stored line is drawn as its decimal, not rounded to 1');
  assert.equal(q0.getAttribute('inputmode'), 'decimal', 'the keyboard offers the decimal point');
  const q1 = box.querySelector('[data-rq="1"]');
  q1.value = '0,25'; q1.oninput?.();
  box.querySelector('[data-rm="0"]').onclick(); // - 10 g on 0.5 g floors at 0 -> the line is out
  await box.querySelector('#dSave').onclick(); await tick();
  assert.deepEqual(bomOf(saved(f, 'd1')[0]), [['rice', '0.25']], 'the fraction travels as text, the way the hub parses it');
});

test('TZ1 twin: a whole quantity still goes as the integer it always was; a 4th decimal is refused before the hub', async () => {
  freshBook();
  const { box, f } = page({ ...BASE, 'GET /owner/stock': { supplies: BOOK4, noRecipe: [] } });
  await M.openDish('d1'); await tick();
  const q0 = box.querySelector('[data-rq="0"]');
  q0.value = '70'; q0.oninput?.();
  q0.value = '0.0005'; q0.oninput?.(); q0.onchange?.();
  assert.ok(f.toast.length >= 1, 'a 4th decimal is said no to');
  // TZ 2026-10-16: the unreadable entry stops the save instead of quietly sending the old 70.
  await box.querySelector('#dSave').onclick(); await tick();
  assert.equal(saved(f, 'd1').length, 0, 'an unreadable quantity is not saved as something else');
  q0.value = '70'; q0.oninput?.();
  await box.querySelector('#dSave').onclick(); await tick();
  assert.deepEqual(bomOf(saved(f, 'd1')[0]), [['tuna', 70], ['rice', 120]]);
});

// ── W-TKUI TZ 3 (CONTRACT-QTY section 2): the review badge, on the sheet and on the list ─────────
const REVIEWED = { products: [{ id: 'd1', recipeReview: [{ supply: 'nori', name: 'Nori' }],
  bom: [{ supply: 'tuna', qty: 80, qtyMilli: 80000 }, { supply: 'nori', qty: 0, qtyMilli: 0, name: 'Nori', removed: true }] }] };
test('TZ3: a dish with a ghost line shows the badge naming the removed ingredient; the ghost is drawn greyed and never sent back', async () => {
  freshBook();
  const { box, f } = page({ ...BASE, 'GET /owner/products?id=d1': REVIEWED });
  await M.openDish('d1'); await tick();
  const badge = box.querySelector('#rcReview');
  assert.ok(badge, 'no review badge on the dish sheet (TZ 3)');
  assert.match(badge.textContent, /Nori/, 'the badge names the removed ingredient');
  const ghost = box.querySelectorAll('.rc-line').find(l => /Nori/.test(l.innerHTML));
  assert.ok(ghost && ghost.classList.contains('off'), 'the ghost line is greyed');
  assert.ok(ghost.querySelector('[data-rx]'), 'and can be taken out');
  await box.querySelector('#dSave').onclick(); await tick();
  assert.deepEqual(bomOf(saved(f, 'd1')[0]), [['tuna', 80]], 'a ghost line is never sent: the hub would refuse it, and dropping it clears the badge');
});

test('TZ3 twin: the menu list badges the dish from the owner read, once per render; a clean dish has none', async () => {
  freshBook();
  const { box, f } = page({ ...BASE, 'GET /owner/products': { products: [{ id: 'd1', recipeReview: [{ supply: 'nori', name: 'Nori' }], bom: [] }, { id: 'd2', bom: [] }] } });
  f.S.categories = [{ id: 'c1', name: 'Rolls', products: f.S.products }];
  const host = box.ownerDocument.createElement('div'); box.ownerDocument.body.appendChild(host);
  await M.render(host); await tick();
  assert.ok(host.querySelector('[data-p="d1"] .review'), 'the row of the flagged dish carries the badge (TZ 3)');
  assert.equal(host.querySelector('[data-p="d2"] .review'), null);
  assert.equal(f.calls.filter(c => c[1] === '/owner/products').length, 1, 'one owner read for the whole list');
});

// ── W-TKUI sweep (c): the dish sheet's optional groups fold ─────────────────────────────────────
test('sweep c: translations and nutrition sit under remembered folds; price, name, stop-list stay in the open', async () => {
  globalThis.localStorage = fakeStorage();
  const { box } = page(BASE);
  await M.openDish('d1'); await tick();
  for (const id of ['dTrFold', 'dNutFold']) { const d = box.querySelector(`details#${id}`); assert.ok(d, `${id} is a fold`); assert.equal(d.hasAttribute('open'), false); }
  assert.ok(box.querySelector('details#dTrFold #d-name-uk'), 'the other languages are inside the translations fold');
  assert.ok(box.querySelector('details#dNutFold #d-kcal'), 'kcal is inside the nutrition fold');
  for (const id of ['d-name', 'd-price', 'd-avail', 'd-cat']) assert.equal(box.querySelector('#' + id).closest('details'), null, `${id} is a primary field, never folded`);
});
