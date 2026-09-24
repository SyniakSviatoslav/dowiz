// THE ROOM: the floor staff's app -- waiter and Counter-Manager at a venue.
// Sign in, see every open table, open a round's sheet, change it, take money,
// and (Counter-Manager) run the till. The kitchen has its own screen.
//
// WHAT IS WHERE: `logic.js` pure rules (tested), `net.js` the wire and the
// outbox, `sheet.js` a round, `menu.js` adding to it, `pay.js` payments,
// `till.js` + `till-view.js` the drawer, `i18n.js` the words.
//
// READS SAY THEIR AGE. The room is read every 20 s while the screen is
// visible; the last answer is kept and, when the network is gone, drawn with
// how old it is -- never presented as now.
import { esc, money, parseCaps, canTill, sittingDue, slugOfHost, ageOf, canMoveSitting } from './logic.js';
import { api, write, session, lastRoom, OUT, hooks } from './net.js';
import { t, lang, statusWord, intlLocale, nextLang, retranslate } from './i18n.js';
import { renderRound, bindRound, amend } from './sheet.js';
import { loadMenu, renderAdd, bindAdd } from './menu.js';
import { renderPay, bindPay } from './pay.js';
import { renderOpen, bindOpen } from './open.js';
import { renderTransfer, bindTransfer, renderMoveSitting, bindMoveSitting } from './transfer.js';
import { renderTillScreen, bindTill, lastTill, keep as keepTill } from './till.js';
import { visible } from './till-view.js';
import { renderFloor, bindFloor, loadFloor } from './floor.js';
import { safeGet, safeSet } from '../store/storage.js';
import { sittingHasGuest } from './guest.js';

const $ = s => document.querySelector(s);
const POLL_MS = 20000;

const S = {
  view: 'login', claiming: false, loc: null, role: null, caps: new Set(),
  slug: slugOfHost(location.hostname, location.search),
  currency: safeGet('dw_room_currency') || null,
  sittings: [], at: 0, live: false, sittingId: null, roundId: null,
  known: {}, queued: [], till: null, menu: null, floor: null, floorPick: null,
};

const c = {
  S, t, api, statusWord, lang, locale: intlLocale,
  write: (path, body, tag) => write(path, body, { tag, signedOut }),
  render: () => render(), reload: () => loadRoom(), toast,
  remember: ({ currency }) => { if (currency) safeSet('dw_room_currency', currency); },
  tableOf: r => r?.fulfilment?.table || sitting()?.table || '',
  sitting: () => sitting(),
};

const sitting = () => S.sittings.find(s => s.sitting_id === S.sittingId) || null;
const round = () => sitting()?.rounds?.find(r => r.id === S.roundId) || null;

function toast(msg) {
  const el = $('#toast');
  el.textContent = msg;
  el.hidden = false;
  clearTimeout(toast.t); toast.t = setTimeout(() => { el.hidden = true; }, 3500);
}

// ── session ─────────────────────────────────────────────────────────────────
function adopt(s) {
  S.loc = s.staff.locationId; S.role = s.staff.role; S.caps = parseCaps(s.staff.caps);
  S.till = lastTill(S.loc);
}
function signedOut() {
  session.set(null); lastRoom.clear(); OUT.forget();
  Object.assign(S, { view: 'login', sittings: [], at: 0, loc: null, caps: new Set(), known: {}, floor: null, floorPick: null });
  render();
}

async function onLogin(ev) {
  ev.preventDefault();
  const f = ev.target, btn = f.querySelector('button[type="submit"]');
  const body = { email: f.elements.email.value.trim(), password: f.elements.password.value };
  if (S.claiming) body.code = f.elements.code.value.trim();
  btn.disabled = true;
  try {
    const d = await api(S.claiming ? '/staff/claim' : '/staff/login', { method: 'POST', body });
    session.set(d); adopt(d);
    S.view = 'room';
    render(); loadRoom(); loadMenu(c);
  } catch (e) {
    toast(e.offline ? t('offline') : e.message || t('error'));
    btn.disabled = false;
  }
}

// ── the room read ───────────────────────────────────────────────────────────
async function loadRoom() {
  if (!S.loc || S.role === 'kitchen') return;
  try {
    const d = await api(`/staff/room?location_id=${encodeURIComponent(S.loc)}`, { signedOut });
    S.sittings = carry(d?.sittings || []); S.at = Date.now(); S.live = true;
    lastRoom.set(S.loc, S.sittings, S.at);
  } catch (e) {
    S.live = false;
    if (!e.offline) toast(e.message || t('error'));
    if (!S.at) { const was = lastRoom.get(S.loc); if (was) { S.sittings = was.sittings; S.at = was.at; } }
  }
  render();
}

