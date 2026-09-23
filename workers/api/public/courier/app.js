// Courier app. One job on screen at a time, because the person holding this
// phone is on a scooter. Every status change is the server's answer -- there is
// no ordered list of statuses in this file.
//
// Design pass 2026-09-15: the API surface is untouched (same paths, same
// payloads, same field names). What changed is the skin: project tokens, a
// dark theme that follows the phone, Tabler icons, a one-CTA task list, the
// cash handover as an in-sheet form instead of window.prompt, and the plan's
// "one incoming ripple + ping" on a new task.
import { create as vcreate, speak, supported as vsupported } from '/lib/voice.js';
import { createGuide } from '/lib/guide.js';
import { createOutbox, newKey } from '/lib/outbox.js';
import { t, lang, LANGS, setLang, nextLang, retranslate, intlLocale, voiceLocale } from '/courier/i18n.js';

const API = '/api';
const $ = (s, r = document) => r.querySelector(s);
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
// MONEY COMES FROM ONE PLACE NOW. This line was a second copy of a rule that
// also lived on the storefront, with `currency:'ALL'` hardcoded -- so a venue
// trading in anything else showed lek here and the right currency to its
// customers. `MoneyFmt` is reassigned once the venue's currency and the
// viewer's chosen display currency are known; until then it renders the
// venue-neutral default rather than a wrong symbol.
import * as Money from '/lib/money.js';
let MoneyBase = 'ALL', MoneyDisplay = 'ALL', MoneyRates = null;
let MoneyFmt = Money.formatter({ base: MoneyBase, display: MoneyDisplay, rates: null, locale: 'sq' });
const money = n => MoneyFmt(n);
/// Called once the venue is known, and again when the viewer switches currency.
async function setCurrency(base, display) {
  MoneyBase = base || MoneyBase;
  MoneyDisplay = display || MoneyBase;
  if (MoneyDisplay !== MoneyBase && (!MoneyRates || MoneyRates.base !== MoneyBase)) {
    MoneyRates = await Money.loadRates(MoneyBase);
  }
  MoneyFmt = Money.formatter({ base: MoneyBase, display: MoneyDisplay, rates: MoneyRates, locale: 'sq' });
  Money.remember(MoneyDisplay);
}
const short = id => esc(String(id).slice(0, 8));
const icon = (name, cls = '') => `<i class="ti ti-${name} i ${cls}" aria-hidden="true"></i>`;

// THE LAST ANSWER THE HUB GAVE, for the phone that reopens underground. It
// holds the courier's own run (an address and a phone number), so it lives
// only as long as the session: `signedOut` clears it, and it is never shown
// once it is older than a shift.
const LAST_KEY = 'dw_c_last';
const LAST_MAX_AGE_MS = 12 * 60 * 60 * 1000;
const last = {
  save(d){ try { localStorage.setItem(LAST_KEY, JSON.stringify({ at: Date.now(), d })); } catch {} },
  read(){
    try {
      const v = JSON.parse(localStorage.getItem(LAST_KEY) || 'null');
      return v && v.d && Date.now() - v.at < LAST_MAX_AGE_MS ? v : null;
    } catch { return null; }
  },
  clear(){ try { localStorage.removeItem(LAST_KEY); } catch {} },
};
const store = {
  get t(){ try { return localStorage.getItem('dw_c_jwt'); } catch { return null; } },
  set t(v){ try { v ? localStorage.setItem('dw_c_jwt', v) : localStorage.removeItem('dw_c_jwt'); } catch {} },
  // Theme preference is a display setting, not PII: '' (follow the phone), 'dark' or 'light'.
  get theme(){ try { return localStorage.getItem('dw_c_theme') || ''; } catch { return ''; } },
  set theme(v){ try { v ? localStorage.setItem('dw_c_theme', v) : localStorage.removeItem('dw_c_theme'); } catch {} },
};
// `phase` exists because `onShift:false` is a LIE until the first load answers.
// Rendering it as fact told a courier their shift was closed -- the one screen
// state that makes them stop working -- while the request was still in flight.
let S = { onShift:false, mine:[], available:[], shift:null, watchId:null, wake:null, booted:false,
          queued:[],
          phase:'loading', error:null,
          loadedOnce:false, sel:null, cashFor:null, moving:false, hiddenAt:0, inflight:0 };

// ── theme ──
// Default is the phone's own setting. A courier's OS already goes dark at
// sunset on most Androids, and that is exactly the signal wanted: light in the
// midday sun (the panel's full brightness behind the ink), dark on the night
// shift (no white sheet in a dark-adapted eye). The button exists because cheap
// handsets do not always schedule dark mode.
const mqDark = matchMedia('(prefers-color-scheme: dark)');
const isDark = () => (store.theme ? store.theme === 'dark' : mqDark.matches);
const THEME_UI = {
  '':      { icon:'sun-moon', label:'themeSystem' },
  'dark':  { icon:'moon',     label:'themeDark' },
  'light': { icon:'sun',      label:'themeLight' },
};
function applyTheme(){
  const p = store.theme;
  if (p) document.documentElement.dataset.theme = p; else delete document.documentElement.dataset.theme;
  const ui = THEME_UI[p] || THEME_UI[''];
  $('#themeBtn').innerHTML = icon(ui.icon);
  $('#themeBtn').setAttribute('aria-label', t(ui.label)); $('#themeBtn').title = t('theme');
  // The two media-scoped theme-color metas follow the OS; a forced theme has
  // to drive the browser chrome itself, from the token the theme resolved to.
  if (p) document.querySelectorAll('meta[name="theme-color"]').forEach(m => m.setAttribute('content', cssVar('--brand-surface')));
  syncMapStyle();
}
$('#themeBtn').onclick = () => { store.theme = ({ '':'dark', dark:'light', light:'' })[store.theme] ?? ''; applyTheme(); };
mqDark.addEventListener('change', () => { if (!store.theme) syncMapStyle(); });

// ── map ──
// OpenFreeMap: no key, no quota, no account. Both style URLs were checked to
// answer 200 on 2026-09-15; `dark` is the night map, `bright` the day one.
const STYLE = { light:'https://tiles.openfreemap.org/styles/bright', dark:'https://tiles.openfreemap.org/styles/dark' };
let map = null, meMarker = null, dropMarker = null, mapDark = null;
const cssVar = n => getComputedStyle(document.documentElement).getPropertyValue(n).trim();
// THE MAP LIBRARY ARRIVES WHEN THERE IS A MAP, not when the page loads.
//
// maplibre-gl is 932 KB raw / 245 KB gzipped -- measured, not estimated -- and
// it was being fetched on every single page load, including the login screen
// and the off-shift screen, neither of which contains a map. A courier opening
// this on a phone at the edge of coverage paid for it every time before seeing
// anything at all.
//
// SELF-HOSTED, and that is not about the bytes -- it is the same 932 KB either
// way. It is about who has to be reachable for a courier to see where they are
// going. From a CDN, a blocked or slow jsdelivr means no map on a delivery
// screen, and the venue can do nothing about it. From here it is one origin,
// the same one that just served the app, with no extra DNS or TLS handshake.
//
// It also removes the last third party any surface touched: every customer and
// every courier now talks to exactly one host.
//
// Loaded once, cached by the promise so two callers race safely, and its
// stylesheet comes with it rather than sitting in the document head.
let mapLibPromise = null;
function loadMapLibrary(){
  if (window.maplibregl) return Promise.resolve();
  if (mapLibPromise) return mapLibPromise;
  mapLibPromise = new Promise((resolve, reject) => {
    const css = document.createElement('link');
    css.rel = 'stylesheet';
    css.href = '/lib/map/maplibre-gl.css';
    document.head.appendChild(css);
    const js = document.createElement('script');
    js.src = '/lib/map/maplibre-gl.js';
    js.async = true;
    js.onload = () => resolve();
    // A failed load must REJECT rather than hang: the delivery screen has to
    // fall back to the address and the call button, and it can only do that if
    // it is told. A courier with no map still has a job.
    js.onerror = () => { mapLibPromise = null; reject(new Error('map unavailable')); };
    document.head.appendChild(js);
  });
  return mapLibPromise;
}

