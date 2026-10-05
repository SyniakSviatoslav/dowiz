// The guest's taste memory ON THIS PHONE: the switch, the IndexedDB record, the hooks that feed it,
// the "For you" strip and the sheet that shows and forgets it (W-MR0 row MR7, 2026-10-04).
// What is scored and how lives in store/taste.js (pure, node-tested); this file only wires it.
//
// ON BY DEFAULT, OFF IN ONE TAP (operator ruling 2026-10-04: personalisation is automatic, basis
// legitimate interest, DECISIONS.md D0). `dw_taste_on` = '0' is the guest's "off"; off, nothing is
// recorded and nothing is read. The record is `dowiz.taste.v1` in IndexedDB, one entry per venue,
// and it never leaves the phone raw: the only thing that may (`syncVector`, aggregated weights)
// goes with an order, and never once the guest said "turn off" for the venue (`notObjected()`).
// ePrivacy 5(3) is the honest residual risk here (docs/privacy/DPIA-personalisation.md §2): this
// is first-party storage the guest sees in full ("what this phone remembers") and wipes in a tap.
// NEVER A FINGERPRINT: the venue recognises a guest only by what they gave it -- the phone number
// at checkout, or the order link they hold -- never by the device (no-tracking.sh rule 5).

import { state, SLUG, findProduct, history, API, todayAt } from '/store/state.js';
import { t, retranslate } from '/store/i18n.js';
import { $, $$, esc, icon, sheet, toast } from '/store/ui.js';
import { safeGet, safeSet } from '/store/storage.js';
import * as T from '/store/taste.js';
import { ui } from '/store/parts.js';
import '/store/taste-words.js';
import { currentMood, momentKeys, moodMarkup, wireMood } from '/store/sense-ui.js';
import { becauseLine, yearMarkup } from '/store/sense-view.js';
import { senseOf } from '/store/sense.js';

const SWITCH = 'dw_taste_on';
/// The guest said "turn off" to the VENUE's profile (Art. 21). Kept here so the phone stops sending
/// at once, and sent once with the next order (`taste_off`) so the hub files it under their phone.
const OBJECTED = 'dw_taste_objected';
const DB = 'dowiz.taste.v1', STORE = 'p';
/// Writes are coalesced: a burst of opens is one IndexedDB put.
const SAVE_AFTER_MS = 800;

export const isOn = () => safeGet(SWITCH) !== '0';
/// Neither switch is off: the phone may send its aggregated vector with an order.
export const notObjected = () => isOn() && safeGet(OBJECTED) !== '1';
const today = () => T.dayOf(Date.now(), new Date().getTimezoneOffset());

// ── the record ─────────────────────────────────────────────────────────────
function db(){
  return new Promise((ok, no) => {
    try {
      const r = indexedDB.open(DB, 1);
      r.onupgradeneeded = () => r.result.createObjectStore(STORE);
      r.onsuccess = () => ok(r.result); r.onerror = () => no(r.error);
    } catch (e) { no(e); }
  });
}
async function tx(mode, fn){
  const d = await db();
  return new Promise((ok, no) => { const x = d.transaction(STORE, mode); const r = fn(x.objectStore(STORE)); x.oncomplete = () => ok(r?.result); x.onerror = () => no(x.error); });
}
let profile = null, saveTimer = null;
async function load(){
  if (profile) return profile;
  let got = null;
  try { got = await tx('readonly', s => s.get(SLUG)); } catch (e) { console.warn('taste: the record could not be read:', String(e?.message || e)); }
  profile = got && got.v === T.VERSION ? got
    : T.empty({ day: today(), device: T.deviceClass(innerWidth, matchMedia('(pointer: coarse)').matches), first: T.firstVisit(document.referrer, location.search) });
  return profile;
}
function save(){
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => { tx('readwrite', s => s.put(profile, SLUG)).catch(e => console.warn('taste: not saved:', String(e?.message || e))); }, SAVE_AFTER_MS);
}
const menuNow = () => [...(state.products?.values?.() || [])];
async function note(ev){
  if (!isOn()) return;
  profile = T.record(await load(), ev, today());
  // W-SENSE: an order re-takes this month's snapshot of the taste vector ("your taste over the year").
  if (ev.kind === 'order') profile = T.snapshot(profile, menuNow(), today());
  save();
}
/// The vector as the strip last saw it (sync, for the dish sheet); null while off.
let guestCache = null;
export const guestNow = () => (isOn() ? guestCache : null);
/// The guest's taste/texture/aroma vector on this phone, for the dish sheet's "you may like it".
export async function guestVec(){
  if (!isOn()) return null;
  return T.senseVec(await load(), menuNow(), today());
}
export async function forget(){
  profile = null; clearTimeout(saveTimer);
  try { await tx('readwrite', s => s.delete(SLUG)); } catch (e) { console.warn('taste: not forgotten:', String(e?.message || e)); }
}

