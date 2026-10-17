// The console's core: who is signed in, how the hub is asked, the one sheet,
// the toast, the bottom bar, the words. Every screen imports from here and
// renders into #app; nothing here renders a screen.
//
// A VIEW OVER SERVER STATE. The queue, the numbers and the menu are re-read,
// never accumulated locally. No status chain lives in the console -- an
// action names an intent and the hub's FSM answers.

import * as Money from '/lib/money.js';
import { bodyOf } from '/lib/body.js';
import { safeGet, safeSet } from '/store/storage.js';
import { t, lang, LANGS, setLang, retranslate, intlLocale } from '/admin/i18n.js';
import * as ui from '/lib/ui/index.js';
import { btn, field, check } from '/admin/parts.js';
import { createRenewer, roleOf, dueToRenew } from '/lib/session-logic.js';
import * as Draft from '/lib/draft.js';
import { DRAFT_WORDS } from '/lib/draft-words.js';
import { fresh } from '/admin/fresh-logic.js';

export const API = '/api';
export const $ = (s, r = document) => r.querySelector(s);
export const $$ = (s, r = document) => [...r.querySelectorAll(s)];
/// The one escaper is the design system's (four byte-identical copies existed).
export const esc = ui.esc;
export const icon = (name, cls = '') => `<i class="ti ti-${name} ${cls}" aria-hidden="true"></i>`;
export { t, lang, LANGS, setLang, retranslate };
// The components render the console's words through its own t(); a key they
// are given also lands as `data-t`, so `retranslate()` keeps working.
ui.useTranslator(t);

/// The queue is re-read this often.
export const POLL_MS = 15_000;
/// When nothing is live, the queue is re-read this much more slowly: an idle
/// venue with an open console used to cost as many requests as a busy one.
export const POLL_IDLE_MS = 60_000;
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
/// `expired`: the hub ended the session (a 401), not the person. What they
/// typed stays on the device for the sign-in that follows (lib/draft.js);
/// an EXPLICIT sign-out forgets it, because the next person at this screen
/// is not its author.
export function logout({ expired = false } = {}){
  store.t = null; store.r = null; S.booted = false;
  stopRenewal();
  if (!expired) Draft.forgetAllDrafts();
  // THE NEXT PERSON AT THIS SCREEN IS NOT ENTITLED TO THIS QUEUE. The replica
  // holds customers' names and addresses; a logout that left it behind would
  // be a privacy hole with a friendly name.
  import('/lib/replica.js').then(R => R.forget()).catch(() => {});
  onLogout?.();
}