async function initMap(){
  if (map) return;
  try { await loadMapLibrary(); }
  catch { return; }
  if (map || !window.maplibregl) return;
  mapDark = isDark();
  // A phone whose WebGL refuses the canvas gets the sheet without the map,
  // not an uncaught error over it (measured on emulated dpr-3 devices).
  try {
    map = new maplibregl.Map({
      container: 'map',
      style: STYLE[mapDark ? 'dark' : 'light'],
      center: [19.4449964, 41.315347],   // the venue, until a fix arrives
      zoom: 13, attributionControl: { compact: true }, failIfMajorPerformanceCaveat: false,
    });
  } catch (e) { map = null; return; }
  map.on('error', e => { if (map && !map.loaded()) { try { map.remove(); } catch {} map = null; } });
  map.addControl(new maplibregl.NavigationControl({ showCompass:false }), 'top-right');
}
function syncMapStyle(){
  if (!map) return;
  const d = isDark();
  if (mapDark === d) return;
  mapDark = d;
  map.setStyle(STYLE[d ? 'dark' : 'light']);   // markers are DOM; they survive a style swap
}
function sheetHeight(){ const s = $('#sheet'); return s ? Math.round(s.getBoundingClientRect().height) : 288; }
function markMe(lon, lat){
  S.me = { lat, lon };
  const line = $('#etaLine'); if (line && S.mine[0]) line.textContent = etaText(S.mine[0]);
  if (!map) return;
  if (!meMarker) {
    const el = document.createElement('div');
    el.className = 'me-marker';
    meMarker = new maplibregl.Marker({ element: el });
  }
  meMarker.setLngLat([lon, lat]).addTo(map);
}
// The drop pin. This was dead code with a note saying the task card carried no
// coordinates -- true until the delivery-zone work put `lat_udeg`/`lon_udeg` on
// the address. It is called from `renderActive` now.
function markDrop(lon, lat){
  if (!map) return;
  if (!dropMarker) dropMarker = new maplibregl.Marker({ color: cssVar('--brand-primary') });
  dropMarker.setLngLat([lon, lat]).addTo(map);
  if (meMarker) {
    const b = new maplibregl.LngLatBounds();
    b.extend(meMarker.getLngLat()); b.extend([lon, lat]);
    // The sheet covers the bottom of the map: pad for it so no pin sits under it.
    map.fitBounds(b, { padding: { top: 112, bottom: sheetHeight() + 24, left: 48, right: 48 }, maxZoom: 16 });
  } else map.setCenter([lon, lat]);
}

// The sheet is content-sized; the toast and the burst canvas sit relative to
// its top edge, so its height is published as a CSS variable when it changes.
new ResizeObserver(() => {
  document.documentElement.style.setProperty('--sheet-h', sheetHeight() + 'px');
}).observe($('#sheet'));

// ── the Sea, as far as this surface gets one ──
// The plan is specific for the courier: during a run "the Sea IS the map", and
// on shift the sea is calm with "one incoming ripple + ping" on a new task.
// So there is no ambient field over the map -- a courier's battery is the
// scarcest thing on a shift -- only event bursts, born at the spectral edge
// (the canvas is a band centred on the seam and the library emits from its
// centre). particle-cloud stops its own frame loop when the burst dies, and
// bursts are skipped entirely while the courier is moving.
let sea = null, seaTried = false;
async function initSea(){
  if (sea || seaTried) return;
  seaTried = true;
  try {
    const { createParticleCloud } = await import('/lib/particle-cloud.js');
    sea = createParticleCloud();
    sea.init($('#sea'));
    sea.setReducedMotion(matchMedia('(prefers-reduced-motion: reduce)').matches);
    addEventListener('resize', () => sea && sea.resize(), { passive:true });
  } catch { sea = null; }
}
function seaEvent(kind, n){ if (!sea || S.moving) return; try { sea.burst(kind, n); } catch {} }

// Sound + vibration on a new task (product-context §14). The AudioContext is
// created on the first touch, because that is the only moment a browser lets
// a page make a sound; nothing else about the ping depends on it.
let audio = null;
addEventListener('pointerdown', () => {
  if (audio) return;
  try { audio = new (window.AudioContext || window.webkitAudioContext)(); } catch { audio = null; }
}, { once:true, passive:true });
function ping(){
  try { if (navigator.vibrate) navigator.vibrate([120, 60, 120]); } catch {}
  if (!audio) return;
  try {
    if (audio.state === 'suspended') audio.resume();
    const t = audio.currentTime;
    [[880, 0], [1320, .14]].forEach(([f, d]) => {
      const o = audio.createOscillator(), g = audio.createGain();
      o.type = 'sine'; o.frequency.value = f;
      g.gain.setValueAtTime(.0001, t + d);
      g.gain.exponentialRampToValueAtTime(.35, t + d + .01);
      g.gain.exponentialRampToValueAtTime(.0001, t + d + .13);
      o.connect(g).connect(audio.destination);
      o.start(t + d); o.stop(t + d + .14);
    });
  } catch {}
}

function toast(m, ic = 'info-circle'){
  const el = $('#toast');
  el.innerHTML = `${icon(ic)}<span>${esc(m)}</span>`;
  el.hidden = false;                       // hidden → shown animates from @starting-style
  clearTimeout(toast._t); toast._t = setTimeout(() => { el.hidden = true; }, 3000);
}

// `attend` marks a request the courier is waiting on: the spectral edge sweeps
// while it is in flight. The GPS heartbeat and the 12s poll do not attend --
// they are background, and an edge that never rests says nothing.
//
// THE ERROR SAYS WHICH KIND OF FAILURE IT WAS. Every caller used to see one
// `Error` with a message, and a message cannot be told apart: "HTTP 409" and
// "Failed to fetch" both arrived as a toast. The outbox needs the difference,
// because a request the network never carried is a tap to KEEP and a request
// the server refused is a tap to report. `e.offline` is the first, `e.status`
// is the second; the message is unchanged, so nothing that reads it broke.
async function api(path, opts = {}){
  const { attend, ...rest } = opts;
  if (attend) { S.inflight++; $('#sheet').classList.add('attending'); }
  try {
    let r;
    try {
      r = await fetch(API + path, { ...rest,
        headers: { 'content-type':'application/json', ...(rest.headers || {}),
                   ...(store.t ? { authorization:'Bearer ' + store.t } : {}) } });
    } catch (e) {
      const err = new Error(String(e?.message || e)); err.offline = true; throw err;
    }
    if (r.status === 401) { signedOut(); throw new Error('unauthorised'); }
    if (!r.ok) {
      let m = 'HTTP ' + r.status; try { const d = await r.json(); m = d.error || d.message || m; } catch {}
      const err = new Error(m); err.status = r.status; throw err;
    }
    return r.status === 204 ? null : r.json();
  } finally {
    if (attend && --S.inflight <= 0) { S.inflight = 0; $('#sheet').classList.remove('attending'); }
  }
}

