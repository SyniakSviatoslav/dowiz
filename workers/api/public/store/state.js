// The storefront's state and the rules that are not visual.
//
// Everything here existed in the old single-file app.js and is carried over
// unchanged in behaviour: the cart model with option-aware line keys, the
// reading currency and its rates, the venue theme applied as tokens through
// CSSOM, the order history that lives in this browser because the hub keeps no
// customer registry, the saved addresses, the fourteen allergens and the
// filter that hides the undeclared. What changed is that nothing in this file
// renders: a module that owns the truth must not also own the screen, or the
// screen re-renders every time the truth moves.

import * as Money from '/lib/money.js';
import { safeGet, safeSet } from '/store/storage.js';
import { lang, intlLocale } from '/store/i18n.js';

// ── which venue ─────────────────────────────────────────────────────────────
// The host names the venue: `dubin-sushi.dowiz.org` is that venue's storefront.
// A local stand or a preview passes `?s=<slug>` instead, and the platform's
// apex has no venue at all -- the Worker answers that path itself.
export const SLUG = (() => {
  // `?s=` STILL WINS WHEN IT IS GIVEN: the workers.dev deployment has no
  // per-venue hostname and is how this is tested. A workers.dev host is
  // explicitly NOT read as naming a venue -- `dowiz-api.…workers.dev` would
  // otherwise be read as a venue called `dowiz-api`.
  const explicit = new URLSearchParams(location.search).get('s');
  if (explicit) return explicit;
  const host = location.hostname.toLowerCase();
  if (host.endsWith('.workers.dev') || host === 'localhost') return 'demo';
  const labels = host.split('.');
  if (labels.length > 2 && labels[0] !== 'www') return labels[0];
  return 'demo';
})();
export const API = '/api';

// ── state ───────────────────────────────────────────────────────────────────
// `q`, `sort`, `availOnly`, `tag` and `avoid` live HERE and not in the DOM, so
// a re-render for any other reason cannot silently reset what the customer was
// looking at.
export const state = {
  loc: null, cats: [], products: new Map(),
  cart: {}, placing: false,
  q: '', sort: 'pop', availOnly: false, tag: null, avoid: [], avoidOpen: false,
  how: 'delivery', tip: 0, promo: null, geo: null, pin: null,
  currency: null, lastEta: null, sheetName: null,
};

export const findProduct = id => state.products.get(id) || null;
export function indexProducts(cats){
  state.products.clear();
  for (const c of cats) for (const p of (c.products || [])) state.products.set(p.id, p);
}

// ── cart ────────────────────────────────────────────────────────────────────
// A cart line is a dish AND the choices made about it: two rolls of the same
// dish with different extras are two lines, not one with a quantity of two.
export const lineKey = (pid, mods) => [pid, ...[...(mods || [])].sort()].join('|');

export function loadCart(){
  try {
    const raw = JSON.parse(safeGet('dw_cart_' + SLUG) || '{}');
    const out = {};
    for (const [k, v] of Object.entries(raw)) {
      // A cart saved before options existed is `{id: quantity}`. Migrated
      // rather than discarded.
      if (typeof v === 'number') out[lineKey(k, [])] = { p: k, m: [], q: v };
      else if (v && typeof v === 'object' && v.p) out[k] = { p: v.p, m: v.m || [], q: v.q || 1 };
    }
    return out;
  } catch { return {}; }
}
state.cart = loadCart();

// A DISCOUNT BELONGS TO THE BASKET IT WAS QUOTED FOR. Change the basket and the
// number the hub gave back stops describing it, so the quote is dropped with
// the change and re-asked. The delivery estimate is dropped for the same
// reason: it was computed for a basket that no longer exists.
export function saveCart(){
  safeSet('dw_cart_' + SLUG, JSON.stringify(state.cart));
  state.promo = null;
  state.lastEta = null;
  dispatchEvent(new Event('dw:cart'));
}
export function addLine(pid, mods, q){
  const k = lineKey(pid, mods);
  const cur = state.cart[k];
  state.cart[k] = { p: pid, m: mods || [], q: (cur?.q || 0) + q };
  saveCart();
}
export function lineUnit(p, mods){
  const groups = Array.isArray(p.modifierGroups) ? p.modifierGroups : [];
  let d = 0;
  for (const g of groups) for (const o of (g.options || [])) if ((mods || []).includes(o.id)) d += (o.priceDelta | 0);
  return Math.max(0, p.price + d);
}
export function lineNames(p, mods){
  const groups = Array.isArray(p.modifierGroups) ? p.modifierGroups : [];
  const out = [];
  for (const g of groups) for (const o of (g.options || [])) if ((mods || []).includes(o.id)) out.push(o.name);
  return out;
}
export const cartLines = () => Object.entries(state.cart)
  .map(([k, l]) => ({ k, p: findProduct(l.p), m: l.m || [], q: l.q }))
  .filter(x => x.p && x.p.available);