const WRITES = new Set(['POST', 'PUT', 'PATCH', 'DELETE']);
/// One fetch wrapper so a 401 has exactly one meaning everywhere: the session is
/// over. It tries a renewal once, then stops. A write that landed forgets the
/// draft of the form whose button sent it.
export async function api(path, opts = {}, retried = false){
  const body = opts.body && typeof opts.body === 'object' && !(opts.body instanceof Blob) ? JSON.stringify(opts.body) : opts.body;
  const r = await fetch(API + path, {
    ...opts, body,
    headers: { ...(body instanceof Blob ? {} : { 'content-type': 'application/json' }), ...(opts.headers || {}),
               ...(store.t ? { authorization: 'Bearer ' + store.t } : {}) },
  });
  if (r.status === 401 && store.t && !retried) {
    const ok = await renew();
    if (ok) return api(path, opts, true);
  }
  if (r.status === 401 && store.t) { logout({ expired: true }); throw new Error(t('sessionOver')); }
  // A write that landed: the draft of its form is forgotten, and EVERY cache
  // the console keeps is stale (W-TZFRESH, TZ 6: the shelf, the review set,
  // the categories re-read on their next ask -- no F5, no sixty-second wait).
  if (r.ok && WRITES.has(String(opts.method || 'GET').toUpperCase())) { Draft.draftSaved(); fresh.bump(path); }
  if (!r.ok) {
    // The hub's refusal, with the machine-readable `code` when it sends one,
    // so a screen can say it in the reader's language.
    let msg = 'HTTP ' + r.status, code = null, d = null;
    try { d = await bodyOf(r); msg = d.error || d.message || msg; code = d.code || null; } catch {}
    const err = new Error(msg); err.status = r.status; if (code) { err.code = code; err.body = d; }
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
/// A member of staff holds no refresh token: the still-valid bearer is the
/// proof, and the hub slides their session (`POST /api/staff/refresh`).
async function renewStaff(){
  try {
    const r = await fetch(API + '/staff/refresh', { method: 'POST', headers: { 'content-type': 'application/json', authorization: 'Bearer ' + store.t }, body: '{}' });
    if (!r.ok) return false;
    const d = await r.json();
    if (d.jwt) store.t = d.jwt;
    return !!d.jwt;
  } catch { return false; }
}
/// Renew whoever is signed in. True when the token moved.
export async function renew(){
  if (store.r) return refresh();
  if (roleOf(store.t) === 'staff') return renewStaff();
  return false;
}
// ── background renewal (TZ 7) ───────────────────────────────────────────────
// Every five minutes while the tab is visible, and on every return to it, the
// token is renewed when under two hours remain (lib/session-logic.js), so an
// open console never meets the 401 mid-edit. The hub still re-derives the
// owner's membership and the staff session on EVERY request: a removed person
// is refused on their next call whatever this timer did.
const renewer = createRenewer({ token: () => store.t, renew, visible: () => !document.hidden });
export function startRenewal(){ renewer.start(); renewer.tick(); }
export function stopRenewal(){ renewer.stop(); }
document.addEventListener('visibilitychange', () => { if (!document.hidden && S.booted) renewer.tick(); });
/// Is the token this tab holds about to end (or gone)? `boot` asks before the first read.
export const tokenDue = () => !store.t || dueToRenew(store.t, Date.now());

// ── drafts (TZ 7) ───────────────────────────────────────────────────────────
/// Every form rendered into `root` keeps what is typed (lib/draft.js). Called
/// by `sheet()` and by app.js after a tab renders; the login screen is not.
export function mountDrafts(root, scope){
  try { return Draft.mountDrafts(root, { app: 'admin', venue: store.loc || '', scope, lang }); } catch { return []; }
}
export const draftWords = () => DRAFT_WORDS[lang] || DRAFT_WORDS.en;
export const hasDrafts = () => { try { return Draft.hasDrafts('admin', store.loc || ''); } catch { return false; } };

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
// One polite live region (`#toast` in index.html, `ui-toast`), one toaster.
let toaster = null;
export function toast(m, o = {}){
  toaster = toaster || ui.createToaster($('#toast'));
  toaster.show(String(m ?? ''), o);
}

// ── the one sheet ───────────────────────────────────────────────────────────
let onClose = null;
export function sheet(html, { name = null, keepScroll = false } = {}){
  const box = $('#sheet'), inner = $('#sheetIn');
  const top = keepScroll && box.classList.contains('show') ? box.scrollTop : 0;
  inner.innerHTML = html;
  retranslate(inner); repaintMoney(inner); hydrate(inner);
  mountDrafts(inner, 'sheet:' + (name || S.tab || ''));
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
  const restore = ui.setBusy(el, t('loading'));
  try { return await fn(); }
  finally { restore(); }
}

/// A confirmation sheet for a consequential action: the one path to it.
/// `hint` is the consequence, one sentence (HUB-APPLE rule B2): what changes
/// and for whom, under the question and before the two buttons.
export function confirm(title, body, { danger = false, reasonLabel = null, reasonDefault = '', hint = null } = {}){
  return new Promise(resolve => {
    sheet(`<p class="eyebrow">${esc(title)}</p><h2>${esc(body)}</h2>
      ${hint ? `<p class="sheet-hint">${esc(hint)}</p>` : ''}
      ${reasonLabel ? field({ id: 'cf-reason', label: reasonLabel, value: reasonDefault, tour: 'confirm.reason' }) : ''}
      <div class="btn-row">${btn({ id: 'cfNo', variant: 'ghost', key: 'cancel', tour: 'confirm.no' })}${btn({ id: 'cfYes', variant: danger ? 'danger' : 'primary', icon: 'check', key: 'done', tour: 'confirm.yes' })}</div>`, { name: 'confirm' });
    let settled = false;
    const done = v => { if (settled) return; settled = true; resolve(v); closeSheet(); };
    $('#cfNo').onclick = () => done(null);
    $('#cfYes').onclick = () => done({ ok: true, reason: $('#cf-reason')?.value.trim() || '' });
    whenSheetCloses(() => { if (!settled) { settled = true; resolve(null); } });
  });
}

/// The switch control, as markup (`parts.check`, the console's one checkbox).
export const switchEl = (id, on, labelKey, hintKey = null, tour = null) => check({ id, checked: on, key: labelKey, hintKey, tour });