// ── the outbox: a tap that outlives the signal ──────────────────────────────
//
// THE DEFECT. Every action on this screen was a `fetch` and nothing else. A
// courier in a lift, a basement or a tram taps "picked up", `fetch` rejects,
// a toast appears for three seconds and the tap is gone -- while the kitchen
// goes on waiting for a courier who is already on the road. The offline panel
// with its Retry button was the whole of the offline story, and a Retry button
// is only useful to a courier who is still looking at the screen.
//
// THE KEY IS MINTED BEFORE THE FIRST ATTEMPT, not when the queue drains, and
// the direct attempt carries it too. A request whose RESPONSE was lost may
// well have landed; queueing that tap under a fresh key would be the duplicate
// `Idempotency-Key` exists to prevent, arriving by the back door.
//
// A QUEUED TAP NEVER LOOKS LIKE A LANDED ONE. The status chip on the order is
// still the server's answer -- nothing here advances it locally -- and the HUD
// carries a count of what this phone has not managed to send. The one thing
// the courier is told at the moment of the tap is that it was SAVED.
const OUT = createOutbox({
  // At drain time, from the store: a token that was refreshed while the phone
  // was in a pocket is the one that must go out. `null` means there is no
  // session, which pauses the drain instead of burning the entry against a 401.
  authorize: () => (store.t ? { authorization: 'Bearer ' + store.t } : null),
  onChange: rows => {
    S.queued = rows;
    drawOutbox();
    // THE SCREEN FOLLOWS THE QUEUE. Without this the courier taps, is told the
    // tap was saved, and goes on looking at the same live button -- which is
    // an invitation to tap it again. `#pback` marks an open panel (earnings,
    // history); redrawing over one would throw a courier out of what they are
    // reading, the same reason `load()` leaves it alone.
    if (S.booted && S.loadedOnce && !$('#pback')) render();
  },
  onSent: () => { load(); },
  onDropped: (entry, reason) => {
    // THE ONE THING THAT MUST NOT BE A RETRY. `courier.rs` answers 409 on an
    // illegal transition (lines 442 and 502) -- the owner cancelled, or another
    // courier took it, while this phone was underground. It is an ANSWER, so
    // the tap is dropped and the courier is told what happened to their order.
    toast(t(reason === 'changed' ? 'queuedChanged' : 'queuedRefused'), 'alert-triangle');
    load();
  },
});

function drawOutbox(){
  const tag = $('#outboxTag');
  if (!tag) return;
  const n = S.queued.length;
  tag.hidden = n === 0;
  if (n) $('#outboxText').textContent = t('queuedN').replace('{n}', n);
}

/// Is there a tap for this order still waiting to be sent?
const queuedFor = id => S.queued.find(q => q.tag && q.tag.endsWith(':' + id));

/// One courier action: try it now, keep it if the NETWORK was what failed.
///
/// A server that ANSWERED is never queued -- a 409, a 403 or a 400 is a
/// decision, and repeating it produces the same decision. Only a request the
/// network never carried is a tap this phone is still holding.
async function tapped(path, { body = null, tag } = {}){
  const key = newKey();
  try {
    const data = await api(path, { method:'POST', attend:true, body,
                                   headers: { 'idempotency-key': key } });
    return { landed: true, data };
  } catch (e) {
    if (!e.offline) throw e;
    const q = await OUT.queue(API + path, { body, tag, key });
    if (!q.ok) {
      // REFUSED, LOUDLY, WHILE THE COURIER IS STILL LOOKING. A queue that is
      // full or a browser that will not store one are both states where the
      // tap is gone; saying nothing would be the original defect with a queue
      // bolted on top of it.
      toast(t(q.reason === 'full' ? 'queueFull' : 'queueNoStore'), 'alert-circle');
      return { landed: false, queued: false };
    }
    toast(t('queuedSaved'), 'cloud-upload');
    return { landed: false, queued: true };
  }
}

// ── geolocation ──
// The design rules are explicit and they are enforced HERE as well as on the
// server: a fix worse than 100m, or a speed above 150km/h, is not data.
const MAX_ACCURACY_M = 100, MAX_SPEED_MPS = 41.67, MOVING_MPS = 1.5;
function gps(text, warn){
  const tag = $('#gpsTag');
  tag.hidden = false; $('#gpsText').textContent = text; tag.classList.toggle('warn', !!warn);
}
// ONE FIX EVERY TEN SECONDS OR TWENTY METRES, whichever comes first. The map
// shows the newest fix per courier and nothing reads the ones between, so a fix
// a second was a database row a second for nobody.
const FIX_MIN_MS = 10_000, FIX_MIN_M = 20;
function metresBetween(lat1, lon1, lat2, lon2){
  const k = Math.PI / 180, x = (lon2 - lon1) * k * Math.cos(((lat1 + lat2) / 2) * k), y = (lat2 - lat1) * k;
  return Math.sqrt(x * x + y * y) * 6371000;
}
function startTracking(){
  if (S.watchId != null || !navigator.geolocation) return;
  gps('GPS', false);
  S.watchId = navigator.geolocation.watchPosition(async pos => {
    const { latitude, longitude, accuracy, speed } = pos.coords;
    markMe(longitude, latitude);
    S.moving = speed != null && speed > MOVING_MPS;
    if (accuracy > MAX_ACCURACY_M) {
      // Say so rather than quietly sending it: a 500m fix looks like a position.
      gps(`±${Math.round(accuracy)} ${t('gpsUnit')}`, true);
      return;
    }
    if (speed != null && speed > MAX_SPEED_MPS) return;
    gps(`±${Math.round(accuracy)} ${t('gpsUnit')}`, false);
    const now = Date.now(), last = S.lastFix;
    if (last && now - last.t < FIX_MIN_MS && metresBetween(last.lat, last.lon, latitude, longitude) < FIX_MIN_M) return;
    S.lastFix = { t: now, lat: latitude, lon: longitude };
    const active = S.mine[0];
    // OVER THE SOCKET WHEN THERE IS ONE. A fix was a D1 row per fix, written
    // for a map that reads the newest one per courier and nothing else -- the
    // largest write source in the system, kept for nobody. On the socket it is
    // a message into the object's memory: no row, no request. The POST stays
    // for a courier with no socket, and it is also what keeps the 48-hour
    // audit trail the data map promises.
    const sent = S.live?.gps(Math.round(latitude * 1e6), Math.round(longitude * 1e6));
    if (sent) return;
    try {
      await api('/courier/position', { method:'POST', body: JSON.stringify({
        lat: latitude, lon: longitude, accuracy_m: accuracy,
        speed_mps: speed ?? null, order_id: active ? active.id : null }) });
    } catch {}
  }, err => {
    gps(err.code === 1 ? t('gpsDenied') : t('gpsUnavailable'), true);
  }, { enableHighAccuracy:true, maximumAge:5000, timeout:20000 });
}
// ── A SESSION THAT HAS ENDED MUST STOP ASKING ──
//
// The 401 handler used to set `store.t = null` and call `renderLogin`, and
// nothing else stopped. Everything that calls `api()` kept calling it: the
// task poll re-arms itself every 12 s on shift and 60 s off it, the GPS watch
// is never cleared outside `render()`, so a moving scooter POSTs a position on
// every accepted fix, and the socket's `onEvent` calls `load()`. Each of those
// 401s re-rendered the login screen, replacing `#app.innerHTML` -- so the
// courier's half-typed phone number and password were wiped every few seconds,
// faster than anyone types one-handed.
//
// It is not a rare state either. `COURIER_TTL_MS` is 24 h and there is no
// courier refresh route, so every courier session dies exactly a day after it
// began, in the middle of whatever they were doing.
//
// Measured: two renders and two 401s in 66 s off-shift, with both fields
// cleared the second time.
function signedOut(){
  store.t = null;
  last.clear();
  S.booted = false;
  // The unsent taps go with the session, for the reason `replica.js` forgets
  // its copy: the next courier to sign in on this phone is not the person who
  // made them, and replaying them under a new token would file one courier's
  // delivery under another's name.
  OUT.forget();
  S.queued = []; drawOutbox();
  clearTimeout(boot._i);
  stopTracking();
  try { S.live?.close(); } catch {}
  S.live = null;
  renderLogin(t('sessionOver'));
}

function stopTracking(){
  if (S.watchId != null) navigator.geolocation.clearWatch(S.watchId);
  S.watchId = null; S.moving = false; $('#gpsTag').hidden = true;
}

// The screen must not sleep mid-run. Wake Lock drops on tab-hide, so it is
// re-taken when the page comes back rather than assumed to survive.
async function keepAwake(on){
  try {
    if (on && !S.wake && 'wakeLock' in navigator) S.wake = await navigator.wakeLock.request('screen');
    if (!on && S.wake) { await S.wake.release(); S.wake = null; }
  } catch {}
}
document.addEventListener('visibilitychange', () => {
  if (document.hidden) { S.hiddenAt = Date.now(); return; }
  const away = S.hiddenAt ? Date.now() - S.hiddenAt : 0; S.hiddenAt = 0;
  if (S.onShift && S.mine.length) {
    keepAwake(true);
    // Page Visibility warning (product-context §14): in the background the
    // browser throttles the GPS watch, so a run's trace has a hole in it.
    if (away > 5000) toast(t('backgroundGps'), 'alert-triangle');
  }
  if (S.booted) load();
});

