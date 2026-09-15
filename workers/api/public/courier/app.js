// Courier app. One job on screen at a time, because the person holding this
// phone is on a scooter. Every status change is the server's answer -- there is
// no ordered list of statuses in this file.
const API = '/api';
const $ = (s, r = document) => r.querySelector(s);
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const money = n => new Intl.NumberFormat('uk', { style:'currency', currency:'ALL', maximumFractionDigits:0 }).format(n || 0);

const store = {
  get t(){ try { return localStorage.getItem('dw_c_jwt'); } catch { return null; } },
  set t(v){ try { v ? localStorage.setItem('dw_c_jwt', v) : localStorage.removeItem('dw_c_jwt'); } catch {} },
};
let S = { onShift:false, mine:[], available:[], shift:null, watchId:null, wake:null, booted:false };

function toast(m){ const el = $('#toast'); el.textContent = m; el.classList.add('show');
  clearTimeout(toast._t); toast._t = setTimeout(() => el.classList.remove('show'), 3000); }

async function api(path, opts = {}){
  const r = await fetch(API + path, { ...opts,
    headers: { 'content-type':'application/json', ...(opts.headers || {}),
               ...(store.t ? { authorization:'Bearer ' + store.t } : {}) } });
  if (r.status === 401) { store.t = null; renderLogin('Сесію завершено'); throw new Error('unauthorised'); }
  if (!r.ok) { let m = 'HTTP ' + r.status; try { const d = await r.json(); m = d.error || d.message || m; } catch {} throw new Error(m); }
  return r.status === 204 ? null : r.json();
}

// ── map ──
let map = null, meMarker = null, dropMarker = null;
function initMap(){
  if (map || !window.maplibregl) return;
  map = new maplibregl.Map({
    container: 'map',
    // OpenFreeMap: no key, no quota, no account. The old platform used the same
    // tiles, so the look carries over and nothing new is signed up for.
    style: 'https://tiles.openfreemap.org/styles/bright',
    center: [19.4449964, 41.315347],   // the venue, until a fix arrives
    zoom: 13, attributionControl: { compact: true },
  });
  map.addControl(new maplibregl.NavigationControl({ showCompass:false }), 'top-right');
}
function markMe(lon, lat){
  if (!map) return;
  if (!meMarker) {
    const el = document.createElement('div');
    el.style.cssText = 'width:18px;height:18px;border-radius:50%;background:#1D4ED8;border:3px solid #fff;box-shadow:0 0 0 4px rgba(29,78,216,.25)';
    meMarker = new maplibregl.Marker({ element: el });
  }
  meMarker.setLngLat([lon, lat]).addTo(map);
}
function markDrop(lon, lat){
  if (!map) return;
  if (!dropMarker) dropMarker = new maplibregl.Marker({ color:'#C1121F' });
  dropMarker.setLngLat([lon, lat]).addTo(map);
  if (meMarker) {
    const b = new maplibregl.LngLatBounds();
    b.extend(meMarker.getLngLat()); b.extend([lon, lat]);
    map.fitBounds(b, { padding: 64, maxZoom: 16 });
  } else map.setCenter([lon, lat]);
}

// ── geolocation ──
// The design rules are explicit and they are enforced HERE as well as on the
// server: a fix worse than 100m, or a speed above 150km/h, is not data.
const MAX_ACCURACY_M = 100, MAX_SPEED_MPS = 41.67;
function startTracking(){
  if (S.watchId != null || !navigator.geolocation) return;
  $('#gpsTag').hidden = false;
  S.watchId = navigator.geolocation.watchPosition(async pos => {
    const { latitude, longitude, accuracy, speed } = pos.coords;
    markMe(longitude, latitude);
    const tag = $('#gpsTag');
    if (accuracy > MAX_ACCURACY_M) {
      // Say so rather than quietly sending it: a 500m fix looks like a position.
      tag.textContent = `GPS ±${Math.round(accuracy)} м`; tag.classList.add('warn');
      return;
    }
    if (speed != null && speed > MAX_SPEED_MPS) return;
    tag.textContent = `GPS ±${Math.round(accuracy)} м`; tag.classList.remove('warn');
    const active = S.mine[0];
    try {
      await api('/courier/position', { method:'POST', body: JSON.stringify({
        lat: latitude, lon: longitude, accuracy_m: accuracy,
        speed_mps: speed ?? null, order_id: active ? active.id : null }) });
    } catch {}
  }, err => {
    $('#gpsTag').hidden = false;
    $('#gpsTag').textContent = err.code === 1 ? 'GPS заборонено' : 'GPS недоступний';
    $('#gpsTag').classList.add('warn');
  }, { enableHighAccuracy:true, maximumAge:5000, timeout:20000 });
}
function stopTracking(){
  if (S.watchId != null) navigator.geolocation.clearWatch(S.watchId);
  S.watchId = null; $('#gpsTag').hidden = true;
}

// The screen must not sleep mid-run. Wake Lock drops on tab-hide, so it is
// re-taken when the page comes back rather than assumed to survive.
async function keepAwake(on){
  try {
    if (on && !S.wake && 'wakeLock' in navigator) S.wake = await navigator.wakeLock.request('screen');
    if (!on && S.wake) { await S.wake.release(); S.wake = null; }
  } catch {}
}
document.addEventListener('visibilitychange', async () => {
  if (!document.hidden && S.onShift && S.mine.length) keepAwake(true);
  if (!document.hidden && S.booted) load();
});