export const cartCount = () => cartLines().reduce((s, l) => s + l.q, 0);
export const subtotal = () => cartLines().reduce((s, l) => s + lineUnit(l.p, l.m) * l.q, 0);
export function deliveryFee(){
  const L = state.loc; if (!L) return 0;
  if (state.how === 'pickup') return 0;
  if (L.freeDeliveryThreshold != null && subtotal() >= L.freeDeliveryThreshold) return 0;
  return L.deliveryFee || 0;
}

// ── money ───────────────────────────────────────────────────────────────────
// Integer minor units in, formatted text out, never parsed back. The customer
// may READ prices in EUR or USD; the charge is in the venue's currency and the
// cart says so. See /lib/money.js.
export let MoneyRates = null;
export const baseCurrency = () => state.loc?.currencyCode || 'ALL';
export const displayCurrency = () => state.currency || baseCurrency();
export const money = n => Money.formatter({
  base: baseCurrency(), display: displayCurrency(), rates: MoneyRates, locale: intlLocale(),
})(n);
/// Money as markup. The integer travels with the node, so a language or
/// currency change re-formats the SAME node: `repaintMoney` walks `[data-money]`
/// and rewrites text. Nothing is ever animated between two amounts.
export const moneyEl = (n, cls = '') =>
  `<span class="money ${cls}" data-money="${Number(n) | 0}">${money(n)}</span>`;
export function repaintMoney(root = document){
  for (const el of root.querySelectorAll('[data-money]')) el.textContent = money(Number(el.dataset.money));
}
export async function setDisplayCurrency(code){
  state.currency = code;
  Money.remember(code);
  if (code !== baseCurrency() && (!MoneyRates || MoneyRates.base !== baseCurrency())) {
    MoneyRates = await Money.loadRates(baseCurrency());
  }
  repaintMoney(document);
  dispatchEvent(new Event('dw:money'));
}
export async function resolveCurrency(){
  state.currency = Money.preferred(baseCurrency());
  if (state.currency !== baseCurrency()) MoneyRates = await Money.loadRates(baseCurrency());
}
export const CURRENCIES = Money.CURRENCIES;
/// What will actually leave the customer's account, when reading another currency.
export function exactCharge(amount){
  const base = baseCurrency();
  if (displayCurrency() === base) return null;
  const stale = MoneyRates && MoneyRates.stale ? ' ⚠' : '';
  return Money.format(amount, base, intlLocale()) + stale;
}

// ── features ────────────────────────────────────────────────────────────────
// A FLAG IS OFF UNLESS THE HUB SAYS ON, and an absent `features` block means
// every flag is on.
export function on(name){
  const f = state.loc?.features;
  if (!f) return true;
  return f[name] !== false;
}

