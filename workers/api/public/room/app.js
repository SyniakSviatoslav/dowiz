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
import { parseCaps, canTill, slugOfHost, ageOf } from './logic.js';
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
import { renderLogin, renderRoom, renderSitting } from './screens.js';
import { mountVoice } from './voice.js';
import * as ui from '/lib/ui/index.js';
import { createGuide } from '/lib/guide.js';
import { createLearn, loadLessons } from '/lib/learn.js';
import { openMcp } from './mcp.js';

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

// ONE live region, in index.html from the first paint (a region created at the
// moment of the notice is not announced); /lib/ui owns the timer.
ui.useTranslator(t);
const toaster = ui.createToaster($('#toast'), { ms: 3500 });
function toast(msg) { toaster.show(msg); }

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
  const restore = ui.setBusy(btn, t('loading'));
  try {
    const d = await api(S.claiming ? '/staff/claim' : '/staff/login', { method: 'POST', body });
    session.set(d); adopt(d);
    S.view = 'room';
    render(); loadRoom(); loadMenu(c);
  } catch (e) {
    toast(e.offline ? t('offline') : e.message || t('error'));
    restore();
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
  $('#syncTag').classList.toggle('ui-chip--warning', !S.live);
  $('#syncTag').classList.toggle('ui-chip--success', S.live);
  const n = S.queued.length;
  $('#outboxTag').hidden = n === 0;
  $('#outboxText').textContent = t('queuedN').replace('{n}', n);
  $('#langBtn').textContent = lang().toUpperCase();
  const mic = $('#voiceBtn'); if (mic) mic.hidden = !session.get();
  $('#mcpBtn').hidden = !session.get();
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
  if (S.view === 'login') { root.innerHTML = renderLogin(c); }
  else if (S.view === 'sitting') root.innerHTML = renderSitting(c, s);
  else { S.view = 'room'; root.innerHTML = renderRoom(c); }
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
$('#langBtn').onclick = () => { tours = null; nextLang(); retranslate(); S.menu = null; if (session.get()) loadMenu(c).then(render); render(); };

// ── lessons (/lib/learn.js): the waiter's own, run as tours on this page ────
// The header's Learn button lists them in a /lib/ui sheet; `#learn=W7` (the
// wiki's "open in the app") starts one directly.
let tours = null;   // the lesson engine; `learn()` above is the payments memory
const learnWords = () => ({ title: t('learn'), new: t('learnNew'), done: t('learnDone'), paused: t('learnPaused'), watch: t('learnWatch'),
  writes: t('learnWrites'), steps: n => t('learnSteps').replace('{n}', n), empty: t('learnEmpty'), offline: t('learnOffline') });
async function getLearn() {
  if (tours) return tours;
  const lessons = await loadLessons();
  if (!lessons.length) return null;
  return (tours = createLearn({ role: 'waiter', lessons, lang, createGuide, toast, words: learnWords() }));
}
async function openLearn() {
  const L = await getLearn();
  if (!L) return toast(t('learnOffline'));
  const s = ui.openSheet({ title: t('learn'), body: L.renderList(), closeLabel: t('learnClose') });
  L.bindList(s.el, () => s.close('picked'));
}
$('#learnBtn').onclick = openLearn;
// Voice (room/voice.js): the mic sits before the language button, where the
// browser can recognise speech; hidden while nobody is signed in (`hud`).
mountVoice(c, $('.hud'), $('#langBtn'));
// The person's own AI agent (room/mcp.js): only once signed in, since the key is theirs.
$('#mcpBtn').onclick = () => openMcp({ api, lang, toast, role: S.role, closeLabel: t('learnClose') });
const learnHash = () => { if (/learn=/.test(location.hash)) getLearn().then(L => { if (L && L.deepLink(location.hash) === false) toast(t('learnEmpty')); }); };
addEventListener('hashchange', learnHash);

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
learnHash();
OUT.start();
setInterval(() => { if (document.visibilityState === 'visible' && session.get() && (S.view === 'room' || S.view === 'sitting')) loadRoom(); else if (document.visibilityState === 'visible' && session.get() && S.view === 'floor') loadFloor(c); else hud(); }, POLL_MS);