// ── login ──
function renderLogin(err){
  $('#sheet').classList.add('tall');
  $('#app').innerHTML = `<form class="login" id="loginForm" novalidate>
    <div class="login-mark" aria-hidden="true"><span>d</span></div>
    <h2>${esc(t('loginTitle'))}</h2><p class="hint2 center">${esc(t('loginLine'))}</p>
    <label for="em">${esc(t('emailOrPhone'))}</label>
    <input id="em" autocomplete="username" inputmode="email" enterkeyhint="next">
    <label for="pw">${esc(t('password'))}</label>
    <input id="pw" type="password" autocomplete="current-password" enterkeyhint="go">
    ${err ? `<div class="err" role="alert">${icon('alert-circle')}<span>${esc(err)}</span></div>` : ''}
    <button class="cta" id="go" type="submit">${icon('login')}${esc(t('signIn'))}</button>
    <button class="ghost" id="toClaim" type="button">${icon('ticket')}${esc(t('haveCode'))}</button>
    <div class="langs">${LANGS.map(l => `<button type="button" class="chip ${l === lang ? 'on' : ''}" data-l="${l}">${l.toUpperCase()}</button>`).join('')}</div>
  </form>`;
  $('#toClaim').onclick = () => renderClaim();
  for (const b of document.querySelectorAll('[data-l]')) b.onclick = () => { setLang(b.dataset.l); renderLogin(err); };
  $('#loginForm').onsubmit = async ev => {
    ev.preventDefault();
    const b = $('#go'); b.disabled = true; b.innerHTML = `${icon('loader-2')}${esc(t('signingIn'))}`;
    const v = $('#em').value.trim();
    try {
      const r = await fetch(API + '/courier/auth/login', { method:'POST',
        headers:{ 'content-type':'application/json' },
        body: JSON.stringify({ [v.includes('@') ? 'email' : 'phone']: v, password: $('#pw').value }) });
      const d = await r.json();
      if (!r.ok) throw new Error(d.error || d.message || 'HTTP ' + r.status);
      store.t = d.jwt; boot();
    } catch (e) { renderLogin(String(e.message || e)); }
  };
}

// ── claiming an invite ──
//
// The courier has no account yet: the code stands in for one, once. They choose
// their own password here, which is the point -- an owner who typed it for them
// would know it, and a shared password is not a password.
function renderClaim(err){
  $('#sheet').classList.add('tall');
  $('#app').innerHTML = `<form class="login" id="claimForm" novalidate>
    <h2>${esc(t('claimTitle'))}</h2>
    <p class="hint2">${esc(t('claimHint'))}</p>
    <label for="cph">${esc(t('yourPhone'))}</label>
    <input id="cph" type="tel" inputmode="tel" autocomplete="tel" enterkeyhint="next">
    <label for="cod">${esc(t('code'))}</label>
    <input id="cod" autocomplete="one-time-code" autocapitalize="characters"
           spellcheck="false" maxlength="16" enterkeyhint="next">
    <label for="cpw">${esc(t('choosePassword'))}</label>
    <input id="cpw" type="password" autocomplete="new-password" minlength="8" enterkeyhint="go">
    <p class="hint2">${esc(t('passwordHint'))}</p>
    ${err ? `<div class="err" role="alert">${icon('alert-circle')}<span>${esc(err)}</span></div>` : ''}
    <button class="cta" id="cgo" type="submit">${icon('check')}${esc(t('start'))}</button>
    <button class="ghost" id="toLogin" type="button">${icon('arrow-left')}${esc(t('havePassword'))}</button>
  </form>`;
  $('#toLogin').onclick = () => renderLogin();
  $('#claimForm').onsubmit = async ev => {
    ev.preventDefault();
    const b = $('#cgo'); b.disabled = true; b.innerHTML = `${icon('loader-2')}${esc(t('checking'))}`;
    try {
      const r = await fetch(API + '/courier/auth/claim', { method:'POST',
        headers:{ 'content-type':'application/json' },
        body: JSON.stringify({ phone: $('#cph').value.trim(),
                               code: $('#cod').value.trim().toUpperCase(),
                               password: $('#cpw').value }) });
      const d = await r.json();
      if (!r.ok) throw new Error(d.error || d.message || 'HTTP ' + r.status);
      // Signed in on the spot: they set the password ten seconds ago and
      // re-typing it proves nothing.
      store.t = d.jwt; boot();
    } catch (e) { renderClaim(String(e.message || e)); }
  };
}

// ── shift + tasks ──
async function load(){
  try {
    const wasOn = S.onShift, hadActive = S.mine.length > 0;
    const before = new Set(S.available.map(o => o.id));
    const d = await api('/courier/tasks');
    last.save(d); S.staleAt = 0;
    S.onShift = d.onShift; S.mine = d.mine || []; S.available = d.available || []; S.shift = d.shift;
    S.courierId = d.courierId || S.courierId;
    // "task_assigned = one incoming ripple + ping": a task that was not on the
    // last poll, arriving while the courier is on shift and free.
    const fresh = S.available.filter(o => !before.has(o.id));
    if (S.loadedOnce && wasOn && S.onShift && !hadActive && !S.mine.length && fresh.length) {
      ping(); seaEvent('order_created', 80);
    }
    S.loadedOnce = true;
    S.phase = 'ready'; S.error = null;
    // A COURIER READING A PANEL IS NOT INTERRUPTED BY THE POLL. Earnings and
    // history live in #app too; redrawing it every twelve seconds threw them
    // back to the queue mid-read. The chip still updates; the panel stays
    // until a job actually arrives or the shift ends.
    if ($('#pback') && S.onShift && !S.mine.length) { setShiftTag(); return; }
    render();
  } catch (e) {
    if (String(e.message) === 'unauthorised') return;
    // REOPENED WITH NO NETWORK: the run the courier was on is still the run
    // they are on. Show the last answer, SAID to be the last answer, rather
    // than an error panel over the address they need.
    const was = e.offline && !S.loadedOnce ? last.read() : null;
    if (was) {
      const d = was.d;
      S.onShift = d.onShift; S.mine = d.mine || []; S.available = d.available || []; S.shift = d.shift;
      S.courierId = d.courierId || S.courierId;
      S.staleAt = was.at; S.phase = 'ready'; S.error = null;
      render();
      return;
    }
    // FIRST load failing is a state; a later one is a toast. After the first
    // success the screen holds real work -- an address, a phone number, a
    // button that says delivered -- and replacing that with an error panel
    // because one poll timed out would take away the thing being used.
    if (!S.loadedOnce) { S.phase = 'error'; S.error = String(e.message || e); render(); }
    else toast(String(e.message || e), 'alert-circle');
  }
}

function setShiftTag(){
  const tag = $('#shiftTag');
  tag.classList.toggle('on', !!S.onShift);
  $('#shiftText').innerHTML = S.onShift
    ? `${esc(t('onShift'))} · ${esc(S.shift?.deliveries ?? 0)} · <span class="money">${esc(money(S.shift?.cash ?? 0))}</span>`
    : esc(t('offline'));
  if (S.staleAt) {
    const at = new Date(S.staleAt).toLocaleTimeString(intlLocale(), { hour:'2-digit', minute:'2-digit' });
    $('#shiftText').innerHTML += ` · ${esc(t('staleAsOf', { t: at }))}`;
  }
}