/// The room card has no `payments` list (see HAND-BACK); a round this phone
/// just took money on keeps the list the pay answer carried, for as long as
/// the server's version of the round is the same one.
function carry(sittings) {
  for (const s of sittings) for (const r of s.rounds || []) {
    const k = S.known[r.id];
    if (!Array.isArray(r.payments) && k && k.seq === r.seq) r.payments = k.payments;
  }
  return sittings;
}
function learn() {
  const r = round();
  if (r && Array.isArray(r.payments)) S.known[r.id] = { seq: r.seq, payments: r.payments };
}

// ── drawing ─────────────────────────────────────────────────────────────────
function hud() {
  const a = ageOf(Date.now() - S.at);
  $('#syncText').textContent = S.at ? `${S.live ? t('live') : t('offline')} · ${t('age' + a.unit.toUpperCase()).replace('{n}', a.n)}` : t(S.live ? 'live' : 'offline');
  $('#syncTag').classList.toggle('warn', !S.live);
  const n = S.queued.length;
  $('#outboxTag').hidden = n === 0;
  $('#outboxText').textContent = t('queuedN').replace('{n}', n);
  $('#langBtn').textContent = lang().toUpperCase();
}

function renderLogin() {
  return `<h1>dowiz</h1><p class="muted">${esc(t('loginLine'))}</p>
    <form data-form="login" class="card-form">
      <label>${esc(t('email'))}<input type="email" name="email" autocomplete="username" required></label>
      ${S.claiming ? `<label>${esc(t('claimCode'))}<input name="code" autocomplete="one-time-code" required></label>` : ''}
      <label>${esc(t('password'))}<input type="password" name="password" autocomplete="${S.claiming ? 'new-password' : 'current-password'}" minlength="${S.claiming ? 8 : 1}" required></label>
      <button class="cta" type="submit">${esc(t(S.claiming ? 'claim' : 'signIn'))}</button>
      <button class="btn ghost" type="button" data-act="claimToggle">${esc(t(S.claiming ? 'haveAccount' : 'haveCode'))}</button>
    </form>`;
}

function renderRoom() {
  const loc = intlLocale();
  const cards = S.sittings.map(s => {
    const due = sittingDue(s);
    const n = (s.rounds || []).length;
    const st = (s.rounds || []).map(r => `<span class="status st-${esc(r.status)}">${esc(statusWord(r.status))}</span>`).join('');
    return `<li><button class="card" data-act="sit" data-id="${esc(s.sitting_id)}">
      <span class="tbl">${esc(t('table'))} ${esc(s.table || '—')}</span>
      <span class="meta">${n} ${esc(t('rounds'))} ${st}${sittingHasGuest(s) ? ` <span class="tag warn">${esc(t('guestWaiting'))}</span>` : ''}</span>
      <span class="due">${esc(t('due'))} ${money(due, S.currency, loc)}</span></button></li>`;
  }).join('');
  return `<div class="bar"><span class="chip">${esc(t(S.role || 'waiter'))}</span><span class="sp"></span>
      ${S.caps.has('take_orders') ? `<button class="btn" data-act="open">${esc(t('openTable'))}</button>` : ''}
      ${canTill(S.caps) ? `<button class="btn" data-act="till">${esc(t('till'))}</button>` : ''}
      ${S.caps.has('take_orders') ? `<button class="btn" data-act="floor">${esc(t('floor'))}</button>` : ''}
      <button class="btn" data-act="refresh">${esc(t('refresh'))}</button></div>
    <h2>${esc(t('room'))}</h2>
    ${S.role === 'kitchen' ? `<p class="muted">${esc(t('kitchenNoRoom'))}</p>` : cards ? `<ul class="tables">${cards}</ul>` : `<p class="muted">${esc(S.at ? t('noOrders') : t('loading'))}</p>`}
    <button class="btn ghost" data-act="signout">${esc(t('signOut'))}</button>`;
}

/// A sitting with more than one round lists them; one round opens directly.
function renderSitting(s) {
  const loc = intlLocale();
  return `<div class="bar"><button class="btn" data-act="back">← ${esc(t('back'))}</button></div>
    <h2>${esc(t('table'))} ${esc(s.table || '—')}</h2>
    <ul class="tables">${(s.rounds || []).map((r, i) => `<li><button class="card" data-act="round" data-id="${esc(r.id)}">
      <span class="tbl">#${i + 1}</span><span class="status st-${esc(r.status)}">${esc(statusWord(r.status))}</span>
      <span class="due">${money(r.total || 0, S.currency, loc)}</span></button></li>`).join('')}</ul>
    ${canMoveSitting(S.caps, s) ? `<div class="acts"><button class="btn" data-act="moveSit">${esc(t('moveSitting'))}</button></div>` : ''}`;
}

