// THE ROOM / KITCHEN BOARD ON CANVAS (Wave CV rows CV1, CV1b): the surface's adapter. Routes and
// reasons: crates/dowiz-canvas src/lib.rs `THE HOST FILES`. ASCII QUOTES ONLY (rule 11).
import { arrivals, makeChime } from './chime.js';
const WASM = WebAssembly.compileStreaming(fetch('/room/canvas/board.wasm'));
WASM.catch(() => {});   // start() reports it
let start, safeGet, safeSet;
const CORE = Promise.all([import('./loader.js'), import('/store/storage.js')]).then(([lo, st]) => {
  ({ start } = lo); ({ safeGet, safeSet } = st);
});

let api, write, session, lastRoom, toFeed, opens, ROLE, BUMPS, W, ages, live, NET = null, TB = null;
const net = () => NET || (NET = Promise.all([import('/room/net.js'), import('./feed.js'), import('/lib/live.js')]).then(([n, f, l]) => {
  ({ api, write, session, lastRoom } = n); ({ toFeed, opens, ROLE, BUMPS, W, ages } = f); ({ live } = l);
}));

const POLL_MS = 20000;

const S = { s: null, sock: null, pass: false, floor: false, orders: [], sittings: [], timer: 0, toastT: 0, loading: null, known: null };
let C = null, code = 'sq', CHIME = null;

