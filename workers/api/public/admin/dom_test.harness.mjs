// THE CONSOLE IN NODE, ONE HARNESS (lane W-TKUI, 2026-10-12): what every
// `*-dom.test.mjs` under /admin needs -- core, i18n and app as fakes reading
// `globalThis.__md`, every other /admin, /lib or /store path the real file,
// a document from `lib/ui/dom-shim.mjs` with the few things the shim lacks
// (value/checked, insertAdjacentHTML, <head>), and `page(routes)`: a console
// whose hub answers `routes['METHOD path-prefix']` and keeps every call.
// Import this module FIRST (its loader hook must be in place before the
// screen under test is imported). menu-dom.test.mjs still carries its own
// copy of the fakes; new tests use this one.
import { register } from 'node:module';
import { Document, Element } from '../lib/ui/dom-shim.mjs';

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
    export const confirm = async () => F().confirm ?? { ok: true, reason: 'because' };
    export const sheet = html => { document.querySelector('#sheetIn').innerHTML = html; };
    export const closeSheet = () => { F().closed++; };
    export const money = n => String(n), moneyEl = n => '<span>' + n + '</span>';
    export const hydrate = () => {}, retranslate = () => {}, repaintMoney = () => {};
    let onClose = null; export const whenSheetCloses = fn => { onClose = fn; }; export const closeSheetFor = () => { const f = onClose; onClose = null; f?.(); };
    export const switchEl = (id, on, key, hintKey) => check({ id, checked: on, key, hintKey });
    export const lang = 'en', day = ms => String(ms), ago = ms => String(ms);`,
  '/admin/i18n.js': `export const T = { sq: {}, en: {}, uk: {}, ru: {} }; export const lang = 'en'; export const LANGS = ['sq', 'en', 'uk', 'ru']; export const st = k => k, intlLocale = () => 'en', retranslate = () => {};`,
  '/admin/app.js': `const F = () => globalThis.__md; export const loadVenue = async () => { F().loads++; }, rerender = async () => { F().rerenders++; }, me = () => ({}), show = async () => {}, loadOrders = async () => {}, loadCouriers = async () => {}, loadStaff = async () => {};`,
};
register('data:text/javascript,' + encodeURIComponent(`const F = ${JSON.stringify(FAKES)}, P = ${JSON.stringify(PUBLIC)};
  export async function resolve(spec, ctx, next){
    if (F[spec]) return { url: 'data:text/javascript,' + encodeURIComponent(F[spec]), shortCircuit: true };
    if (/^\\/(admin|lib|store)\\//.test(spec)) return { url: P + spec.slice(1), shortCircuit: true };
    return next(spec, ctx);
  }`));

Object.defineProperty(Element.prototype, 'value', { get(){ return this.getAttribute('value') ?? ''; }, set(v){ this.setAttribute('value', v); }, configurable: true });
Object.defineProperty(Element.prototype, 'checked', { get(){ return this.hasAttribute('checked'); }, set(v){ v ? this.setAttribute('checked', '') : this.removeAttribute('checked'); }, configurable: true });
Element.prototype.insertAdjacentHTML = function(where, html){ const tmp = this.ownerDocument.createElement('div'); tmp.innerHTML = html;
  for (const k of [...tmp.childNodes]) { if (where === 'afterbegin' && this.childNodes.length) { k.parentNode = this; this.childNodes.unshift(k); } else this.appendChild(k); } };

export const tick = async (n = 6) => { for (let i = 0; i < n; i++) await new Promise(r => setTimeout(r, 0)); };
export const fakeStorage = (init = {}) => { const m = new Map(Object.entries(init)); return { getItem: k => (m.has(k) ? m.get(k) : null), setItem: (k, v) => m.set(k, String(v)), removeItem: k => m.delete(k), m }; };

/// A console whose hub answers `routes[METHOD path-prefix]` (longest prefix wins; an Error throws; a function answers); every call is kept in `f.calls`.
export function page(routes, S = {}){
  const doc = new Document();
  globalThis.document = doc;
  doc.head = { append(){} };
  const box = doc.createElement('div'); box.setAttribute('id', 'sheetIn'); doc.body.appendChild(box);
  const f = { S: { products: [], venue: { defaultLocale: 'en' }, categories: [], ...S }, toast: [], calls: [], closed: 0, loads: 0, rerenders: 0,
    reply: async (method, path, body) => {
      f.calls.push([method, path, body]);
      const key = Object.keys(routes).filter(k => `${method} ${path}`.startsWith(k)).sort((a, b) => b.length - a.length)[0];
      const a = key ? routes[key] : {};
      if (a instanceof Error) throw a;
      return typeof a === 'function' ? a(body, path) : a;
    } };
  globalThis.__md = f;
  return { box, f, doc };
}