function render(){
  setShiftTag();
  // Signed in: the map is the point again, so the sheet gives it back its half.
  $('#sheet').classList.remove('tall');

  // BEFORE anything is claimed about the shift. A skeleton here is not
  // decoration: the alternative is asserting "you are offline" on no evidence.
  if (S.phase === 'loading' && !S.loadedOnce) {
    $('#app').innerHTML = `<div class="loadwrap" aria-busy="true" aria-label="${esc(t('loading'))}">
      <div class="skel skel-line"></div>
      <div class="skel skel-block"></div>
      <div class="skel skel-block"></div>
      <div class="skel skel-cta"></div>
    </div>`;
    return;
  }
  if (S.phase === 'error' && !S.loadedOnce) {
    $('#app').innerHTML = `<div class="empty" role="alert">${icon('plug-connected-x')}
      <b>${esc(t('noLink'))}</b>
      <span class="reason">${esc(S.error || '')}</span></div>
      <button class="cta go" id="retry" type="button">${icon('refresh')}${esc(t('retry'))}</button>`;
    $('#retry').onclick = async () => {
      S.phase = 'loading'; render(); await load();
    };
    return;
  }

  if (!S.onShift) {
    stopTracking(); keepAwake(false); S.cashFor = null;
    $('#app').innerHTML = `<div class="empty">${icon('moon-stars')}<b>${esc(t('youAreOffline'))}</b>${esc(t('offlineHint'))}</div>
      <button class="cta go" id="openShift" type="button">${icon('player-play')}${esc(t('openShift'))}</button>`;
    $('#openShift').onclick = () => setShift(true);
    return;
  }
  startTracking();
  initSea();

  // ONE job on screen. A run in hand hides everything else.
  const active = S.mine[0];
  // AN OFFER IS NOT A RUN. An order assigned to this courier that they have not
  // taken yet gets its own screen: the active screen's first control is
  // "picked up", and a courier who has not agreed to the job should not be one
  // mis-tap from telling the kitchen they have it.
  if (active && active.offerEndsMs) { keepAwake(true); return renderOffer(active); }
  if (active) { keepAwake(true); return S.cashFor && S.cashFor === active.id ? renderCash(active) : renderActive(active); }
  keepAwake(false); S.cashFor = null;

  if (!S.available.length) {
    // The ask box lives HERE and nowhere else. This app shows one job at a time
    // on purpose, and a text field on a live delivery screen would compete with
    // the address and the call button for a thumb that is on a handlebar. This
    // is the one state where the courier is standing still.
    $('#app').innerHTML = `<div class="empty">${icon('radar-2')}<b>${esc(t('noneFree'))}</b>${esc(t('noneFreeHint'))}</div>
      <div class="askrow">
        <input id="askBox" class="ask" type="text" placeholder="${esc(t('askPlaceholder'))}"
               autocomplete="off" enterkeyhint="send">
        <button class="ghost narrow" id="askGo" type="button">${icon('send')}</button>
      </div>
      <p id="answer" class="answer" hidden></p>
      <div class="row2">
        <button class="ghost" id="earn" type="button">${icon('coins')}${esc(t('myShifts'))}</button>
        <button class="ghost" id="hist" type="button">${icon('history')}${esc(t('history'))}</button>
      </div>
      <button class="ghost" id="endShift" type="button">${icon('power')}${esc(t('endShift'))}</button>`;
    $('#endShift').onclick = () => setShift(false);
    $('#earn').onclick = openEarnings;
    $('#hist').onclick = openHistory;
    bindAsk();
    return;
  }

  // Rows select; ONE CTA takes. N "Взяти" buttons would be N main actions on
  // one screen, which the courier rules forbid.
  if (!S.available.some(o => o.id === S.sel)) S.sel = S.available[0].id;
  const chosen = S.available.find(o => o.id === S.sel);
  $('#app').innerHTML = `<h2>${esc(t('readyForPickup'))}</h2><p class="sub">${S.available.length} ${esc(t('pcs'))} · ${esc(t('pickOne'))}</p>
    ${S.available.map(o => `<button class="task" type="button" data-sel="${esc(o.id)}" aria-pressed="${o.id === S.sel}">
      ${icon(o.id === S.sel ? 'circle-check-filled' : 'circle', 'pick')}
      <b>#${short(o.id)}</b>
      <span class="amt money">${esc(money(o.total))}</span>
      <span class="note">${esc(o.address?.line || '—')}</span>
    </button>`).join('')}
    <button class="cta" id="take" type="button"${queuedFor(chosen.id) ? ' disabled' : ''}>${
      queuedFor(chosen.id)
        ? `${icon('cloud-upload')}${esc(t('queued'))}`
        : `${icon('package')}${esc(t('take'))}`} #${short(chosen.id)}</button>
    <button class="ghost" id="endShift" type="button">${icon('power')}${esc(t('endShift'))}</button>`;
  $('#endShift').onclick = () => setShift(false);
  document.querySelectorAll('[data-sel]').forEach(b => b.onclick = () => { S.sel = b.dataset.sel; render(); });
  $('#take').onclick = async () => {
    const b = $('#take'); b.disabled = true;
    try {
      const r = await tapped(`/courier/orders/${encodeURIComponent(chosen.id)}/accept`, { tag:'accept:' + chosen.id });
      // The ripple is the sea's way of saying the hub answered. A tap that is
      // only queued has not been answered by anything, so it does not get one
      // -- and it does not get a re-read either: the network that just refused
      // the tap will refuse the read, and a second failure toast on top of
      // "saved" reads as if the save failed too.
      if (r.landed) { seaEvent('courier_assigned', 60); await load(); }
    }
    catch (e) { toast(String(e.message || e), 'alert-circle'); seaEvent('dispatch_failed', 40); b.disabled = false; }
  };
}

/// A courier's own estimate to the door: straight-line distance from the last
/// fix at a city cycling pace, plus the handover. The hub's kitchen profile
/// does the same sum for the customer; here the courier sees their leg of it.
const COURIER_SPEED_M_PER_MIN = 250, HANDOVER_MIN = 2, EARTH_R_M = 6_371_000;
function straightLineM(a, b){
  const toRad = d => d * Math.PI / 180;
  const dLat = toRad(b.lat - a.lat), dLon = toRad(b.lon - a.lon);
  const h = Math.sin(dLat / 2) ** 2 + Math.cos(toRad(a.lat)) * Math.cos(toRad(b.lat)) * Math.sin(dLon / 2) ** 2;
  return 2 * EARTH_R_M * Math.asin(Math.sqrt(h));
}
function etaText(o){
  const lat = o.address?.lat_udeg, lon = o.address?.lon_udeg;
  if (!S.me || !Number.isFinite(lat) || !Number.isFinite(lon)) return '';
  const m = straightLineM(S.me, { lat: lat / 1e6, lon: lon / 1e6 });
  const min = Math.max(1, Math.round(m / COURIER_SPEED_M_PER_MIN) + HANDOVER_MIN);
  return `≈ ${min} ${t('min')} · ${(m / 1000).toFixed(1)} ${t('km')} ${t('etaToDoor')}`;
}
function bindLangChrome(){
  const b = $('#langBtn'); if (!b) return;
  b.textContent = lang.toUpperCase();
  b.onclick = () => { setLang(nextLang()); b.textContent = lang.toUpperCase(); applyTheme(); guide = null; initGuide(); drawOutbox(); if (store.t) render(); else renderLogin(); };
}
function orderHead(o, picked){
  const cash = o.payment === 'cash' ? o.total : 0;
  const addr = o.address?.line || '';
  return `
    <h2>${picked ? esc(t('delivering')) : esc(t('pickUpOrder'))}</h2>
    <p class="sub"><span class="status st-${picked ? 'delivery' : 'ready'}${picked ? ' live' : ''}">${picked ? esc(t('onTheWay')) : esc(t('ready'))}</span>
      <span>#${short(o.id)} · ${esc(o.items)} ${esc(t('items'))}</span></p>
    <p class="eta" id="etaLine">${etaText(o)}</p>
    <div class="addr">${icon('map-pin')}<span>${esc(addr || '—')}</span></div>
    ${o.address?.note ? `<div class="note">${esc(o.address.note)}</div>` : ''}
    <div class="meta">
      ${cash ? `<span class="cash">${icon('cash')}<span class="money">${esc(money(cash))}</span></span>`
             : `<span class="note">${icon('credit-card')} ${esc(t('paidOnline'))}</span>`}
      ${o.contact?.phone ? `<a class="tel" href="tel:${esc(o.contact.phone)}">${icon('phone')}${esc(o.contact.phone)}</a>` : ''}
    </div>`;
}

