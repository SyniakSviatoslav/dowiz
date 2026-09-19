// The console's core: who is signed in, how the hub is asked, the one sheet,
// the toast, the bottom bar, the words. Every screen imports from here and
// renders into #app; nothing here renders a screen.
//
// A VIEW OVER SERVER STATE. The queue, the numbers and the menu are re-read,
// never accumulated locally. No status chain lives in the console -- an
// action names an intent and the hub's FSM answers.

import * as Money from '/lib/money.js';
import { safeGet, safeSet } from '/store/storage.js';
import { t, lang, LANGS, setLang, retranslate, intlLocale } from '/admin/i18n.js';

export const API = '/api';
export const $ = (s, r = document) => r.querySelector(s);
export const $$ = (s, r = document) => [...r.querySelectorAll(s)];
export const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
export const icon = (name, cls = '') => `<i class="ti ti-${name} ${cls}" aria-hidden="true"></i>`;
export { t, lang, LANGS, setLang, retranslate };

/// The toast shows for this long; the queue is re-read this often.
const TOAST_MS = 2600;
export const POLL_MS = 15_000;
/// A drag on the sheet's grip past this many pixels dismisses it.
const SHEET_DISMISS_PX = 90;
/// The order id is shown short: enough to tell two apart, short enough to say.
export const ORDER_ID_SHOWN = 8;

// ── the session ─────────────────────────────────────────────────────────────
// The access token lives in sessionStorage (gone when the tab closes); the
// refresh token in localStorage so a reload does not force a re-login.
export const store = {
  get t(){ try { return sessionStorage.getItem('dw_at'); } catch { return null; } },
  set t(v){ try { v ? sessionStorage.setItem('dw_at', v) : sessionStorage.removeItem('dw_at'); } catch {} },
  get r(){ try { return localStorage.getItem('dw_rt'); } catch { return null; } },
  set r(v){ try { v ? localStorage.setItem('dw_rt', v) : localStorage.removeItem('dw_rt'); } catch {} },
  get loc(){ try { return localStorage.getItem('dw_loc'); } catch { return null; } },
  set loc(v){ try { v ? localStorage.setItem('dw_loc', v) : localStorage.removeItem('dw_loc'); } catch {} },
};

/// What every screen shares: the venue, the queue, the day's numbers.
export const S = {
  tab: 'orders', orders: [], stats: null, venue: null, couriers: [], products: [], categories: [],
  phase: 'loading', error: null, booted: false, seen: new Set(), fresh: new Set(),
};

let onLogout = null;
export function whenLoggedOut(fn){ onLogout = fn; }
export function logout(){ store.t = null; store.r = null; S.booted = false; onLogout?.(); }

/// One fetch wrapper so a 401 has exactly one meaning everywhere: the session is
/// over. It tries a refresh once, then stops.
export async function api(path, opts = {}, retried = false){
  const body = opts.body && typeof opts.body === 'object' && !(opts.body instanceof Blob) ? JSON.stringify(opts.body) : opts.body;
  const r = await fetch(API + path, {
    ...opts, body,
    headers: { ...(body instanceof Blob ? {} : { 'content-type': 'application/json' }), ...(opts.headers || {}),
               ...(store.t ? { authorization: 'Bearer ' + store.t } : {}) },
  });
  if (r.status === 401 && !retried && store.r) {
    const ok = await refresh();
    if (ok) return api(path, opts, true);
    logout(); throw new Error(t('sessionOver'));
  }
  if (!r.ok) {
    // The hub's refusal, with the machine-readable `code` when it sends one,
    // so a screen can say it in the reader's language.
    let msg = 'HTTP ' + r.status, code = null;
    try { const d = await r.json(); msg = d.error || d.message || msg; code = d.code || null; } catch {}
    const err = new Error(msg); err.status = r.status; if (code) err.code = code;
    throw err;
  }
  return r.status === 204 ? null : r.json();
}
export const post = (path, body) => api(path, { method: 'POST', body });
/// Every owner write names the venue.
export const withLoc = (body = {}) => ({ location_id: store.loc, ...body });