function render() {
  const root = $('#app');
  root.onclick = root.onsubmit = root.oninput = null;
  learn();
  hud();
  const r = round(), s = sitting();
  if (!session.get()) S.view = 'login';
  else if (['round', 'add', 'pay', 'transfer'].includes(S.view) && !r) S.view = s ? 'sitting' : 'room';
  else if (['sitting', 'moveSit'].includes(S.view) && !s) S.view = 'room';
  if (S.view === 'round') { root.innerHTML = renderRound(c, r); return bindRound(c, root, r); }
  if (S.view === 'add') { root.innerHTML = renderAdd(c); return bindAdd(c, root, r, amend); }
  if (S.view === 'pay') { root.innerHTML = renderPay(c, r); return bindPay(c, root, r); }
  if (S.view === 'transfer') { root.innerHTML = renderTransfer(c, r); return bindTransfer(c, root, r); }
  if (S.view === 'moveSit') { root.innerHTML = renderMoveSitting(c, s); return bindMoveSitting(c, root, s); }
  if (S.view === 'open' && S.caps.has('take_orders')) { root.innerHTML = renderOpen(c); return bindOpen(c, root, () => { S.view = 'room'; render(); }); }
  if (S.view === 'till' && canTill(S.caps)) { root.innerHTML = renderTillScreen(c); return bindTill(c, root); }
  if (S.view === 'floor' && S.caps.has('take_orders')) { root.innerHTML = renderFloor(S.floor, t, S.caps, S.floorPick); return bindFloor(c, root, () => { S.view = 'room'; render(); }); }
  if (S.view === 'login') { root.innerHTML = renderLogin(); }
  else if (S.view === 'sitting') root.innerHTML = renderSitting(s);
  else { S.view = 'room'; root.innerHTML = renderRoom(); }
  root.onsubmit = ev => { if (ev.target.dataset.form === 'login') onLogin(ev); };
  root.onclick = ev => {
    const b = ev.target.closest('[data-act]');
    if (!b) return;
    const act = b.dataset.act, id = b.dataset.id;
    if (act === 'claimToggle') { S.claiming = !S.claiming; return render(); }
    if (act === 'refresh') return loadRoom();
    if (act === 'signout') return signedOut();
    if (act === 'till') { S.view = 'till'; return render(); }
    if (act === 'floor') { S.view = 'floor'; S.floorPick = null; render(); return loadFloor(c); }
    if (act === 'open') { S.view = 'open'; S.basket = {}; return render(); }
    if (act === 'back') { S.view = 'room'; return render(); }
    if (act === 'sit') {
      S.sittingId = id;
      const rs = sitting()?.rounds || [];
      if (rs.length === 1) { S.roundId = rs[0].id; S.view = 'round'; } else S.view = 'sitting';
      return render();
    }
    if (act === 'round') { S.roundId = id; S.view = 'round'; return render(); }
    if (act === 'moveSit') { S.roundId = null; S.view = 'moveSit'; return render(); }
  };
}

// ── the outbox, told to the screen ──────────────────────────────────────────
hooks.onChange = rows => { S.queued = rows; hud(); };
hooks.onSent = (entry, payload) => {
  if (entry.tag && entry.tag.startsWith('till:') && payload) { S.till = visible(payload); keepTill(S.loc, payload); }
  loadRoom();
};
hooks.onDropped = (entry, reason) => { toast(t(reason === 'changed' ? 'queuedChanged' : 'queuedRefused')); loadRoom(); };

// ── chrome ──────────────────────────────────────────────────────────────────
function applyTheme() {
  const v = safeGet('dw_room_theme') || '';
  if (v) document.documentElement.dataset.theme = v; else delete document.documentElement.dataset.theme;
}
$('#themeBtn').onclick = () => {
  safeSet('dw_room_theme', ({ '': 'dark', dark: 'light', light: '' })[safeGet('dw_room_theme') || ''] ?? '');
  applyTheme();
};
$('#langBtn').onclick = () => { nextLang(); retranslate(); S.menu = null; if (session.get()) loadMenu(c).then(render); render(); };

if ('serviceWorker' in navigator) navigator.serviceWorker.register('/room/sw.js', { scope: '/room/' }).catch(() => {});

applyTheme();
document.documentElement.lang = lang();
retranslate();
const s0 = session.get();
if (s0) {
  adopt(s0); S.view = 'room';
  const was = lastRoom.get(S.loc);
  if (was) { S.sittings = was.sittings; S.at = was.at; }
  loadRoom(); loadMenu(c).then(() => { if (S.view === 'add') render(); });
}
render();
OUT.start();
setInterval(() => { if (document.visibilityState === 'visible' && session.get() && (S.view === 'room' || S.view === 'sitting')) loadRoom(); else if (document.visibilityState === 'visible' && session.get() && S.view === 'floor') loadFloor(c); else hud(); }, POLL_MS);