// ── voice ───────────────────────────────────────────────────────────────────
// The surface voice matters most on: a courier is outdoors, moving, often with
// one hand on a handlebar and gloves on. Typing here is close to useless, which
// is why the typed ask box exists only in the one state where they are standing
// still -- and why this does not.
//
// The MICROPHONE IS ALWAYS AVAILABLE while on shift, including mid-delivery,
// because that is exactly when hands are busy.

let vrec = null, vlistening = false;

function voiceLang(){ return voiceLocale(); }

async function sendVoice(payload){
  return api('/voice', { method:'POST', body: JSON.stringify(payload) });
}

// The pending proposal. A consequential command is never acted on from one
// utterance -- the hub returns a token and a read-back, and this holds it until
// the courier agrees.
let vpending = null;

function voiceSay(line, kind){
  const el = $('#vsay');
  if (!el) return;
  el.hidden = false;
  el.className = 'vsay ' + (kind || '');
  el.textContent = line;
}

function clearVoice(){
  vpending = null;
  const el = $('#vsay'); if (el) el.hidden = true;
  const bar = $('#vconfirm'); if (bar) bar.hidden = true;
}

async function handleVoice(r){
  if (!r.understood) {
    // Echo what was heard: it is how a person learns to speak to the thing,
    // instead of repeating the same misheard phrase louder.
    voiceSay(`${r.say}${r.heard ? ' · «' + r.heard + '»' : ''}`, 'bad');
    speak(r.say, voiceLang());
    return;
  }
  if (!r.needsConfirmation) {
    if (r.action === 'status') {
      const line = t('openWaiting', { open: r.open, waiting: r.waiting });
      voiceSay(line); speak(line, voiceLang());
    } else if (r.action === 'ask') {
      // NOT answered here. Voice works with the assistant off; if it is on, the
      // question goes to it, and if it is off the courier is told plainly.
      voiceSay(t('asking'));
      try {
        const d = await api('/courier/assist', { method:'POST', body: JSON.stringify({ question: r.question }) });
        voiceSay(d.answer); speak(d.answer, voiceLang());
      } catch (e) { voiceSay(String(e.message || e), 'bad'); }
    }
    return;
  }
  // A proposal: read it back, out loud, and wait.
  vpending = r.token;
  voiceSay(r.readback);
  speak(r.readback + '?', voiceLang());
  const bar = $('#vconfirm'); if (bar) bar.hidden = false;
}

async function confirmVoice(){
  if (!vpending) return;
  const t = vpending; vpending = null;
  try {
    await sendVoice({ confirm: t });
    clearVoice();
    await load();
  } catch (e) { voiceSay(String(e.message || e), 'bad'); }
}

function startVoice(){
  if (vlistening) { vrec?.stop(); return; }
  if (!vsupported()) { toast(t('voiceUnsupported'), 'microphone-off'); return; }
  clearVoice();
  vrec = vcreate({
    lang: voiceLang(),
    onResult: async res => {
      voiceSay(res.transcript, res.isFinal ? '' : 'dim');
      if (!res.isFinal) return;
      try { await handleVoice(await sendVoice({
        transcript: res.transcript, confidence: res.confidence,
        is_final: true, lang: 'uk' })); }
      catch (e) { voiceSay(String(e.message || e), 'bad'); }
    },
    onError: err => {
      vlistening = false; setMicState();
      voiceSay(err === 'microphone-denied'
        ? t('micDenied')
        : err === 'network' ? t('voiceOffline') : String(err), 'bad');
    },
    onEnd: () => { vlistening = false; setMicState(); },
  });
  if (!vrec) return;
  vlistening = true; setMicState();
  try { vrec.start(); } catch { vlistening = false; setMicState(); }
}

function setMicState(){
  const b = $('#mic'); if (!b) return;
  b.classList.toggle('on', vlistening);
  b.setAttribute('aria-pressed', String(vlistening));
  b.setAttribute('aria-label', vlistening ? t('stopListening') : t('sayCommand'));
}

// The courier's own assistant. Its facts are only this courier's open
// deliveries -- the hub builds them that way, so a question cannot reach work
// that is not theirs.
function bindAsk(){
  const box = $('#askBox'), go = $('#askGo'), out = $('#answer');
  if (!box || !go) return;
  const ask = async () => {
    const q = box.value.trim(); if (!q) return;
    go.disabled = true; out.hidden = false; out.textContent = t('thinking');
    try {
      const d = await api('/courier/assist', { method:'POST', body: JSON.stringify({ question: q }) });
      out.textContent = d.answer;
    } catch (e) {
      // The reason, not a shrug: "assistant is off" and "model unreachable"
      // need different people to do different things.
      out.textContent = String(e.message || e);
    }
    go.disabled = false;
  };
  go.onclick = ask;
  box.onkeydown = e => { if (e.key === 'Enter') ask(); };
}

// ── what I did, and what I am holding ──
//
// Reachable only from the WAITING state, like the ask box, and for the same
// reason: this app shows one job at a time and a permanent tab bar would
// compete with the address and the call button on a live delivery. A courier
// checks their cash between runs, not while riding.
//
// `sheet()` is the app's existing panel; these replace its content and put a
// back button on it rather than introducing a second navigation model.
async function panel(title, bodyHtml){
  $('#app').innerHTML = `
    <div class="phead">
      <button class="icon-btn" id="pback" type="button" aria-label="${esc(t('back'))}">${icon('arrow-left')}</button>
      <b>${esc(title)}</b>
    </div>
    ${bodyHtml}`;
  $('#pback').onclick = () => render();
}

async function openEarnings(){
  await panel(t('myShifts'), `<div class="skel skel-5"></div>`);
  let d;
  try { d = await api('/courier/earnings'); }
  catch (e) { return panel(t('myShifts'), `<p class="answer">${esc(String(e.message || e))}</p>`); }
  // TIPS ARE SHOWN APART FROM THE FLOAT. The cash on the first line is money
  // the courier is holding FOR the venue and will hand over; the tips are
  // theirs. One combined figure at the end of a shift is the wrong number to
  // reach for, whichever way you reach.
  const row = (label, w) => `
    <div class="erow"><span>${esc(label)}</span>
      <span><b>${w.deliveries}</b> · <span class="money">${money(w.cash)}</span>${
        w.tips ? ` · <span class="money tips">+${money(w.tips)}</span>` : ''}</span></div>`;
  await panel(t('myShifts'), `
    <div class="ecash">
      <span class="k">${esc(t('cashInHand'))}</span>
      <span class="v money">${money(d.cashInHand)}</span>
    </div>
    ${d.expectedCash ? `<p class="hint2">${esc(t('stillOnRoad'))}: <span class="money">${money(d.expectedCash)}</span></p>` : ''}
    <div class="elist">
      ${row(t('today'), d.today)}${row(t('days7'), d.week)}${row(t('days30'), d.month)}
    </div>
    <p class="hint2">${esc(t('earningsHint'))}</p>`);
}

async function openHistory(){
  await panel(t('history'), `<div class="skel skel-4"></div>`);
  let d;
  try { d = await api('/courier/history'); }
  catch (e) { return panel(t('history'), `<p class="answer">${esc(String(e.message || e))}</p>`); }
  const rows = d.history || [];
  await panel(t('history'), rows.length ? `
    <div class="elist">${rows.map(r => `
      <div class="erow">
        <span>${esc(r.street || '—')}<br>
          <small class="hint2">${new Date(r.at || 0).toLocaleDateString(intlLocale(), { day:'numeric', month:'short' })} · ${esc(r.status)}</small></span>
        <span class="money">${money(r.cashCollected ?? r.total ?? 0)}</span>
      </div>`).join('')}</div>`
    : `<div class="empty">${icon('history')}<b>${esc(t('emptyHistory'))}</b>${esc(t('emptyHistoryHint'))}</div>`);
}

