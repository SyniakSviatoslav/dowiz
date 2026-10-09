// The console's venue load in node (W-LOOPC, R-LOOPS row 8): the dishes'
// translations come out EXACTLY as the code they replaced made them.
// `node --test workers/api/public/admin/app-venue.test.mjs`
//
// THE ORACLE is the old loop of `loadVenue` (base 6ccca6ef, admin/app.js
// 316-322) copied verbatim: `S.products.find` inside languages x categories x
// dishes. The new code builds one Map id -> dish. The test drives the REAL
// `loadVenue` of the real admin/app.js, with the hub answering four menus.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';

// THE BROWSER SEAM (as kitchen-dom.test.mjs): core and i18n are fakes reading
// globalThis.__av; every other /admin/, /lib/ or /store/ path is the real file.
const PUBLIC = new URL('../', import.meta.url).href;
const FAKES = {
  '/admin/core.js': `const F = () => globalThis.__av;
    import * as ui from '/lib/ui/index.js';
    const stub = () => ({ hidden: false, disabled: false, innerHTML: '', value: '', dataset: {}, focus(){}, addEventListener(){} });
    export const esc = ui.esc, lang = 'sq', LANGS = ['sq', 'en', 'uk', 'ru'], store = { t: '', loc: 'V1' };
    export const $ = () => stub(), $$ = () => [];
    export const icon = n => '<i class="ti ti-' + n + '"></i>', t = k => k;
    export const S = new Proxy({}, { get: (_, k) => F().S[k], set: (_, k, v) => { F().S[k] = v; return true; } });
    export const api = async path => F().reply(path);
    export const post = async () => ({}), withLoc = (b = {}) => b;
    export const setLang = () => {}, retranslate = () => {}, logout = () => {}, whenLoggedOut = () => {}, bindSheetChrome = () => {}, hydrate = () => {};
    export const toast = () => {}, sheet = () => {}, closeSheet = () => {}, confirm = async () => ({ ok: true }), switchEl = () => '';
    export const setCurrency = async (base, shown) => { F().currency = [base, shown]; };
    export const displayCurrency = () => 'ALL', baseCurrency = () => 'ALL', CURRENCIES = ['ALL', 'EUR'];
    export const POLL_MS = 15000, POLL_IDLE_MS = 60000;`,
  '/admin/i18n.js': `export const T = { sq: {}, en: {}, uk: {}, ru: {} }; export const LANGS = ['sq', 'en', 'uk', 'ru'];`,
};
register('data:text/javascript,' + encodeURIComponent(`const F = ${JSON.stringify(FAKES)}, P = ${JSON.stringify(PUBLIC)};
  export async function resolve(spec, ctx, next){
    if (F[spec]) return { url: 'data:text/javascript,' + encodeURIComponent(F[spec]), shortCircuit: true };
    if (/^\\/(admin|lib|store)\\//.test(spec)) return { url: P + spec.slice(1), shortCircuit: true };
    return next(spec, ctx);
  }`));

// The page around the shell: what admin/app.js touches at load (the theme, the
// stored choices, the host the venue slug is read from). Signed out: boot()
// draws the login and asks nothing.
globalThis.document = { documentElement: { lang: '', setAttribute(){}, removeAttribute(){} }, hidden: false,
  querySelectorAll: () => [], querySelector: () => null, addEventListener(){} };
globalThis.localStorage = { getItem: () => null, setItem(){}, removeItem(){} };
globalThis.location = { hostname: 'qa-durres.dowiz.org' };
globalThis.__av = { S: {}, reply: async () => { throw new Error('no hub yet'); } };
const A = await import('./app.js');

// ── the oracle: base admin/app.js:310-323, verbatim but for `api`/`S` -> arguments ──
const OTHERS = ['en', 'uk', 'ru'];
function oracleLoad(d, rest){
  const S = {};
  S.products = (d.categories || []).flatMap(c => (c.products || []).map(p => ({ ...p, categoryId: c.id, categoryName: c.name, translations: {} })));
  const others = OTHERS;
  for (const [i, r] of rest.entries()) {
    if (!r) continue;
    for (const c of r.categories || []) for (const p of c.products || []) {
      const mine = S.products.find(x => x.id === p.id); if (mine) mine.translations[others[i]] = { name: p.name, description: p.description || '' };
    }
  }
  for (const p of S.products) p.translations.sq = { name: p.name, description: p.description || '' };
  return S.products;
}

