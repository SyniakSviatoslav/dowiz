// THE ROOM / KITCHEN BOARD ON CANVAS (Wave CV row CV1), at /room/canvas/ beside the old room page.
//
// This file is the surface's ADAPTER: it reads the hub with the room's own wire (room/net.js:
// the same session in localStorage `dw_room_session`, the same `api()`), listens on the same
// socket as the kitchen board (lib/live.js), formats money with the one formatter (room/logic.js
// -> lib/money.js), and hands the answers to Rust as a flat feed (crates/dowiz-canvas
// src/board/feed.rs is its reader). Rust draws, lays out, hit-tests and decides what a tap means;
// what must reach the hub comes back here as an intent.
//
// Routes (the same the staff use today): GET /api/staff/kitchen (cap `advance`),
// GET /api/staff/room (cap `take_orders`), POST /api/owner/orders/:id/action (bump),
// POST /api/staff/orders/:id/kitchen-ack (seen), the same action route with reject/cancel + reason,
// POST /api/staff/login | /api/staff/claim.
//
// ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11).
// No static import: the wasm compiles before any other module is requested.
const WASM = WebAssembly.compileStreaming(fetch('/room/canvas/board.wasm'));
WASM.catch(() => {});   // start() reports it
let start, LANGS, pickLang, safeGet, safeSet;
const CORE = Promise.all([import('./loader.js'), import('/lib/langs.js'), import('/store/storage.js')]).then(([lo, la, st]) => {
  ({ start } = lo); ({ LANGS, pickLang } = la); ({ safeGet, safeSet } = st);
});

// The network stack is four module levels deep: it loads after the first frame (canvas-frame gate).
let api, session, lastRoom, toFeed, opens, ROLE, BUMPS, live, NET = null;
const net = () => NET || (NET = Promise.all([import('/room/net.js'), import('./feed.js'), import('/lib/live.js')]).then(([n, f, l]) => {
  ({ api, session, lastRoom } = n); ({ toFeed, opens, ROLE, BUMPS } = f); ({ live } = l);
}));

const POLL_MS = 20000;

const S = { s: null, sock: null, pass: false, floor: false, orders: [], sittings: [], timer: 0, toastT: 0, loading: null };
let C = null;

/// A toast: the hub's own message (`msg`), or one of the board's words (1 saved, 2 error, 3 offline).
function toast(msg, word = 2) {
  if (msg) C.ex.toast(C.put(msg)); else C.ex.toast_word(word);
  C.ask();
  clearTimeout(S.toastT);
  S.toastT = setTimeout(() => { C.ex.toast(0); C.ask(); }, 3500);
}
const langCode = () => pickLang([safeGet('dw_room_lang')], navigator.languages || [], 'sq');