function toast(word, msg) {
  if (word) C.ex.toast_word(W(word)); else C.ex.toast(C.put(msg || ''));
  C.ask();
  clearTimeout(S.toastT);
  S.toastT = setTimeout(() => { C.ex.toast(0); C.ask(); }, 3500);
}
function feed() {
  const locale = code;
  const when = ms => { try { return new Date(ms).toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' }); } catch { return ''; } };
  const n = C.put(toFeed({ orders: S.orders, sittings: S.sittings, currency: safeGet('dw_room_currency') || null, locale, when }));
  C.ex.feed(n); C.ask();
  const { ids, fresh } = arrivals(S.known, S.orders);
  S.known = ids;
  CHIME?.ring(fresh);   // CV8a: a new ticket rings (after a gesture, and only if the chime is on)
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
      TB?.draw();
    } catch (e) {
      if (e.offline) C.ex.set_sync(0); else toast('', e.message);
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
  if (S.pass) api(`/owner/settings?location_id=${encodeURIComponent(s.staff.locationId)}`).then(d => { const a = ages(d); if (a) { C.ex.set_ages(a[0], a[1]); C.ask(); } }, () => {});
}

function signOut() {
  TB?.close();
  session.set(null); lastRoom.clear();
  S.sock?.close(); S.sock = null; S.s = null; S.orders = []; S.sittings = []; S.known = null;
  C.ex.session(0, 0, 0, 0); C.ex.set_sync(0); C.ask();
}

async function post(path, body) {
  return api(path, { method: 'POST', body, signedOut: () => signOut() });
}

const fieldText = f => C.ex.field_len(f) ? new TextDecoder().decode(new Uint8Array(C.ex.memory.buffer, C.ex.field_ptr(f), C.ex.field_len(f))) : '';

const host = () => ({ C, api, toast, reload: load, sittings: () => S.sittings, loc: () => S.s?.staff?.locationId, caps: () => S.s?.staff?.caps,
  write: (path, body, tag) => write(path, body, { tag, signedOut: signOut }), locale: () => code, field: fieldText,
  setField: (f, v) => C.ex.field_set(f, C.put(v)), currency: () => safeGet('dw_room_currency') || null, remember: v => safeSet('dw_room_currency', v) });

const MODES = ['', 'dark', 'light'];
function theme() {
  const v = safeGet('dw_room_theme') || '';
  C.ex.set_theme(Math.max(0, MODES.indexOf(v)), v ? v === 'dark' : matchMedia('(prefers-color-scheme: dark)').matches);
  C.ask();
  return v;
}

async function onIntent(kind, a, text) {
  const loc = S.s?.staff?.locationId;
  try {
    await net();
    if (kind === 1) {
      await post(`/owner/orders/${encodeURIComponent(text)}/action`, { location_id: loc, action: BUMPS[a] });
      navigator.vibrate?.(12); toast('Saved'); return load();
    }
    if (kind === 2) { await post(`/staff/orders/${encodeURIComponent(text)}/kitchen-ack`, { location_id: loc }); return load(); }
    if (kind === 3) { safeSet('dw_room_lang', text); document.documentElement.lang = code = text; feed(); return TB?.draw(); }
    if (kind === 4) return load();
    if (kind === 5) return signOut();
    if (kind === 8) {
      const body = { email: fieldText(0).trim(), password: fieldText(2) };
      if (a) body.code = fieldText(1).trim();
      const d = await api(a ? '/staff/claim' : '/staff/login', { method: 'POST', body });
      session.set(d);
      return signedIn(d);
    }
    if (kind === 9) { TB = await import('./table.js'); return TB.open(host(), text); }
    if (kind === 11) return TB?.act(a, text);
    if (kind === 12) return TB?.enter(a);
    if (kind === 14) { C.ex.set_sound(CHIME.toggle() ? 1 : 0); return C.ask(); }
    if (kind === 13) {
      safeSet('dw_room_theme', MODES[(MODES.indexOf(safeGet('dw_room_theme') || '') + 1) % 3]);
      return toast(['ThemeSystem', 'ThemeDark', 'ThemeLight'][MODES.indexOf(theme())]);
    }
    if (kind === 10) {
      const reason = fieldText(3).trim();
      await post(`/owner/orders/${encodeURIComponent(text)}/action`, { location_id: loc, action: a ? 'cancel' : 'reject', reason });
      toast('Saved'); return load();
    }
  } catch (e) {
    toast(e.offline ? 'Offline' : '', e.message);
    if (kind === 1 || kind === 2 || kind === 10) load();
  }
}

export async function boot(canvas = document.querySelector('canvas')) {
  await CORE;
  const mode = safeGet('dw_room_theme') || '';
  const dark = mode ? mode === 'dark' : matchMedia('(prefers-color-scheme: dark)').matches;
  const restoring = safeGet('dw_room_session') ? 1 : 0;
  C = await start(canvas, WASM, { langs: [safeGet('dw_room_lang'), ...(navigator.languages || [])], dark: dark ? 1 : 0, restoring, onIntent,
    onInput: f => { if (f >= 4) TB?.input(); } });
  document.documentElement.lang = code = C.code;
  // CV8a: the chime. Its AudioContext waits for the first pointer or key event (chime.js).
  CHIME = makeChime({ AudioCtx: window.AudioContext || window.webkitAudioContext, vibrate: n => navigator.vibrate?.(n), store: { get: safeGet, set: safeSet } });
  for (const e of ['pointerdown', 'keydown']) addEventListener(e, () => CHIME.gesture(), { capture: true, passive: true });
  C.ex.set_sound(CHIME.on ? 1 : 0);
  window.__canvas = C;
  theme();
  matchMedia('(prefers-color-scheme: dark)').addEventListener?.('change', theme);
  await net();
  const s = session.get();
  if (s) {
    const was = lastRoom.get(s.staff.locationId);
    if (was) S.sittings = was.sittings;
    signedIn(s);
  } else if (restoring) { C.ex.session(0, 0, 0, 0); C.ask(); }   // expired: the login form
  setInterval(() => { if (document.visibilityState === 'visible' && S.s && (!S.sock || S.sock.due())) load(); else C.ask(); }, POLL_MS);
  if ('serviceWorker' in navigator) navigator.serviceWorker.register('/room/sw.js', { scope: '/room/' }).catch(() => {});
  addEventListener('online', () => load());
  addEventListener('offline', () => { C.ex.set_sync(0); C.ask(); });
  return C;
}

if (typeof document !== 'undefined' && document.querySelector && document.querySelector('canvas')) {
  window.__ready = boot().then(() => true, e => { window.__err = String(e && e.stack || e); console.error(e); return false; });
}