// ── the venue's own colours ─────────────────────────────────────────────────
// Applied as tokens on the root element through CSSOM; the server derives and
// contrast-checks the set, the client only wears it. The type pairs live here
// and the hub only ever names one: a font-family arriving from the network is
// arbitrary text reaching CSS.
const TYPE_PAIRS = {
  classic: { heading: "Georgia,'Iowan Old Style','Times New Roman',serif",
             body: "system-ui,-apple-system,'Segoe UI',Roboto,sans-serif" },
  modern:  { heading: "system-ui,-apple-system,'Segoe UI',sans-serif",
             body: "system-ui,-apple-system,'Segoe UI',sans-serif" },
  warm:    { heading: "Georgia,'Iowan Old Style','Palatino Linotype',serif",
             body: "Georgia,'Iowan Old Style','Palatino Linotype',serif" },
  plain:   { heading: "ui-sans-serif,system-ui,sans-serif",
             body: "ui-sans-serif,system-ui,sans-serif" },
};
let VENUE_THEME = null;
let themeApplied = [];
export function themeMode(){
  const explicit = document.documentElement.getAttribute('data-theme');
  if (explicit === 'dark' || explicit === 'light') return explicit;
  return matchMedia('(prefers-color-scheme:dark)').matches ? 'dark' : 'light';
}
export function paintTheme(){
  const root = document.documentElement;
  for (const name of themeApplied) root.style.removeProperty(name);
  themeApplied = [];
  if (!VENUE_THEME) return;
  // THE VENUE'S PAPER DECIDES, NOT THE BROWSER. A venue that chose a dark paper
  // (#07141c) is a dark venue in every browser; the "light" set the hub derived
  // carries that paper too, so the browser's preference only picks between two
  // sets that share the venue's ground. If the light set's paper is dark, the
  // dark set is the one that would betray the brand, and it is not used.
  const lightBg = (VENUE_THEME.light.find(([k]) => k === '--brand-bg') || [])[1];
  const lightIsDark = lightBg && luminance(lightBg) < 0.35;
  const decls = (!lightIsDark && themeMode() === 'dark' && VENUE_THEME.dark.length)
    ? VENUE_THEME.dark : VENUE_THEME.light;
  for (const [name, value] of decls){ root.style.setProperty(name, value); themeApplied.push(name); }
  for (const [name, value] of VENUE_THEME.shape){ root.style.setProperty(name, value); themeApplied.push(name); }
  root.setAttribute('data-paper', (lightIsDark || (themeMode() === 'dark')) ? 'dark' : 'light');
  // The browser chrome should be the colour the page actually is.
  const bg = decls.find(([k]) => k === '--brand-bg');
  if (bg) for (const m of document.querySelectorAll('meta[name="theme-color"]')) m.setAttribute('content', bg[1]);
}
function luminance(hex){
  const h = hex.replace('#', '');
  const n = h.length === 3 ? h.split('').map(c => c + c).join('') : h;
  const [r, g, b] = [0, 2, 4].map(i => parseInt(n.slice(i, i + 2), 16) / 255);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
export function applyTheme(theme){
  if (!theme || !theme.light) { VENUE_THEME = null; paintTheme(); return; }
  const safe = s => String(s || '').split(';')
    .map(d => d.trim())
    .filter(d => /^--brand-[a-z-]+:\s*#[0-9a-fA-F]{3,8}$/.test(d))
    .map(d => { const i = d.indexOf(':'); return [d.slice(0, i).trim(), d.slice(i + 1).trim()]; });
  const light = safe(theme.light), dark = safe(theme.dark);
  if (!light.length) { VENUE_THEME = null; paintTheme(); return; }
  const pair = TYPE_PAIRS[theme.typePair] || TYPE_PAIRS.classic;
  const r = Math.max(0, Math.min(20, parseInt(theme.radius, 10) || 0));
  const shape = [
    ['--brand-font-heading', pair.heading],
    ['--brand-font-body', pair.body],
    ['--radius-sm', `${Math.round(r / 2)}px`],
    ['--radius-md', `${r}px`],
    ['--radius-lg', `${Math.round(r * 1.5)}px`],
    ['--radius-xl', `${r * 2}px`],
  ];
  VENUE_THEME = { light, dark, shape };
  paintTheme();
}
matchMedia('(prefers-color-scheme:dark)').addEventListener('change', paintTheme);

// ── the venue's stage ───────────────────────────────────────────────────────
// Two supporting colours from the venue's own mark, as tokens on the root.
// The hub checks them on the way in; they are checked again here, because
// a value that reaches CSS is a value that has to be a colour.
const STAGE_COLOURS = { warm: '--stage-warm', sage: '--stage-sage' };
const HEX_COLOUR = /^#[0-9a-fA-F]{6}$/;
export function applyStage(stage){
  const root = document.documentElement;
  for (const [key, name] of Object.entries(STAGE_COLOURS)) {
    const v = stage?.[key];
    if (typeof v === 'string' && HEX_COLOUR.test(v)) root.style.setProperty(name, v);
    else root.style.removeProperty(name);
  }
}

// ── my orders ───────────────────────────────────────────────────────────────
// NO ACCOUNT, and that is the design. Each order comes back with a token
// scoped to that ONE order, and the browser keeps the list.
const HIST_KEY = 'dw_orders';
export const history = () => { try { return JSON.parse(safeGet(HIST_KEY) || '[]'); } catch { return []; } };
export function remember(order){
  if (!order?.id || !order?.access_token) return;
  const list = history().filter(o => o.id !== order.id);
  list.unshift({ id: order.id, t: order.access_token, at: Date.now(), total: order.total ?? 0 });
  safeSet(HIST_KEY, JSON.stringify(list.slice(0, 20)));
}
export async function fetchRemembered(entry){
  const r = await fetch(`${API}/order/${encodeURIComponent(entry.id)}`,
                        { headers: { authorization: 'Bearer ' + entry.t } });
  if (!r.ok) throw new Error('HTTP ' + r.status);
  return r.json();
}
export const tokenFor = id => history().find(x => x.id === id)?.t || null;

// ── the addresses this browser has ordered to ───────────────────────────────
// A choice, not a blank field; kept here for the same reason the history is.
const ADDR_KEY = 'dw_addrs';
const normAddr = x => String(x || '').replace(/\s+/g, ' ').trim().toLowerCase();
export const addresses = () => {
  try {
    const list = JSON.parse(safeGet(ADDR_KEY) || '[]');
    return Array.isArray(list) ? list.filter(a => a && typeof a === 'object' && a.line) : [];
  } catch { return []; }
};
/// Most recent first, no duplicates, six at most. An address carries the pin
/// it was confirmed at, when there was one, so choosing it again needs no map.
export function rememberAddress(entry){
  const line = String(entry?.line || '').trim();
  if (!line) return;
  const list = [{ line, lat: entry.lat ?? null, lng: entry.lng ?? null },
                ...addresses().filter(x => normAddr(x.line) !== normAddr(line))].slice(0, 6);
  safeSet(ADDR_KEY, JSON.stringify(list));
}
export function forgetAddress(line){
  safeSet(ADDR_KEY, JSON.stringify(addresses().filter(x => normAddr(x.line) !== normAddr(line))));
}

// ── allergens ───────────────────────────────────────────────────────────────
// THE FILTER HIDES THE UNDECLARED TOO, and that is the whole design.
export const ALLERGENS = [
  ['gluten',      { sq:'Gluten',      en:'Gluten',      uk:'Глютен' }],
  ['crustaceans', { sq:'Guaskorë',    en:'Crustaceans', uk:'Ракоподібні' }],
  ['eggs',        { sq:'Vezë',        en:'Eggs',        uk:'Яйця' }],
  ['fish',        { sq:'Peshk',       en:'Fish',        uk:'Риба' }],
  ['peanuts',     { sq:'Kikirikë',    en:'Peanuts',     uk:'Арахіс' }],
  ['soy',         { sq:'Soja',        en:'Soy',         uk:'Соя' }],
  ['milk',        { sq:'Qumësht',     en:'Milk',        uk:'Молоко' }],
  ['nuts',        { sq:'Arra',        en:'Nuts',        uk:'Горіхи' }],
  ['celery',      { sq:'Selino',      en:'Celery',      uk:'Селера' }],
  ['mustard',     { sq:'Mustardë',    en:'Mustard',     uk:'Гірчиця' }],
  ['sesame',      { sq:'Susam',       en:'Sesame',      uk:'Кунжут' }],
  ['sulphites',   { sq:'Sulfite',     en:'Sulphites',   uk:'Сульфіти' }],
  ['lupin',       { sq:'Lupin',       en:'Lupin',       uk:'Люпин' }],
  ['molluscs',    { sq:'Molusqe',     en:'Molluscs',    uk:'Молюски' }],
];
export const allergenName = c => {
  const row = ALLERGENS.find(a => a[0] === c);
  return row ? (row[1][lang] || row[1].en) : c;
};
export function loadAvoid(){
  try {
    const v = JSON.parse(safeGet('dw_avoid') || '[]');
    return Array.isArray(v) ? v.filter(c => ALLERGENS.some(a => a[0] === c)) : [];
  } catch { return []; }
}
export const saveAvoid = () => safeSet('dw_avoid', JSON.stringify(state.avoid || []));
/// Why this dish is hidden by the allergen filter, or null.
export function hiddenBecause(p){
  const avoid = state.avoid || [];
  if (!avoid.length) return null;
  if (!Array.isArray(p.allergens)) return 'undeclared';
  return p.allergens.some(c => avoid.includes(c)) ? 'contains' : null;
}

// ── text ────────────────────────────────────────────────────────────────────
/// Accent- and case-insensitive: "cmimi" must find "çmimi".
export const normalise = x => String(x ?? '').toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '');

// ── time ────────────────────────────────────────────────────────────────────
export const DAY_NAMES = {
  sq: ['E hënë','E martë','E mërkurë','E enjte','E premte','E shtunë','E diel'],
  en: ['Monday','Tuesday','Wednesday','Thursday','Friday','Saturday','Sunday'],
  uk: ['Понеділок','Вівторок','Середа','Четвер','Пʼятниця','Субота','Неділя'],
};
export const hhmm = m => String(Math.floor(m / 60)).padStart(2, '0') + ':' + String(m % 60).padStart(2, '0');
/// Which weekday it is where the VENUE is, not where the phone is.
export const todayAt = () => {
  const off = Number.isFinite(state.loc?.tzOffsetMinutes) ? state.loc.tzOffsetMinutes : 120;
  const local = new Date(Date.now() + off * 60_000);
  return { day: (local.getUTCDay() + 6) % 7, minute: local.getUTCHours() * 60 + local.getUTCMinutes() };
};
/// "Opens Monday at 11:00" rather than "closed". Weekday 0 is Monday.
export function whenOpens(n){
  const days = { uk:['пн','вт','ср','чт','пт','сб','нд'], en:['Mon','Tue','Wed','Thu','Fri','Sat','Sun'],
                 sq:['Hën','Mar','Mër','Enj','Pre','Sht','Die'] };
  const d = (days[lang] || days.en)[n.weekday] ?? '';
  const time = hhmm(n.minute || 0);
  return n.weekday === todayAt().day ? time : `${d} ${time}`;
}