function feed() {
  const locale = langCode();
  const when = ms => { try { return new Date(ms).toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' }); } catch { return ''; } };
  const n = C.put(toFeed({ orders: S.orders, sittings: S.sittings, currency: safeGet('dw_room_currency') || null, locale, when }));
  C.ex.feed(n); C.ask();
}

async function load() {
  if (!S.s || S.loading) return S.loading;
  const loc = encodeURIComponent(S.s.staff.locationId);
  const signedOut = () => signOut();
  S.loading = (async () => {
    try {
      const [k, r] = await Promise.all([
        S.pass ? api(`/staff/kitchen?location_id=${loc}`, { signedOut }) : null,
        S.floor ? api(`/staff/room?location_id=${loc}`, { signedOut }) : null,
      ]);
      if (k) S.orders = k.orders || [];
      if (r) { S.sittings = r.sittings || []; lastRoom.set(S.s.staff.locationId, S.sittings, Date.now()); }
      S.sock?.polled();
      if (!S.sock || S.sock.state !== 'live') C.ex.set_sync(2);
      feed();
    } catch (e) {
      if (e.offline) C.ex.set_sync(0); else toast(e.message);
    } finally { S.loading = null; }
  })();
  return S.loading;
}

function signedIn(s) {
  S.s = s;
  [S.pass, S.floor] = opens(s);
  C.ex.session(1, S.pass ? 1 : 0, S.floor ? 1 : 0, ROLE[s.staff.role] ?? 0);
  C.ex.set_sync(0);
  S.sock?.close();
  S.sock = live({ token: s.jwt, onEvent: () => load(), onState: st => { C.ex.set_sync(st === 'live' ? 1 : st === 'polling' ? 2 : 0); C.ask(); } });
  load();
}

function signOut() {
  session.set(null); lastRoom.clear();
  S.sock?.close(); S.sock = null; S.s = null; S.orders = []; S.sittings = [];
  C.ex.session(0, 0, 0, 0); C.ex.set_sync(0); C.ask();
}

async function post(path, body) {
  return api(path, { method: 'POST', body, signedOut: () => signOut() });
}

/// A field's current value, as Rust holds it (0 email, 1 invite code, 2 password, 3 reason).
const fieldText = f => C.ex.field_len(f) ? new TextDecoder().decode(new Uint8Array(C.ex.memory.buffer, C.ex.field_ptr(f), C.ex.field_len(f))) : '';

async function onIntent(kind, a, text) {
  const loc = S.s?.staff?.locationId;
  try {
    await net();
    if (kind === 1) {
      await post(`/owner/orders/${encodeURIComponent(text)}/action`, { location_id: loc, action: BUMPS[a] });
      navigator.vibrate?.(12); toast('', 1); return load();
    }
    if (kind === 2) { await post(`/staff/orders/${encodeURIComponent(text)}/kitchen-ack`, { location_id: loc }); return load(); }
    if (kind === 3) { safeSet('dw_room_lang', text); document.documentElement.lang = text; return feed(); }
    if (kind === 4) return load();
    if (kind === 5) return signOut();
    if (kind === 8) {
      const body = { email: fieldText(0).trim(), password: fieldText(2) };
      if (a) body.code = fieldText(1).trim();
      const d = await api(a ? '/staff/claim' : '/staff/login', { method: 'POST', body });
      session.set(d);
      return signedIn(d);
    }
    if (kind === 9) location.assign('/room/');
    if (kind === 10) {
      // Reject / cancel: the reason is required and it is the one the customer reads (kitchen.js askReason).
      const reason = fieldText(3).trim();
      await post(`/owner/orders/${encodeURIComponent(text)}/action`, { location_id: loc, action: a ? 'cancel' : 'reject', reason });
      toast('', 1); return load();
    }
  } catch (e) {
    toast(e.offline ? '' : e.message, e.offline ? 3 : 2);
    if (kind === 1 || kind === 2 || kind === 10) load();
  }
}

export async function boot(canvas = document.querySelector('canvas')) {
  await CORE;
  const theme = safeGet('dw_room_theme') || '';
  const dark = theme ? theme === 'dark' : matchMedia('(prefers-color-scheme: dark)').matches;
  const code = langCode();
  document.documentElement.lang = code;
  // Raw: room/net.js, which validates it, is not loaded yet.
  const restoring = safeGet('dw_room_session') ? 1 : 0;
  C = await start(canvas, WASM, { lang: Math.max(0, LANGS.indexOf(code)), dark: dark ? 1 : 0, restoring, onIntent });
  window.__canvas = C;
  await net();
  const s = session.get();
  if (s) {
    const was = lastRoom.get(s.staff.locationId);
    if (was) S.sittings = was.sittings;
    signedIn(s);
  } else if (restoring) { C.ex.session(0, 0, 0, 0); C.ask(); }   // expired: the login form
  setInterval(() => { if (document.visibilityState === 'visible' && S.s && (!S.sock || S.sock.due())) load(); else C.ask(); }, POLL_MS);
  // The room's own service worker (scope /room/) keeps this page, its wasm and its modules for
  // a reopen with no network (room/sw.js SHELL), the same registration room/app.js makes.
  if ('serviceWorker' in navigator) navigator.serviceWorker.register('/room/sw.js', { scope: '/room/' }).catch(() => {});
  addEventListener('online', () => load());
  addEventListener('offline', () => { C.ex.set_sync(0); C.ask(); });
  return C;
}

if (typeof document !== 'undefined' && document.querySelector && document.querySelector('canvas')) {
  window.__ready = boot().then(() => true, e => { window.__err = String(e && e.stack || e); console.error(e); return false; });
}