// ── the hooks ──────────────────────────────────────────────────────────────
// A dish sheet's time runs from its opening to the next thing the guest does (another dish, the
// basket, leaving the page): the sheet itself has no close event this file can own.
let open = null;
/// This visit only (the intent guess), never stored.
const session = { adds: 0, opens: 0 };
function endDwell(){ if (open) { note({ kind: 'dwell', id: open.id, sec: (Date.now() - open.at) / 1000 }); open = null; } }
export function noteOpen(id){ if (!isOn()) return; endDwell(); open = { id, at: Date.now() }; session.opens += 1; note({ kind: 'open', id }); }
const seenNow = new Set();
/// A category the guest scrolled to, once per visit.
export function noteSeen(cat){ if (!isOn() || !cat || seenNow.has(cat)) return; seenNow.add(cat); note({ kind: 'seen', cat }); }

/// The basket, as portions per dish.
const portions = () => { const m = {}; for (const l of Object.values(state.cart || {})) m[l.p] = (m[l.p] || 0) + (l.q | 0); return m; };
let before = {}, ordered = false;
function cartMoved(){
  setTimeout(() => {
    const now = portions();
    if (ordered) {
      note({ kind: 'order', items: Object.entries(before).map(([id, qty]) => ({ id, qty })), ctx: momentKeys() || [] });
      ordered = false;
    } else {
      for (const id of new Set([...Object.keys(before), ...Object.keys(now)])) {
        const d = (now[id] || 0) - (before[id] || 0);
        if (d > 0) { note({ kind: 'add', id }); session.adds += 1; }
        if (d < 0) note({ kind: 'drop', id });
      }
    }
    before = now;
  }, 0);
}
let wired = false;
/// Once per page: the basket, the order, leaving the page, and the visit's hour.
export function wireTaste(){
  if (wired) return; wired = true;
  before = portions();
  addEventListener('dw:cart', () => { endDwell(); cartMoved(); });
  addEventListener('dw:ordered', () => { ordered = true; });
  addEventListener('visibilitychange', () => { if (document.visibilityState === 'hidden') endDwell(); });
  note({ kind: 'visit', hour: Math.floor(todayAt().minute / 60) }); // the venue's hour, from its zone
}

// ── what the guest sees ────────────────────────────────────────────────────
/// The filter-row button that opens the sheet.
export const tasteButton = () => ui.chip({ as: 'button', selected: isOn(), icon: 'sparkles', label: { t: 'forYou' },
  cls: ui.cx('tag', isOn() && 'on'), attrs: { id: 'tasteOpen', data: { tour: 'menu.taste' } } });

/// The strip above the menu: drawn only when on and when there is something to say.
export async function paintStrip(onOpen){
  const host = $('#forYou'); if (!host) return;
  if (!isOn()) { host.hidden = true; host.innerHTML = ''; return; }
  const p = await load();
  const products = [...(state.products?.values?.() || [])];
  const items = T.strip(products, p, today(), { prior, ctx: momentKeys(), mood: currentMood() }).map(x => ({ ...x, p: findProduct(x.id) })).filter(x => x.p);
  const senses = products.some(d => senseOf(d));
  host.hidden = !items.length && !senses;
  if (host.hidden) { host.innerHTML = ''; return; }
  const cart = Object.values(state.cart || {});
  const it = T.intent({ cartLines: cart.length, cartQty: cart.reduce((a, l) => a + (l.q | 0), 0), adds: session.adds, opens: session.opens });
  host.innerHTML = `<h2 class="sec-h"><span class="sec-name" data-t="forYou"></span><small class="muted" data-t="intent_${it.kind}"></small></h2>
    <div class="fy-row">${items.map(x => ui.chip({ as: 'button', icon: x.why === 'again' ? 'history' : 'sparkles', label: x.p.name, cls: 'fy',
      ariaLabel: `${t(x.why === 'again' ? 'again' : 'yourTaste')} ${x.p.name}`, attrs: { data: { fy: x.id, tour: 'menu.forYou' } } })).join('')}</div>
    ${becauseLine(T.because(guestCache = T.senseVec(p, products, today())))}${senses ? moodMarkup() : ''}`;
  retranslate(host);
  wireMood(host.querySelector('.sx-mood'), () => paintStrip(onOpen));
  for (const b of $$('[data-fy]', host)) b.onclick = () => { const d = findProduct(b.dataset.fy); if (d && d.available !== false) onOpen?.(d, null); };
}