async function refresh(){
  try {
    const r = await fetch(API + '/auth/refresh', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ refresh_token: store.r }) });
    if (r.status === 409) return true;
    if (!r.ok) return false;
    const d = await r.json();
    store.t = d.access_token; if (d.refresh_token) store.r = d.refresh_token;
    return true;
  } catch { return false; }
}

// ── money ───────────────────────────────────────────────────────────────────
// Integer minor units in, formatted text out, never parsed back.
let MoneyBase = 'ALL', MoneyDisplay = 'ALL', MoneyRates = null;
let MoneyFmt = Money.formatter({ base: MoneyBase, display: MoneyDisplay, rates: null, locale: intlLocale() });
export const money = n => MoneyFmt(n);
export const moneyEl = (n, cls = '') => `<span class="money ${cls}" data-money="${Number(n) | 0}">${money(n)}</span>`;
export async function setCurrency(base, display){
  MoneyBase = base || MoneyBase; MoneyDisplay = display || MoneyBase;
  if (MoneyDisplay !== MoneyBase && (!MoneyRates || MoneyRates.base !== MoneyBase)) MoneyRates = await Money.loadRates(MoneyBase);
  MoneyFmt = Money.formatter({ base: MoneyBase, display: MoneyDisplay, rates: MoneyRates, locale: intlLocale() });
  Money.remember(MoneyDisplay);
  repaintMoney(document);
}
export function repaintMoney(root = document){ for (const el of root.querySelectorAll('[data-money]')) el.textContent = money(Number(el.dataset.money)); }
export const CURRENCIES = Money.CURRENCIES;
export const displayCurrency = () => MoneyDisplay;
export const baseCurrency = () => MoneyBase;

// ── time ────────────────────────────────────────────────────────────────────
/// "3 min ago" / "just now", from a millisecond stamp.
/// Minutes up to an hour, hours up to a day, then days: "662 min ago" is a
/// number nobody reads on a phone.
const MIN_PER_HOUR = 60, HOURS_PER_DAY = 24;
export function ago(ms){
  const m = Math.max(0, Math.round((Date.now() - ms) / 60_000));
  if (m === 0) return t('justNow');
  if (m < MIN_PER_HOUR) return `${m} ${t('minutesAgo')}`;
  const h = Math.floor(m / MIN_PER_HOUR);
  if (h < HOURS_PER_DAY) return `${h} ${t('hoursAgo')}`;
  return `${Math.floor(h / HOURS_PER_DAY)} ${t('daysAgo')}`;
}
export const clock = ms => new Date(ms).toLocaleTimeString(intlLocale(), { hour: '2-digit', minute: '2-digit' });
export const day = ms => new Date(ms).toLocaleDateString(intlLocale(), { day: 'numeric', month: 'short' });

// ── the toast ───────────────────────────────────────────────────────────────
export function toast(m){
  const el = $('#toast'); el.textContent = m; el.classList.add('show');
  clearTimeout(toast._t); toast._t = setTimeout(() => el.classList.remove('show'), TOAST_MS);
}

// ── the one sheet ───────────────────────────────────────────────────────────
let onClose = null;
export function sheet(html, { name = null, keepScroll = false } = {}){
  const box = $('#sheet'), inner = $('#sheetIn');
  const top = keepScroll && box.classList.contains('show') ? box.scrollTop : 0;
  inner.innerHTML = html;
  retranslate(inner); repaintMoney(inner); hydrate(inner);
  box.dataset.name = name || '';
  box.classList.add('show'); $('#scrim').classList.add('show'); document.body.classList.add('sheet-open');
  box.scrollTop = top;
}
export function closeSheet(){
  const box = $('#sheet');
  if (!box.classList.contains('show')) return;
  box.classList.remove('show'); $('#scrim').classList.remove('show'); document.body.classList.remove('sheet-open');
  box.dataset.name = '';
  const fn = onClose; onClose = null; if (fn) fn();
}
export const sheetName = () => $('#sheet').dataset.name || null;
export function whenSheetCloses(fn){ onClose = fn; }
export function bindSheetChrome(){
  $('#scrim').onclick = closeSheet;
  $('#sheetClose').onclick = closeSheet;
  addEventListener('keydown', e => { if (e.key === 'Escape') closeSheet(); });
  const box = $('#sheet'), grip = $('#grab');
  let y0 = null, dy = 0;
  grip.addEventListener('pointerdown', e => { y0 = e.clientY; dy = 0; box.classList.add('dragging'); grip.setPointerCapture(e.pointerId); });
  grip.addEventListener('pointermove', e => { if (y0 === null) return; dy = Math.max(0, e.clientY - y0); box.style.transform = `translateY(${dy}px)`; });
  const end = () => { if (y0 === null) return; box.classList.remove('dragging'); box.style.transform = ''; if (dy > SHEET_DISMISS_PX) closeSheet(); y0 = null; dy = 0; };
  grip.addEventListener('pointerup', end); grip.addEventListener('pointercancel', end);
}