// ── login ──
function renderLogin(err){
  $('#app').innerHTML = `<div class="login">
    <h2>Вхід для кур'єра</h2>
    <label for="em">Email або телефон</label><input id="em" autocomplete="username" inputmode="email">
    <label for="pw">Пароль</label><input id="pw" type="password" autocomplete="current-password">
    ${err ? `<div class="err">${esc(err)}</div>` : ''}
    <button class="cta" id="go" style="margin-top:20px">Увійти</button>
  </div>`;
  $('#go').onclick = async () => {
    const b = $('#go'); b.disabled = true; b.textContent = 'Входимо…';
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
    const d = await api('/courier/tasks');
    S.onShift = d.onShift; S.mine = d.mine || []; S.available = d.available || []; S.shift = d.shift;
    render();
  } catch (e) { if (String(e.message) !== 'unauthorised') toast(String(e.message || e)); }
}

function render(){
  $('#shiftTag').textContent = S.onShift
    ? `на зміні · ${S.shift?.deliveries ?? 0} · ${money(S.shift?.cash ?? 0)}`
    : 'офлайн';
  $('#shiftTag').classList.toggle('warn', !S.onShift);

  if (!S.onShift) {
    stopTracking(); keepAwake(false);
    $('#app').innerHTML = `<div class="empty"><b>Ви офлайн</b>Замовлення не надходитимуть, поки зміну не відкрито</div>
      <button class="cta go" id="openShift">Почати зміну</button>`;
    $('#openShift').onclick = () => setShift(true);
    return;
  }
  startTracking();

  // ONE job on screen. A run in hand hides everything else.
  const active = S.mine[0];
  if (active) { keepAwake(true); return renderActive(active); }
  keepAwake(false);

  if (!S.available.length) {
    $('#app').innerHTML = `<div class="empty"><b>Вільних замовлень немає</b>Щойно щось буде готове — з'явиться тут</div>
      <button class="ghost" id="endShift">Завершити зміну</button>`;
    $('#endShift').onclick = () => setShift(false);
    return;
  }
  $('#app').innerHTML = `<h2>Готові до забору</h2><p class="sub">${S.available.length} шт.</p>
    ${S.available.map(o => `<div class="task">
      <div class="r1"><b>#${esc(String(o.id).slice(0,8))}</b>
        <span class="amt">${money(o.total)}</span></div>
      <div class="note">${esc(o.address?.line || '—')}</div>
      <button class="cta" data-take="${esc(o.id)}" style="margin-top:12px">Взяти</button>
    </div>`).join('')}
    <button class="ghost" id="endShift">Завершити зміну</button>`;
  $('#endShift').onclick = () => setShift(false);
  document.querySelectorAll('[data-take]').forEach(b => b.onclick = async () => {
    b.disabled = true;
    try { await api(`/courier/orders/${encodeURIComponent(b.dataset.take)}/accept`, { method:'POST' }); await load(); }
    catch (e) { toast(String(e.message || e)); b.disabled = false; }
  });
}

function renderActive(o){
  const picked = o.status === 'IN_DELIVERY';
  const addr = o.address?.line || '';
  const cash = o.payment === 'cash' ? o.total : 0;
  $('#app').innerHTML = `
    <h2>${picked ? 'Доставляєте' : 'Заберіть замовлення'}</h2>
    <p class="sub">#${esc(String(o.id).slice(0,8))} · ${o.items} поз.</p>
    <div class="addr">${esc(addr || '—')}</div>
    ${o.address?.note ? `<div class="note">${esc(o.address.note)}</div>` : ''}
    <div class="meta">
      ${cash ? `<span class="cash"><i class="ti ti-cash" aria-hidden="true"></i>${money(cash)}</span>`
             : `<span class="note">Оплачено онлайн</span>`}
      ${o.contact?.phone ? `<a class="note" href="tel:${esc(o.contact.phone)}">${esc(o.contact.phone)}</a>` : ''}
    </div>
    ${picked
      ? `<button class="cta go" id="done">Доставлено</button>`
      : `<button class="cta" id="pick">Забрав</button>`}
    ${addr ? `<a class="ghost" target="_blank" rel="noopener"
        href="https://www.openstreetmap.org/search?query=${encodeURIComponent(addr)}">Відкрити в картах</a>` : ''}
    ${o.contact?.phone ? `<a class="ghost" href="tel:${esc(o.contact.phone)}">Подзвонити клієнту</a>` : ''}`;

  if ($('#pick')) $('#pick').onclick = async () => {
    $('#pick').disabled = true;
    try { await api(`/courier/orders/${encodeURIComponent(o.id)}/pickup`, { method:'POST' }); await load(); }
    catch (e) { toast(String(e.message || e)); $('#pick').disabled = false; }
  };
  if ($('#done')) $('#done').onclick = async () => {
    let collected = cash;
    if (cash) {
      // Short handovers happen. Record what was actually taken rather than
      // assume the full amount -- this number settles disputes later.
      const v = prompt(`Скільки готівки отримано? (до сплати ${money(cash)})`, String(cash));
      if (v === null) return;
      collected = parseInt(v, 10);
      if (!Number.isFinite(collected) || collected < 0) return toast('Некоректна сума');
    }
    $('#done').disabled = true;
    try {
      const d = await api(`/courier/orders/${encodeURIComponent(o.id)}/deliver`, {
        method:'POST', body: JSON.stringify({ cash_collected: collected }) });
      if (d.short > 0) toast(`Недостача ${money(d.short)} — записано`);
      else toast('Доставлено');
      await load();
    } catch (e) { toast(String(e.message || e)); $('#done').disabled = false; }
  };
}

async function setShift(open){
  try { await api('/courier/shift', { method:'POST', body: JSON.stringify({ open }) }); await load(); }
  catch (e) { toast(String(e.message || e)); }
}

async function boot(){ S.booted = true; initMap(); await load();
  clearInterval(boot._i);
  boot._i = setInterval(() => { if (!document.hidden && S.booted) load(); }, 12000); }

store.t ? boot() : renderLogin();