/// One venue's menu in one language: `dishes` dishes over `cats` categories.
function menu(l, dishes, cats = 12, { drop = new Set(), extra = [], twice = [] } = {}){
  const per = Math.ceil(dishes / cats);
  const categories = Array.from({ length: cats }, (_, c) => ({ id: `c_${c}`, name: `Cat ${c} ${l}`,
    products: Array.from({ length: per }, (_, k) => c * per + k).filter(n => n < dishes && !drop.has(n))
      .map(n => ({ id: `p_${n}`, name: `Roll ${n} ${l}`, price: 1000 + n, ...(n % 4 ? { description: `Desc ${n} ${l}` } : {}) })) }));
  categories[0].products.push(...extra.map(n => ({ id: `p_${n}`, name: `Extra ${n} ${l}`, description: `X ${l}` })));
  // A dish listed in two categories: the console holds it twice, `find` named the FIRST.
  for (const n of twice) categories[cats - 1].products.push({ id: `p_${n}`, name: `Roll ${n} ${l} again`, price: 1 });
  return { location: { id: 'V1', currencyCode: 'ALL' }, categories };
}
/// The hub: `/public/locations/<slug>/menu?locale=<l>` answers `menus[l]`; null = that read fails.
function hub(menus){
  const seen = [];
  globalThis.__av.reply = async path => {
    seen.push(path);
    const l = /locale=(\w+)/.exec(path)[1];
    if (!menus[l]) throw new Error(`503 ${l}`);
    return structuredClone(menus[l]);
  };
  return seen;
}
async function check(menus, why){
  globalThis.__av.S = {};
  const seen = hub(menus);
  await A.loadVenue();
  const want = oracleLoad(structuredClone(menus.sq), OTHERS.map(l => menus[l] ? structuredClone(menus[l]) : null));
  assert.deepEqual(globalThis.__av.S.products, want, why);
  assert.equal(JSON.stringify(globalThis.__av.S.products), JSON.stringify(want), `${why}: key order too`);
  assert.deepEqual(seen.map(p => /locale=(\w+)/.exec(p)[1]), ['sq', 'en', 'uk', 'ru'], `${why}: reads`);
  assert.ok(seen.every(p => p.startsWith('/public/locations/qa-durres/menu?')), `${why}: the host's slug`);
  return globalThis.__av.S.products;
}

test('165 dishes in four languages: every dish carries the oracle\'s translations, in the oracle\'s order', async () => {
  const menus = Object.fromEntries(['sq', 'en', 'uk', 'ru'].map(l => [l, menu(l, 165)]));
  const P = await check(menus, '165');
  assert.equal(P.length, 165);
  assert.deepEqual(P[7].translations, { en: { name: 'Roll 7 en', description: 'Desc 7 en' }, uk: { name: 'Roll 7 uk', description: 'Desc 7 uk' },
    ru: { name: 'Roll 7 ru', description: 'Desc 7 ru' }, sq: { name: 'Roll 7 sq', description: 'Desc 7 sq' } });
  assert.equal(P[8].translations.en.description, '', 'no description reads as empty');
  assert.deepEqual(globalThis.__av.currency, ['ALL', 'ALL']);
});

test('gaps: a dish missing in one language, a dish only another language has, a failed read', async () => {
  await check({ sq: menu('sq', 60), en: menu('en', 60, 12, { drop: new Set([3, 17]), extra: [900] }), uk: null, ru: menu('ru', 60, 12, { drop: new Set([0]) }) }, 'gaps');
  const P = globalThis.__av.S.products;
  assert.deepEqual(Object.keys(P[3].translations).sort(), ['ru', 'sq']);
  assert.deepEqual(Object.keys(P[0].translations).sort(), ['en', 'sq']);
  assert.ok(!P.some(p => p.id === 'p_900'), 'a dish this language does not list is not invented');
});

test('a dish listed in two categories: the FIRST copy takes the words, as `find` gave it', async () => {
  const twice = { twice: [2, 5] };
  const P = await check({ sq: menu('sq', 24, 4, twice), en: menu('en', 24, 4, twice), uk: menu('uk', 24, 4), ru: menu('ru', 24, 4, twice) }, 'twice');
  const copies = P.filter(p => p.id === 'p_2');
  assert.equal(copies.length, 2);
  assert.deepEqual(Object.keys(copies[0].translations).sort(), ['en', 'ru', 'sq', 'uk']);
  assert.deepEqual(Object.keys(copies[1].translations), ['sq'], 'the second copy only carries its own language');
  // The SECOND listing in another language wins on the first copy (the loop ran over all of them).
  assert.equal(copies[0].translations.en.name, 'Roll 2 en again');
});

test('addTranslations alone: an empty console, an empty menu', () => {
  const P = [];
  A.addTranslations(P, OTHERS, [menu('en', 5), null, { categories: [{ id: 'c', products: [] }, { id: 'd' }] }]);
  assert.deepEqual(P, []);
  const Q = [{ id: 'p_1', translations: {} }];
  A.addTranslations(Q, OTHERS, [{}, null, { categories: [{ id: 'c', products: [{ id: 'p_1', name: 'Р' }] }] }]);
  assert.deepEqual(Q, [{ id: 'p_1', translations: { ru: { name: 'Р', description: '' } } }]);
});