// ── CSSOM hydration ─────────────────────────────────────────────────────────
// `style-src 'self'` drops a `style=` attribute, so a width, a stagger index
// and a swatch travel as data- attributes and are written through CSSOM.
const HEX = /^#[0-9a-fA-F]{3,8}$/;
export function hydrate(root){
  if (!root || root.nodeType !== 1) return;
  for (const el of root.querySelectorAll('[data-w]')) { el.style.width = Number(el.dataset.w) + '%'; el.removeAttribute('data-w'); }
  for (const el of root.querySelectorAll('[data-h]')) { el.style.height = Number(el.dataset.h) + '%'; el.removeAttribute('data-h'); }
  for (const el of root.querySelectorAll('[data-i]')) { el.style.setProperty('--i', String(Number(el.dataset.i) || 0)); el.removeAttribute('data-i'); }
  for (const el of root.querySelectorAll('[data-bg]')) { if (HEX.test(el.dataset.bg)) el.style.background = el.dataset.bg; el.removeAttribute('data-bg'); }
  for (const el of root.querySelectorAll('[data-st-var]')) { el.style.setProperty('--st', `var(--st-${el.dataset.stVar})`); el.removeAttribute('data-st-var'); }
}

// ── a button that waits shows that it waits ─────────────────────────────────
export async function busy(el, fn){
  if (!el) return fn();
  const was = el.innerHTML; el.disabled = true; el.setAttribute('aria-busy', 'true');
  el.innerHTML = `${icon('loader-2', 'spin')}`;
  try { return await fn(); }
  finally { el.disabled = false; el.removeAttribute('aria-busy'); el.innerHTML = was; }
}

/// A confirmation sheet for a consequential action: the one path to it.
export function confirm(title, body, { danger = false, reasonLabel = null, reasonDefault = '' } = {}){
  return new Promise(resolve => {
    sheet(`<p class="eyebrow">${esc(title)}</p><h2>${esc(body)}</h2>
      ${reasonLabel ? `<label for="cf-reason">${esc(reasonLabel)}</label><input id="cf-reason" value="${esc(reasonDefault)}">` : ''}
      <div class="btn-row"><button class="btn ghost" id="cfNo" data-t="cancel"></button><button class="btn ${danger ? 'danger' : ''}" id="cfYes" data-t="done"></button></div>`, { name: 'confirm' });
    let settled = false;
    const done = v => { if (settled) return; settled = true; resolve(v); closeSheet(); };
    $('#cfNo').onclick = () => done(null);
    $('#cfYes').onclick = () => done({ ok: true, reason: $('#cf-reason')?.value.trim() || '' });
    whenSheetCloses(() => { if (!settled) { settled = true; resolve(null); } });
  });
}

/// The switch control, as markup.
export const switchEl = (id, on, labelKey, hintKey = null) => `<label class="switch"><input type="checkbox" id="${id}" ${on ? 'checked' : ''}><span class="switch-k"></span>
  <span class="t"><span data-t="${labelKey}"></span>${hintKey ? `<small data-t="${hintKey}"></small>` : ''}</span></label>`;
