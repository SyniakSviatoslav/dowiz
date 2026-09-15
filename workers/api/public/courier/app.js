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

const API = '/api';
const $ = (s, r = document) => r.querySelector(s);
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const money = n => new Intl.NumberFormat('uk', { style:'currency', currency:'ALL', maximumFractionDigits:0 }).format(n || 0);
const short = id => esc(String(id).slice(0, 8));
const icon = (name, cls = '') => `<i class="ti ti-${name} i ${cls}" aria-hidden="true"></i>`;

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
  '':      { icon:'sun-moon', label:'Тема: як на телефоні' },
  'dark':  { icon:'moon',     label:'Тема: темна' },
  'light': { icon:'sun',      label:'Тема: світла' },
};
function applyTheme(){
  const p = store.theme;
  if (p) document.documentElement.dataset.theme = p; else delete document.documentElement.dataset.theme;
  const ui = THEME_UI[p] || THEME_UI[''];
  $('#themeBtn').innerHTML = icon(ui.icon);
  $('#themeBtn').setAttribute('aria-label', ui.label);
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
// Loaded once, cached by the promise so two callers race safely, and its
// stylesheet comes with it rather than sitting in the document head.
let mapLibPromise = null;
function loadMapLibrary(){
  if (window.maplibregl) return Promise.resolve();
  if (mapLibPromise) return mapLibPromise;
  mapLibPromise = new Promise((resolve, reject) => {
    const css = document.createElement('link');
    css.rel = 'stylesheet';
    css.href = 'https://cdn.jsdelivr.net/npm/maplibre-gl@5.9.0/dist/maplibre-gl.css';
    document.head.appendChild(css);
    const js = document.createElement('script');
    js.src = 'https://cdn.jsdelivr.net/npm/maplibre-gl@5.9.0/dist/maplibre-gl.js';
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
  map = new maplibregl.Map({
    container: 'map',
    style: STYLE[mapDark ? 'dark' : 'light'],
    center: [19.4449964, 41.315347],   // the venue, until a fix arrives
    zoom: 13, attributionControl: { compact: true },
  });
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
async function api(path, opts = {}){
  const { attend, ...rest } = opts;
  if (attend) { S.inflight++; $('#sheet').classList.add('attending'); }
  try {
    const r = await fetch(API + path, { ...rest,
      headers: { 'content-type':'application/json', ...(rest.headers || {}),
                 ...(store.t ? { authorization:'Bearer ' + store.t } : {}) } });
    if (r.status === 401) { store.t = null; renderLogin('Сесію завершено'); throw new Error('unauthorised'); }
    if (!r.ok) { let m = 'HTTP ' + r.status; try { const d = await r.json(); m = d.error || d.message || m; } catch {} throw new Error(m); }
    return r.status === 204 ? null : r.json();
  } finally {
    if (attend && --S.inflight <= 0) { S.inflight = 0; $('#sheet').classList.remove('attending'); }
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
function startTracking(){
  if (S.watchId != null || !navigator.geolocation) return;
  gps('GPS', false);
  S.watchId = navigator.geolocation.watchPosition(async pos => {
    const { latitude, longitude, accuracy, speed } = pos.coords;
    markMe(longitude, latitude);
    S.moving = speed != null && speed > MOVING_MPS;
    if (accuracy > MAX_ACCURACY_M) {
      // Say so rather than quietly sending it: a 500m fix looks like a position.
      gps(`±${Math.round(accuracy)} м`, true);
      return;
    }
    if (speed != null && speed > MAX_SPEED_MPS) return;
    gps(`±${Math.round(accuracy)} м`, false);
    const active = S.mine[0];
    try {
      await api('/courier/position', { method:'POST', body: JSON.stringify({
        lat: latitude, lon: longitude, accuracy_m: accuracy,
        speed_mps: speed ?? null, order_id: active ? active.id : null }) });
    } catch {}
  }, err => {
    gps(err.code === 1 ? 'GPS заборонено' : 'GPS недоступний', true);
  }, { enableHighAccuracy:true, maximumAge:5000, timeout:20000 });
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
    if (away > 5000) toast('Застосунок був у фоні — GPS міг перерватися', 'alert-triangle');
  }
  if (S.booted) load();
});

// ── login ──
function renderLogin(err){
  $('#app').innerHTML = `<form class="login" id="loginForm" novalidate>
    <h2>Вхід для кур'єра</h2>
    <label for="em">Email або телефон</label>
    <input id="em" autocomplete="username" inputmode="email" enterkeyhint="next">
    <label for="pw">Пароль</label>
    <input id="pw" type="password" autocomplete="current-password" enterkeyhint="go">
    ${err ? `<div class="err" role="alert">${icon('alert-circle')}<span>${esc(err)}</span></div>` : ''}
    <button class="cta" id="go" type="submit">${icon('login')}Увійти</button>
  </form>`;
  $('#loginForm').onsubmit = async ev => {
    ev.preventDefault();
    const b = $('#go'); b.disabled = true; b.innerHTML = `${icon('loader-2')}Входимо…`;
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

// ── shift + tasks ──
async function load(){
  try {
    const wasOn = S.onShift, hadActive = S.mine.length > 0;
    const before = new Set(S.available.map(o => o.id));
    const d = await api('/courier/tasks');
    S.onShift = d.onShift; S.mine = d.mine || []; S.available = d.available || []; S.shift = d.shift;
    // "task_assigned = one incoming ripple + ping": a task that was not on the
    // last poll, arriving while the courier is on shift and free.
    const fresh = S.available.filter(o => !before.has(o.id));
    if (S.loadedOnce && wasOn && S.onShift && !hadActive && !S.mine.length && fresh.length) {
      ping(); seaEvent('order_created', 80);
    }
    S.loadedOnce = true;
    S.phase = 'ready'; S.error = null;
    render();
  } catch (e) {
    if (String(e.message) === 'unauthorised') return;
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
    ? `на зміні · ${esc(S.shift?.deliveries ?? 0)} · <span class="money">${esc(money(S.shift?.cash ?? 0))}</span>`
    : 'офлайн';
}

function render(){
  setShiftTag();

  // BEFORE anything is claimed about the shift. A skeleton here is not
  // decoration: the alternative is asserting "you are offline" on no evidence.
  if (S.phase === 'loading' && !S.loadedOnce) {
    $('#app').innerHTML = `<div class="loadwrap" aria-busy="true" aria-label="Завантажуємо">
      <div class="skel" style="height:1.2rem;width:9rem"></div>
      <div class="skel" style="height:4.5rem"></div>
      <div class="skel" style="height:4.5rem"></div>
      <div class="skel" style="height:var(--tap);width:60%"></div>
    </div>`;
    return;
  }
  if (S.phase === 'error' && !S.loadedOnce) {
    $('#app').innerHTML = `<div class="empty" role="alert">${icon('plug-connected-x')}
      <b>Немає зв'язку із закладом</b>
      <span class="reason">${esc(S.error || '')}</span></div>
      <button class="cta go" id="retry" type="button">${icon('refresh')}Спробувати ще раз</button>`;
    $('#retry').onclick = async () => {
      S.phase = 'loading'; render(); await load();
    };
    return;
  }

  if (!S.onShift) {
    stopTracking(); keepAwake(false); S.cashFor = null;
    $('#app').innerHTML = `<div class="empty">${icon('moon-stars')}<b>Ви офлайн</b>Замовлення не надходитимуть, поки зміну не відкрито</div>
      <button class="cta go" id="openShift" type="button">${icon('player-play')}Почати зміну</button>`;
    $('#openShift').onclick = () => setShift(true);
    return;
  }
  startTracking();
  initSea();

  // ONE job on screen. A run in hand hides everything else.
  const active = S.mine[0];
  if (active) { keepAwake(true); return S.cashFor && S.cashFor === active.id ? renderCash(active) : renderActive(active); }
  keepAwake(false); S.cashFor = null;

  if (!S.available.length) {
    // The ask box lives HERE and nowhere else. This app shows one job at a time
    // on purpose, and a text field on a live delivery screen would compete with
    // the address and the call button for a thumb that is on a handlebar. This
    // is the one state where the courier is standing still.
    $('#app').innerHTML = `<div class="empty">${icon('radar-2')}<b>Вільних замовлень немає</b>Щойно щось буде готове — з'явиться тут</div>
      <div class="askrow">
        <input id="askBox" class="ask" type="text" placeholder="Спитати про мої доставки…"
               autocomplete="off" enterkeyhint="send">
        <button class="ghost narrow" id="askGo" type="button">${icon('send')}</button>
      </div>
      <p id="answer" class="answer" hidden></p>
      <button class="ghost" id="endShift" type="button">${icon('power')}Завершити зміну</button>`;
    $('#endShift').onclick = () => setShift(false);
    bindAsk();
    return;
  }

  // Rows select; ONE CTA takes. N "Взяти" buttons would be N main actions on
  // one screen, which the courier rules forbid.
  if (!S.available.some(o => o.id === S.sel)) S.sel = S.available[0].id;
  const chosen = S.available.find(o => o.id === S.sel);
  $('#app').innerHTML = `<h2>Готові до забору</h2><p class="sub">${S.available.length} шт. · оберіть і візьміть</p>
    ${S.available.map(o => `<button class="task" type="button" data-sel="${esc(o.id)}" aria-pressed="${o.id === S.sel}">
      ${icon(o.id === S.sel ? 'circle-check-filled' : 'circle', 'pick')}
      <b>#${short(o.id)}</b>
      <span class="amt money">${esc(money(o.total))}</span>
      <span class="note">${esc(o.address?.line || '—')}</span>
    </button>`).join('')}
    <button class="cta" id="take" type="button">${icon('package')}Взяти #${short(chosen.id)}</button>
    <button class="ghost" id="endShift" type="button">${icon('power')}Завершити зміну</button>`;
  $('#endShift').onclick = () => setShift(false);
  document.querySelectorAll('[data-sel]').forEach(b => b.onclick = () => { S.sel = b.dataset.sel; render(); });
  $('#take').onclick = async () => {
    const b = $('#take'); b.disabled = true;
    try {
      await api(`/courier/orders/${encodeURIComponent(chosen.id)}/accept`, { method:'POST', attend:true });
      seaEvent('courier_assigned', 60);
      await load();
    }
    catch (e) { toast(String(e.message || e), 'alert-circle'); seaEvent('dispatch_failed', 40); b.disabled = false; }
  };
}

function orderHead(o, picked){
  const cash = o.payment === 'cash' ? o.total : 0;
  const addr = o.address?.line || '';
  return `
    <h2>${picked ? 'Доставляєте' : 'Заберіть замовлення'}</h2>
    <p class="sub"><span class="status${picked ? ' live' : ''}" style="--st:var(--st-${picked ? 'IN_DELIVERY' : 'READY'})">${picked ? 'В дорозі' : 'Готове'}</span>
      <span>#${short(o.id)} · ${esc(o.items)} поз.</span></p>
    <div class="addr">${icon('map-pin')}<span>${esc(addr || '—')}</span></div>
    ${o.address?.note ? `<div class="note">${esc(o.address.note)}</div>` : ''}
    <div class="meta">
      ${cash ? `<span class="cash">${icon('cash')}<span class="money">${esc(money(cash))}</span></span>`
             : `<span class="note">${icon('credit-card')} Оплачено онлайн</span>`}
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

function voiceLang(){ return 'uk-UA'; }          // this surface ships Ukrainian only

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
      const line = `Відкритих ${r.open}, чекає ${r.waiting}`;
      voiceSay(line); speak(line, voiceLang());
    } else if (r.action === 'ask') {
      // NOT answered here. Voice works with the assistant off; if it is on, the
      // question goes to it, and if it is off the courier is told plainly.
      voiceSay('Питаю…');
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
  if (!vsupported()) { toast('Браузер не розпізнає голос', 'microphone-off'); return; }
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
        ? 'Немає дозволу на мікрофон'
        : err === 'network' ? 'Розпізнавання недоступне офлайн' : String(err), 'bad');
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
  b.setAttribute('aria-label', vlistening ? 'Зупинити запис' : 'Сказати команду');
}

// The courier's own assistant. Its facts are only this courier's open
// deliveries -- the hub builds them that way, so a question cannot reach work
// that is not theirs.
function bindAsk(){
  const box = $('#askBox'), go = $('#askGo'), out = $('#answer');
  if (!box || !go) return;
  const ask = async () => {
    const q = box.value.trim(); if (!q) return;
    go.disabled = true; out.hidden = false; out.textContent = 'Думає…';
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

function renderActive(o){
  const picked = o.status === 'IN_DELIVERY';
  const addr = o.address?.line || '';
  const cash = o.payment === 'cash' ? o.total : 0;
  $('#app').innerHTML = `${orderHead(o, picked)}
    ${picked
      ? `<button class="cta go" id="done" type="button">${icon('circle-check')}Доставлено</button>`
      : `<button class="cta" id="pick" type="button">${icon('package')}Забрав</button>`}
    <div class="row2">
      ${addr ? `<a class="ghost" target="_blank" rel="noopener"
          href="https://www.openstreetmap.org/search?query=${encodeURIComponent(addr)}">${icon('external-link')}У картах</a>` : ''}
      ${o.contact?.phone ? `<a class="ghost" href="tel:${esc(o.contact.phone)}">${icon('phone')}Подзвонити</a>` : ''}
    </div>`;

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
    b.innerHTML = `${icon('loader-2')}Записуємо…`;
    b.querySelector('.ti')?.classList.add('spin');
    try { await api(`/courier/orders/${encodeURIComponent(o.id)}/pickup`, { method:'POST', attend:true }); await load(); }
    catch (e) {
      toast(String(e.message || e), 'alert-circle');
      b.disabled = false; b.removeAttribute('aria-busy'); b.innerHTML = had;
    }
  };
  if ($('#done')) $('#done').onclick = () => {
    // Short handovers happen. Record what was actually taken rather than
    // assume the full amount -- this number settles disputes later. The
    // question is asked in the sheet, at 16px+, in the courier's theme, not
    // in window.prompt.
    if (cash) { S.cashFor = o.id; return renderCash(o); }
    deliver(o, 0);
  };
}

function renderCash(o){
  const cash = o.total;
  $('#app').innerHTML = `${orderHead(o, true)}
    <label for="got">Скільки готівки отримано?</label>
    <input id="got" class="money" inputmode="numeric" pattern="[0-9]*" autocomplete="off" enterkeyhint="done" value="${esc(cash)}">
    <button class="cta go" id="confirm" type="button">${icon('circle-check')}Підтвердити</button>
    <button class="ghost" id="back" type="button">${icon('arrow-left')}Назад</button>`;
  const inp = $('#got');
  inp.focus(); inp.select();
  $('#back').onclick = () => { S.cashFor = null; renderActive(o); };
  const go = () => {
    const collected = parseInt(inp.value, 10);
    if (!Number.isFinite(collected) || collected < 0) return toast('Некоректна сума', 'alert-circle');
    $('#confirm').disabled = true;
    deliver(o, collected);
  };
  $('#confirm').onclick = go;
  inp.onkeydown = ev => { if (ev.key === 'Enter') { ev.preventDefault(); go(); } };
}

async function deliver(o, collected){
  const btn = $('#done') || $('#confirm');
  if (btn) btn.disabled = true;
  try {
    const d = await api(`/courier/orders/${encodeURIComponent(o.id)}/deliver`, {
      method:'POST', attend:true, body: JSON.stringify({ cash_collected: collected }) });
    S.cashFor = null;
    // A toast is plain text and cannot carry a class; the value is still the
    // kernel's integer. // money:toast
    if (d.short > 0) toast(`Недостача ${money(d.short)} — записано`, 'alert-triangle'); // money:toast
    else toast('Доставлено', 'circle-check');
    seaEvent('delivered', 160);
    await load();
  } catch (e) {
    toast(String(e.message || e), 'alert-circle'); seaEvent('dispatch_failed', 40);
    if (btn) btn.disabled = false;
  }
}

async function setShift(open){
  try { await api('/courier/shift', { method:'POST', attend:true, body: JSON.stringify({ open }) }); await load(); }
  catch (e) { toast(String(e.message || e), 'alert-circle'); }
}

function bindVoiceChrome(){
  // The microphone appears only where there is a recogniser behind it.
  const mic = $('#mic');
  if (mic && vsupported()) { mic.hidden = false; mic.onclick = startVoice; }
  const yes = $('#vyes'); if (yes) yes.onclick = confirmVoice;
  const no = $('#vno'); if (no) no.onclick = () => { clearVoice(); };
}

async function boot(){ S.booted = true;
  bindVoiceChrome();
  // NOT awaited. The task list is what this screen is for; the map is how the
  // task is easier. Blocking the first paint on a 245 KB download would make
  // the important thing wait for the helpful one.
  initMap();
  await load();
  clearInterval(boot._i);
  boot._i = setInterval(() => { if (!document.hidden && S.booted) load(); }, 12000); }

applyTheme();
store.t ? boot() : renderLogin();