// ── an offer, with the time left on it ──────────────────────────────────────
//
// The venue gave this order to this courier. Five minutes later it goes back to
// the pool -- NOT declined, not held against them, just no longer exclusively
// theirs. So the screen says what is left rather than counting down to a
// punishment, and the order stays takeable after it lapses if nobody else got
// there first.
//
// The deadline arrives as an INSTANT and the remaining time is worked out here.
// A server-computed "seconds left" is stale the moment it is sent, and a phone
// polling every few seconds would show it jumping backwards.
function renderOffer(o){
  const left = () => Math.max(0, Math.round((o.offerEndsMs - Date.now()) / 1000));
  const mmss = s => `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
  const lapsed = left() === 0;
  $('#app').innerHTML = `
    <div class="offer">
      <span class="tag on"><span class="dot"></span>${esc(t('offered'))}</span>
      <b class="oid">#${short(o.id)}</b>
      <span class="amt money">${esc(money(o.total))}</span>
      <p class="note">${esc(o.address?.line || '—')}</p>
      <p class="hint2" id="offerLeft">${lapsed
        ? esc(t('offerLapsed'))
        : `${esc(t('timeLeft'))} <b id="offerClock">${mmss(left())}</b>`}</p>
    </div>
    <button class="cta" id="takeOffer" type="button">${icon('package')}${esc(t('take'))}</button>
    <button class="ghost" id="endShift" type="button">${icon('power')}${esc(t('endShift'))}</button>`;
  $('#endShift').onclick = () => setShift(false);
  $('#takeOffer').onclick = async () => {
    const b = $('#takeOffer'); b.disabled = true;
    try {
      const r = await tapped(`/courier/orders/${encodeURIComponent(o.id)}/accept`, { tag:'accept:' + o.id });
      if (r.landed) { seaEvent('courier_assigned', 60); await load(); }
    } catch (e) {
      toast(String(e.message || e), 'alert-circle');
      seaEvent('dispatch_failed', 40);
      b.disabled = false;
    }
  };
  // One interval, cleared by the next render. A timer left running behind a
  // screen that no longer exists is a battery drain nobody can see.
  clearInterval(renderOffer._t);
  renderOffer._t = setInterval(() => {
    const el = document.getElementById('offerClock');
    if (!el) { clearInterval(renderOffer._t); return; }
    const n = left();
    if (n === 0) { clearInterval(renderOffer._t); return load(); }
    el.textContent = mmss(n);
  }, 1000);
}

function renderActive(o){
  const picked = o.status === 'IN_DELIVERY';
  const addr = o.address?.line || '';
  const cash = o.payment === 'cash' ? o.total : 0;
  // A TAP THIS PHONE IS STILL HOLDING REPLACES THE CONTROL, and it does not
  // touch the status chip above it -- that chip is the hub's answer and the hub
  // has not answered yet. Offering the same button again would invite a second
  // tap for a job whose first tap is already saved.
  const waiting = queuedFor(o.id);
  $('#app').innerHTML = `${orderHead(o, picked)}
    ${waiting
      ? `<div class="empty" role="status">${icon('cloud-upload')}
           <b>${esc(t('queued'))}</b>${esc(t('queuedHint'))}</div>`
      : picked
      ? `<div class="slide" id="slide">
           <div class="slide-fill" id="slideFill"></div>
           <button class="cta go slide-knob" id="done" type="button"
                   aria-label="${esc(t('deliveredAria'))}">
             ${icon('circle-check')}${esc(t('delivered'))}</button>
           <span class="slide-hint" aria-hidden="true">${esc(t('swipe'))}</span>
         </div>`
        : `<button class="cta" id="pick" type="button">${icon('package')}${esc(t('pickedUp'))}</button>`}
    <div class="row2">
      ${addr ? `<a class="ghost" target="_blank" rel="noopener"
          href="https://www.openstreetmap.org/search?query=${encodeURIComponent(addr)}">${icon('external-link')}${esc(t('inMaps'))}</a>` : ''}
      ${o.contact?.phone ? `<a class="ghost" href="tel:${esc(o.contact.phone)}">${icon('phone')}${esc(t('call'))}</a>` : ''}
    </div>
    ${picked && !waiting ? `<button class="ghost" id="refused" type="button">${icon('x')}${esc(t('refusedAtDoor'))}</button>` : ''}`;

  // THE DESTINATION ON THE MAP. Micro-degrees back to degrees here and nowhere
  // else: the wire and the store hold integers, and this is the single boundary
  // where a float is correct because maplibre speaks degrees.
  //
  // Awaited rather than fired blind, because `initMap` may still be fetching the
  // library on a slow connection and a marker added to a null map is silently
  // lost -- which would leave the courier looking at a map with no destination
  // on it and no way to know why.
  const lat = o.address?.lat_udeg, lon = o.address?.lon_udeg;
  if (Number.isFinite(lat) && Number.isFinite(lon)) {
    initMap().then(() => markDrop(lon / 1e6, lat / 1e6)).catch(() => {});
  }

  if ($('#pick')) $('#pick').onclick = async () => {
    const b = $('#pick');
    const had = b.innerHTML;
    b.disabled = true; b.setAttribute('aria-busy', 'true');
    b.innerHTML = `${icon('loader-2')}${esc(t('saving'))}`;
    b.querySelector('.ti')?.classList.add('spin');
    try { const r = await tapped(`/courier/orders/${encodeURIComponent(o.id)}/pickup`, { tag:'pickup:' + o.id }); if (r.landed) await load(); }
    catch (e) {
      toast(String(e.message || e), 'alert-circle');
      b.disabled = false; b.removeAttribute('aria-busy'); b.innerHTML = had;
    }
  };
  // REFUSED AT THE DOOR (§2.4): the refund under this courier, with nothing
  // collected. Asked on its own screen -- it ends the run -- with an optional
  // note of what happened at the door.
  if ($('#refused')) $('#refused').onclick = () => renderRefused(o);
  // ── swipe to complete ──
  //
  // "Delivered" is irreversible and sits under a thumb that has been holding a
  // phone in the rain. A tap is too cheap for it. The slider is the deliberate
  // gesture; the element underneath is still a real <button>, so a keyboard or
  // a screen reader activates it directly and gets a confirm instead -- the
  // gesture is the guard, not the interface.
  //
  // IT RESETS ON RELEASE. A knob left halfway is a courier who thinks the order
  // is done and a kitchen that thinks it is not.
  const track = $('#slide'), knob = $('#done'), fill = $('#slideFill');
  if (track && knob) {
    let dragging = false, startX = 0, travelled = 0, fired = false;
    const width = () => track.clientWidth - knob.offsetWidth;
    const put = px => {
      travelled = Math.max(0, Math.min(width(), px));
      knob.style.transform = `translateX(${travelled}px)`;
      fill.style.width = `${travelled + knob.offsetWidth}px`;
    };
    const reset = () => { dragging = false; put(0); track.classList.remove('dragging'); };
    knob.addEventListener('pointerdown', e => {
      dragging = true; fired = false; startX = e.clientX;
      track.classList.add('dragging');
      knob.setPointerCapture(e.pointerId);
    });
    knob.addEventListener('pointermove', e => {
      if (!dragging) return;
      put(e.clientX - startX);
      // Ninety per cent, not the whole track: the last few pixels are where a
      // thumb runs out of screen.
      if (!fired && travelled >= width() * 0.9) { fired = true; dragging = false; finish(); }
    });
    knob.addEventListener('pointerup', () => { if (!fired) reset(); });
    knob.addEventListener('pointercancel', reset);
    knob.onclick = e => {
      // A real click: keyboard, assistive technology, or a thumb that tapped
      // instead of dragging. Confirm rather than refuse -- refusing would leave
      // a keyboard user with no way to finish a delivery at all.
      if (fired || travelled > 4) { e.preventDefault(); return; }
      if (confirm(t('confirmDelivered'))) finish();
    };
  }

  function finish(){
    // Short handovers happen. Record what was actually taken rather than
    // assume the full amount -- this number settles disputes later. The
    // question is asked in the sheet, at 16px+, in the courier's theme, not
    // in window.prompt.
    if (cash) { S.cashFor = o.id; return renderCash(o); }
    deliver(o, 0);
  }
}

function renderCash(o){
  const cash = o.total;
  $('#app').innerHTML = `${orderHead(o, true)}
    <label for="got">${esc(t('howMuchCash'))}</label>
    <input id="got" class="money" inputmode="numeric" pattern="[0-9]*" autocomplete="off" enterkeyhint="done" value="${esc(cash)}">
    <button class="cta go" id="confirm" type="button">${icon('circle-check')}${esc(t('confirm'))}</button>
    <button class="ghost" id="back" type="button">${icon('arrow-left')}${esc(t('back'))}</button>`;
  const inp = $('#got');
  inp.focus(); inp.select();
  $('#back').onclick = () => { S.cashFor = null; renderActive(o); };
  const go = () => {
    const collected = parseInt(inp.value, 10);
    if (!Number.isFinite(collected) || collected < 0) return toast(t('badAmount'), 'alert-circle');
    $('#confirm').disabled = true;
    deliver(o, collected);
  };
  $('#confirm').onclick = go;
  inp.onkeydown = ev => { if (ev.key === 'Enter') { ev.preventDefault(); go(); } };
}

/// The refused-at-door screen: a confirm, and a short optional note (the
/// server keeps at most 280 characters; the field stops there too). Sent
/// through the outbox like every other tap.
const NOTE_MAX = 280;
function renderRefused(o){
  $('#app').innerHTML = `${orderHead(o, true)}
    <p>${esc(t('confirmRefused'))}</p>
    <label for="rnote">${esc(t('refusedNoteLabel'))}</label>
    <textarea id="rnote" maxlength="${NOTE_MAX}" rows="3" autocomplete="off" placeholder="${esc(t('refusedNoteHint'))}"></textarea>
    <button class="cta" id="rgo" type="button">${icon('x')}${esc(t('refusedAtDoor'))}</button>
    <button class="ghost" id="back" type="button">${icon('arrow-left')}${esc(t('back'))}</button>`;
  $('#back').onclick = () => renderActive(o);
  $('#rgo').onclick = async () => {
    const b = $('#rgo');
    b.disabled = true; b.setAttribute('aria-busy', 'true');
    const note = $('#rnote').value.trim().slice(0, NOTE_MAX);
    try {
      const r = await tapped(`/courier/orders/${encodeURIComponent(o.id)}/refused`, { tag:'refused:' + o.id, body: JSON.stringify(note ? { note } : {}) });
      // A queued tap is shown as queued on the run's own screen; only the
      // hub's answer reloads.
      if (r.landed) { toast(t('refusedDone'), 'circle-check'); await load(); }
      else renderActive(o);
    } catch (e) {
      toast(String(e.message || e), 'alert-circle');
      b.disabled = false; b.removeAttribute('aria-busy');
    }
  };
}

async function deliver(o, collected){
  const btn = $('#done') || $('#confirm');
  if (btn) btn.disabled = true;
  try {
    const r = await tapped(`/courier/orders/${encodeURIComponent(o.id)}/deliver`, {
      tag:'deliver:' + o.id, body: JSON.stringify({ cash_collected: collected }) });
    S.cashFor = null;
    // ONLY A DELIVERY THE HUB CONFIRMED IS ANNOUNCED AS ONE. A queued tap has
    // no shortfall to report and no event to ripple: `tapped` has already said
    // it was saved, and saying "Delivered" on top of that would be the client
    // telling the courier something only the hub can know.
    if (r.landed) {
      const d = r.data || {};
      // A toast is plain text and cannot carry a class; the value is still the
      // kernel's integer. // money:toast
      if (d.short > 0) toast(`${t('shortfall')} ${money(d.short)} — ${t('recorded')}`, 'alert-triangle'); // money:toast
      else toast(t('delivered'), 'circle-check');
      seaEvent('delivered', 160);
      await load();
    }
  } catch (e) {
    toast(String(e.message || e), 'alert-circle'); seaEvent('dispatch_failed', 40);
    if (btn) btn.disabled = false;
  }
}

async function setShift(open){
  try { await api('/courier/shift', { method:'POST', attend:true, body: JSON.stringify({ open }) }); await load(); }
  catch (e) { toast(String(e.message || e), 'alert-circle'); }
}

// ── guide ───────────────────────────────────────────────────────────────────
// The courier's version is short because the surface is: one screen, one job,
// four controls. Same module, same rule -- one table, the tour is an order
// over it. The help entry lives in the sheet and only while the courier is
// standing still (offline, or on shift with nothing in hand): a "Довідка"
// button beside a live delivery is one more thing to mis-tap on a handlebar.
const HELP = () => ({
  welcome: { hint:false, title:t('hWelcomeT'), body:t('hWelcome') },
  shift: { at:'#shiftTag', hint:false, title:t('hShiftT'), body:t('hShift') },
  sheet: { at:'#sheet', hint:false, title:t('hSheetT'), body:t('hSheet') },
  mic: { at:'#mic', hint:false, title:t('hMicT'), body:t('hMic') },
  ask: { at:'#askBox', title:t('hAskT'), body:t('hAsk') },
  help: { at:'.gd-help', hint:false, title:t('hHelpT'), body:t('hHelp') },
});
/// The guide's own buttons, in the reader's language.
const guideWords = () => ({ step: (i, n) => t('gStep', { i, n }), skip: t('gSkip'), later: t('gLater'), back: t('gBack'), next: t('gNext'), done: t('gDone'),
  paused: t('gPaused'), skipped: t('gSkipped'), finished: t('gFinished'), unfinished: t('gUnfinished'), what: t('gWhat'), close: t('gClose'), help: t('help') });
const TOUR = ['welcome', 'shift', 'sheet', 'mic', 'help'];
let guide = null;
function initGuide(){
  guide ??= createGuide({
    key:'courier', help: HELP(), tour: TOUR, toast, words: guideWords(),
    mount: { into:'#app', className:'ghost', text:t('help'),
             when: () => S.loadedOnce && !S.mine.length && !$('#pback') },
  });
  guide.init();
}

function bindVoiceChrome(){
  // The microphone appears only where there is a recogniser behind it.
  const mic = $('#mic');
  if (mic && vsupported()) { mic.hidden = false; mic.onclick = startVoice; }
  const yes = $('#vyes'); if (yes) yes.onclick = confirmVoice;
  const no = $('#vno'); if (no) no.onclick = () => { clearVoice(); };
}

async function boot(){ S.booted = true;
  // BEFORE the first load, and not only on the `online` event: a courier who
  // killed the app in the tunnel and reopens it above ground is already online,
  // so no event will ever fire and the queue would sit there.
  OUT.start();
  bindVoiceChrome(); bindLangChrome();
  initGuide();
  // NOT awaited. The task list is what this screen is for; the map is how the
  // task is easier. Blocking the first paint on a 245 KB download would make
  // the important thing wait for the helpful one.
  initMap();
  await load();
  guide.autoStart();
  clearTimeout(boot._i);
  const scheduleLoad = () => {
    // ON SHIFT IS THE LIVE CASE, not "has work": an offer arrives when the
    // courier has nothing, and waiting a minute to see it is how a courier
    // loses the run. Off shift the app is a sign-in screen.
    const wait = S.onShift ? 12000 : 60000;
    boot._i = setTimeout(() => {
      // The socket has been quiet and healthy: nothing has been missed.
      if (!document.hidden && S.booted && (!S.live || S.live.due())) { S.live?.polled(); load(); }
      scheduleLoad();
    }, wait);
  };
  scheduleLoad();
  openSocket();
}

/// THE HUB TELLS THIS APP when the queue moves, and carries this courier's
/// position back the other way. The interval above still runs -- a socket dies
/// for reasons a courier on a motorbike cannot do anything about -- but while
/// the socket is healthy it is the socket that wakes the screen.
function openSocket(){
  import('/lib/live.js').then(({ live }) => {
    S.live = live({
      token: store.t,
      onEvent: () => { if (!document.hidden && S.booted) load(); },
    });
  }).catch(() => { /* no socket: the interval is the whole story */ });
}

document.documentElement.lang = lang; document.title = t('appTitle'); retranslate(document);
applyTheme(); bindLangChrome();
store.t ? boot() : renderLogin();

// The shell that lets this page open with no network (`/courier/sw.js`).
if ('serviceWorker' in navigator) {
  addEventListener('load', () => navigator.serviceWorker.register('/courier/sw.js', { scope: '/courier/' })
    .catch(e => console.error('courier: service worker not registered', e)));
}