/// The sheet: the switch, the plain list of what is kept, and Forget.
export async function openTaste(onChange){
  const on = isOn();
  const r = on ? T.remembered(await load()) : null;
  const rows = r ? [['tr_since', r.since], ['tr_ordered', r.ordered], ['tr_portions', r.portions], ['tr_opened', r.opened], ['tr_added', r.added],
    ['tr_removed', r.removed], ['tr_dwell', r.dwellMin], ['tr_cats', r.categoriesSeen], ['tr_visits', r.visits], ['tr_device', r.device || '-'],
    ['tr_ref', [r.ref, r.utm && Object.values(r.utm).join(' / ')].filter(Boolean).join(' · ') || '-']] : [];
  sheet(`<p class="eyebrow" data-t="forYou"></p><h2 data-t="tasteTitle"></h2>
    ${ui.chip({ as: 'button', selected: on, icon: on ? 'check' : 'x', label: { t: 'tasteSwitch' }, cls: ui.cx('taste-switch', on && 'on'), attrs: { id: 'tasteOn', data: { tour: 'taste.switch' } } })}
    <p class="muted small" data-t="tasteHint"></p>
    <h3 class="dsec" data-t="tasteRemembers"></h3>
    ${rows.length ? `<dl class="tr-list">${rows.map(([k, v]) => `<dt data-t="${k}"></dt><dd class="mono">${esc(String(v))}</dd>`).join('')}</dl>` : `<p class="muted small" data-t="tasteNothing"></p>`}
    ${on ? yearMarkup((await load()).months) : ''}
    ${ui.button({ variant: 'ghost', label: { t: 'tasteForget' }, id: 'tasteForget', cls: 'linky', attrs: { data: { tour: 'taste.forget' } } })}`, { name: 'taste' });
  retranslate($('#sheetIn'));
  $('#tasteOn').onclick = async () => {
    if (!isOn()) { safeSet(SWITCH, '1'); wireTaste(); } else { safeSet(SWITCH, '0'); }
    onChange?.(); openTaste(onChange);
  };
  $('#tasteForget').onclick = async () => { await forget(); toast(t('tasteForgot')); onChange?.(); openTaste(onChange); };
}

/// MR8: the aggregated vector an order may carry.
export async function tasteVector(){
  if (!notObjected()) return null;
  return T.syncVector(await load(), [...(state.products?.values?.() || [])], today());
}

/// What an order body carries for the venue's profile: the vector by default, or -- once the guest
/// said "turn off" -- the objection itself (`taste_off`), so the hub stops and deletes under the
/// phone they give. Nothing without a phone: there is nobody to keep it under.
export async function tasteBody(phone){
  if (!phone) return {};
  if (safeGet(OBJECTED) === '1') return { taste_off: true };
  const vec = await tasteVector();
  return vec && notObjected() ? { taste_sync: vec } : {};
}

/// "We remember your taste · turn off": the objection, on this phone at once and at the venue
/// through every order link this phone holds (`POST /api/order/:id/taste/withdraw` deletes the
/// profile). `links` = [{id, token}]. Answers how many the venue confirmed.
export async function objectVenue(links, api){
  safeSet(OBJECTED, '1');
  let done = 0;
  for (const l of links || []) {
    try {
      const r = await fetch(`${api}/order/${encodeURIComponent(l.id)}/taste/withdraw`, { method: 'POST', headers: { authorization: 'Bearer ' + l.token } });
      if (r.ok) done += 1;
    } catch { /* the next order carries taste_off */ }
  }
  return done;
}
export const objected = () => safeGet(OBJECTED) === '1';

/// The venue's view of this guest, read through the newest order link this phone holds; it ranks
/// the strip with the phone's own (`T.strip` prior). Silent on failure.
let prior = null;
export async function loadPrior(link, api){
  if (!link || objected()) return null;
  try {
    const r = await fetch(`${api}/order/${encodeURIComponent(link.id)}/taste`, { headers: { authorization: 'Bearer ' + link.token } });
    if (r.ok) prior = (await r.json())?.taste || null;
  } catch { prior = null; }
  return prior;
}

// ── "We remember your taste · turn off" ────────────────────────────────────
/// The newest order link this phone holds, as {id, token}: how the venue recognises a returning
/// guest without an account and without a fingerprint.
export const newestLink = () => { const h = history()[0]; return h?.id && h?.t ? { id: h.id, token: h.t } : null; };
/// The one visible line (storefront footer; the order page has its own, store/taste-venue.js).
export const rememberLine = () => objected() ? '' :
  `<p class="taste-line small muted"><span data-t="tasteVenueLine"></span> · ${ui.button({ variant: 'ghost', label: { t: 'tasteVenueOff' }, id: 'tasteOff', cls: 'linky', attrs: { data: { tour: 'menu.tasteOff' } } })}</p>`;
export function wireRememberLine(){
  const b = $('#tasteOff'); if (!b) return;
  b.onclick = async () => {
    await objectVenue(history().filter(x => x?.id && x?.t).map(x => ({ id: x.id, token: x.t })), API);
    toast(t('tasteVenueOffDone'));
    b.closest('.taste-line')?.remove();
  };
}
