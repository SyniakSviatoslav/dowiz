// Owner console. Everything here is a view over server state: the queue, the
// numbers and the menu are re-read, never accumulated locally. No status chain
// lives in this file -- an action names an intent and the server's FSM answers.
const API = '/api';
const $ = (s, r = document) => r.querySelector(s);
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));

const store = {
  get t(){ try { return sessionStorage.getItem('dw_at'); } catch { return null; } },
  set t(v){ try { v ? sessionStorage.setItem('dw_at', v) : sessionStorage.removeItem('dw_at'); } catch {} },
  // The access token lives in sessionStorage (gone when the tab closes); the
  // refresh token in localStorage so a reload does not force a re-login. The old
  // platform's storage rule, and the reason is the same: no cookies anywhere,
  // because Safari blocks third-party cookies in the embed iframe.
  get r(){ try { return localStorage.getItem('dw_rt'); } catch { return null; } },
  set r(v){ try { v ? localStorage.setItem('dw_rt', v) : localStorage.removeItem('dw_rt'); } catch {} },
  get loc(){ try { return localStorage.getItem('dw_loc'); } catch { return null; } },
  set loc(v){ try { v ? localStorage.setItem('dw_loc', v) : localStorage.removeItem('dw_loc'); } catch {} },
};

let S = { tab:'orders', orders:[], stats:null, products:[], venue:null, seen:new Set(), booted:false };

function toast(m){ const el = $('#toast'); el.textContent = m; el.classList.add('show');
  clearTimeout(toast._t); toast._t = setTimeout(() => el.classList.remove('show'), 2600); }

const money = n => new Intl.NumberFormat('uk', { style:'currency', currency:'ALL', maximumFractionDigits:0 }).format(n || 0);

// One fetch wrapper so a 401 has exactly one meaning everywhere: the session is
// over. It tries a refresh once, then stops -- retrying forever on a dead token
// is how a console ends up hammering its own API.
async function api(path, opts = {}, retried = false){
  const r = await fetch(API + path, {
    ...opts,
    headers: { 'content-type':'application/json', ...(opts.headers || {}),
               ...(store.t ? { authorization:'Bearer ' + store.t } : {}) },
  });
  if (r.status === 401 && !retried && store.r) {
    const ok = await refresh();
    if (ok) return api(path, opts, true);
    logout(); throw new Error('session expired');
  }
  if (!r.ok) {
    let msg = 'HTTP ' + r.status;
    try { const d = await r.json(); msg = d.error || d.message || msg; } catch {}
    throw new Error(msg);
  }
  return r.status === 204 ? null : r.json();
}

async function refresh(){
  try {
    const r = await fetch(API + '/auth/refresh', { method:'POST',
      headers:{ 'content-type':'application/json' }, body: JSON.stringify({ refresh_token: store.r }) });
    if (r.status === 409) return true;            // concurrent refresh: the other tab won
    if (!r.ok) return false;
    const d = await r.json();
    store.t = d.access_token; if (d.refresh_token) store.r = d.refresh_token;
    return true;
  } catch { return false; }
}

function logout(){ store.t = null; store.r = null; S.booted = false; $('#top').hidden = true; renderLogin(); }
$('#logout').onclick = async () => { try { await api('/auth/logout', { method:'POST' }); } catch {} logout(); };

// ── login ──
function renderLogin(err){
  $('#app').innerHTML = `<div class="login">
    <h1>dowiz</h1><p>Панель власника</p>
    <label for="e">Email</label><input id="e" type="email" autocomplete="username" inputmode="email">
    <label for="p">Пароль</label><input id="p" type="password" autocomplete="current-password">
    ${err ? `<div class="err">${esc(err)}</div>` : ''}
    <button class="btn pri" id="go" style="width:100%;margin-top:18px;min-height:46px">Увійти</button>
  </div>`;
  const submit = async () => {
    const b = $('#go'); b.disabled = true; b.textContent = 'Входимо…';
    try {
      const r = await fetch(API + '/auth/login', { method:'POST', headers:{ 'content-type':'application/json' },
        body: JSON.stringify({ email: $('#e').value.trim(), password: $('#p').value }) });
      const d = await r.json();
      if (!r.ok) throw new Error(d.error || d.message || 'HTTP ' + r.status);
      store.t = d.access_token; store.r = d.refresh_token; store.loc = d.user.locationId;
      boot();
    } catch (e) { renderLogin(String(e.message || e)); }
  };
  $('#go').onclick = submit;
  $('#p').onkeydown = e => { if (e.key === 'Enter') submit(); };
  $('#e').focus();
}

// ── shell ──
async function boot(){
  if (!store.t || !store.loc) return renderLogin();
  $('#top').hidden = false;
  S.booted = true;
  render();
  await Promise.all([loadStats(), loadOrders(), loadVenue()]);
  render();
  poll();
}

function render(){
  if (!S.booted) return;
  const s = S.stats;
  $('#app').innerHTML = `
    <div class="stats">
      <div class="stat"><div class="k">Замовлень сьогодні</div><div class="v">${s ? s.todayOrders : '—'}</div></div>
      <div class="stat"><div class="k">Чекають</div><div class="v">${s ? s.pending : '—'}</div></div>
      <div class="stat"><div class="k">В роботі</div><div class="v">${s ? s.active : '—'}</div></div>
      <div class="stat"><div class="k">Виручка</div><div class="v">${s ? money(s.todayRevenue) : '—'}</div></div>
    </div>
    <div class="tabs" role="tablist">
      <button class="tab" role="tab" data-t="orders" aria-selected="${S.tab==='orders'}">Замовлення</button>
      <button class="tab" role="tab" data-t="menu"   aria-selected="${S.tab==='menu'}">Меню</button>
    </div>
    <div id="pane"></div>`;
  document.querySelectorAll('.tab').forEach(b => b.onclick = () => {
    S.tab = b.dataset.t; render(); if (S.tab === 'menu' && !S.products.length) loadMenu();
  });
  $('#pane').innerHTML = S.tab === 'orders' ? ordersView() : menuView();
  if (S.tab === 'orders') bindOrders(); else bindMenu();
  paintVenue();
}

// ── orders ──
const LIVE = ['PENDING','CONFIRMED','PREPARING','READY','IN_DELIVERY'];
function ordersView(){
  const live = S.orders.filter(o => LIVE.includes(o.status));
  if (!live.length) return `<div class="empty"><b>Поки тихо</b>Нові замовлення з'являться тут автоматично</div>`;
  return live.map(card).join('');
}

function card(o){
  const items = (o.items || []).map(i => `<b>${i.quantity}×</b> ${esc(shortId(i.product_id))}`).join(', ');
  const f = o.fulfilment || {}, c = o.contact || {};
  return `<div class="card ${o.status === 'PENDING' ? 'pending' : ''}">
    <div class="card-h">
      <span class="oid">#${esc(String(o.id).slice(0,8))}</span>
      <span class="chip ${esc(o.status)}">${esc(o.status)}</span>
      <span class="amt">${money(o.total)}</span>
    </div>
    <div class="lines">${items || '—'}</div>
    <div class="who">
      ${c.phone ? `<a href="tel:${esc(c.phone)}">${esc(c.phone)}</a>` : ''}
      ${c.name ? `<span>${esc(c.name)}</span>` : ''}
      ${f.address?.line ? `<span>${esc(f.address.line)}</span>` : ''}
      ${f.address?.note ? `<span style="color:var(--muted)">${esc(f.address.note)}</span>` : ''}
      <span style="color:var(--muted)">${o.payment === 'cash' ? 'готівка' : esc(o.payment || '')}</span>
    </div>
    <div class="acts">${actions(o)}</div>
  </div>`;
}
const shortId = id => String(id || '').slice(0, 8);

// The buttons offered are the ones that make sense next. The SERVER still
// decides: an action the FSM refuses comes back 409 and the card does not move.
function actions(o){
  const b = (a, label, cls = '') => `<button class="btn ${cls}" data-o="${esc(o.id)}" data-a="${a}">${label}</button>`;
  switch (o.status) {
    case 'PENDING':   return b('confirm','Підтвердити','pri') + b('reject','Відхилити','dan');
    case 'CONFIRMED': return b('preparing','Готуємо','pri') + b('cancel','Скасувати','dan');
    case 'PREPARING': return b('ready','Готове','pri') + b('cancel','Скасувати','dan');
    case 'READY':     return `<span style="color:var(--muted);font-size:13px">Чекає кур'єра</span>`;
    default:          return '';
  }
}

function bindOrders(){
  document.querySelectorAll('[data-a]').forEach(btn => btn.onclick = async () => {
    const { o: id, a: action } = btn.dataset;
    let reason = null;
    if (action === 'reject') {
      reason = prompt('Причина відмови (побачить клієнт):', 'Немає в наявності');
      if (reason === null) return;
    }
    document.querySelectorAll('[data-o="' + CSS.escape(id) + '"]').forEach(x => x.disabled = true);
    try {
      await api(`/owner/orders/${encodeURIComponent(id)}/action`, { method:'POST',
        body: JSON.stringify({ location_id: store.loc, action, reason }) });
      await Promise.all([loadOrders(), loadStats()]);
      render();
    } catch (e) {
      toast(String(e.message || e));
      document.querySelectorAll('[data-o="' + CSS.escape(id) + '"]').forEach(x => x.disabled = false);
    }
  });
}

async function loadOrders(){
  try {
    const d = await api(`/owner/orders?location_id=${encodeURIComponent(store.loc)}`);
    const fresh = (d.orders || []).filter(o => o.status === 'PENDING' && !S.seen.has(o.id));
    (d.orders || []).forEach(o => S.seen.add(o.id));
    S.orders = d.orders || [];
    if (fresh.length && S.stats) alert_new(fresh.length);
  } catch (e) { if (String(e.message) !== 'session expired') toast(String(e.message || e)); }
}
async function loadStats(){ try { S.stats = await api(`/owner/dashboard?location_id=${encodeURIComponent(store.loc)}`); } catch {} }

// A new order during a rush must be HEARD, not noticed. iOS will not play audio
// until a user gesture has unlocked the context, so the context is created lazily
// on the first interaction and reused.
let actx = null;
addEventListener('pointerdown', () => { if (!actx) { try { actx = new (AudioContext || webkitAudioContext)(); } catch {} } }, { once:true });
function alert_new(n){
  toast(n === 1 ? 'Нове замовлення' : `Нових замовлень: ${n}`);
  try {
    if (!actx) return;
    const t = actx.currentTime, o = actx.createOscillator(), g = actx.createGain();
    o.type = 'sine'; o.frequency.setValueAtTime(880, t); o.frequency.setValueAtTime(1175, t + 0.12);
    g.gain.setValueAtTime(0.0001, t); g.gain.exponentialRampToValueAtTime(0.28, t + 0.02);
    g.gain.exponentialRampToValueAtTime(0.0001, t + 0.45);
    o.connect(g).connect(actx.destination); o.start(t); o.stop(t + 0.5);
  } catch {}
  if (navigator.vibrate) try { navigator.vibrate([90, 60, 90]); } catch {}
}

// ── menu ──
async function loadMenu(){
  try {
    const d = await fetch(`${API}/public/locations/demo/menu`).then(r => r.json());
    S.products = (d.categories || []).flatMap(c => (c.products || []).map(p => ({ ...p, cat: c.name })));
    S.venue = d.location; render();
  } catch (e) { toast(String(e.message || e)); }
}
function menuView(){
  if (!S.products.length) return `<div class="skel"></div><div class="skel"></div>`;
  let cat = null; const out = [];
  for (const p of S.products) {
    if (p.cat !== cat) { cat = p.cat; out.push(`<div class="card" style="padding:10px 14px;font-weight:600">${esc(cat)}</div>`); }
    out.push(`<div class="card" style="padding:0 14px"><div class="prod">
      <span class="n"><b>${esc(p.name)}</b><small>${p.available ? 'у продажу' : esc(p.unavailableNote || 'зупинено')}</small></span>
      <input type="number" min="0" step="1" value="${p.price}" data-price="${esc(p.id)}" aria-label="Ціна">
      <label class="sw" title="У продажу"><input type="checkbox" data-av="${esc(p.id)}" ${p.available ? 'checked' : ''}><span></span></label>
    </div></div>`);
  }
  return out.join('');
}
function bindMenu(){
  document.querySelectorAll('[data-av]').forEach(el => el.onchange = async () => {
    const id = el.dataset.av, available = el.checked;
    let note = null;
    if (!available) note = prompt('Причина (побачить клієнт):', 'Немає сьогодні') || null;
    try {
      await api(`/owner/products/${encodeURIComponent(id)}`, { method:'POST',
        body: JSON.stringify({ location_id: store.loc, available, unavailable_note: note }) });
      const p = S.products.find(x => x.id === id); if (p) { p.available = available; p.unavailableNote = note; }
      toast(available ? 'У продажу' : 'Зупинено'); render();
    } catch (e) { el.checked = !available; toast(String(e.message || e)); }
  });
  document.querySelectorAll('[data-price]').forEach(el => el.onchange = async () => {
    const id = el.dataset.price, price = parseInt(el.value, 10);
    if (!Number.isFinite(price) || price < 0) { toast('Некоректна ціна'); return; }
    try {
      await api(`/owner/products/${encodeURIComponent(id)}`, { method:'POST',
        body: JSON.stringify({ location_id: store.loc, price }) });
      const p = S.products.find(x => x.id === id); if (p) p.price = price;
      toast('Ціну оновлено');
    } catch (e) { toast(String(e.message || e)); }
  });
}

// ── venue status ──
async function loadVenue(){
  try { const d = await fetch(`${API}/public/locations/demo/menu`).then(r => r.json()); S.venue = d.location; paintVenue(); } catch {}
}
function paintVenue(){
  const b = $('#vstatus'); if (!b || !S.venue) return;
  const s = S.venue.status;
  b.className = 'vstatus ' + (s === 'open' ? 'open' : s === 'busy' ? 'busy' : '');
  $('#vstatusT').textContent = s === 'open' ? 'Відкрито' : s === 'busy' ? 'Завантажені' : 'Зачинено';
  b.onclick = async () => {
    const next = s === 'open' ? 'busy' : s === 'busy' ? 'closed' : 'open';
    // Closing stops new orders reaching the kitchen. That is not a thing to do
    // by mis-tap, so it asks.
    if (next === 'closed' && !confirm('Зачинити заклад? Нові замовлення не надходитимуть.')) return;
    try {
      await api('/owner/location', { method:'POST', body: JSON.stringify({ location_id: store.loc, status: next }) });
      S.venue.status = next; paintVenue(); toast('Статус: ' + next);
    } catch (e) { toast(String(e.message || e)); }
  };
}

// Poll while the tab is visible. A background tab does not need to hammer the
// API, and Page Visibility is the cheap way to know.
function poll(){
  clearInterval(poll._i);
  poll._i = setInterval(async () => {
    if (document.hidden || !S.booted) return;
    await Promise.all([loadOrders(), loadStats()]);
    if (S.tab === 'orders') { $('#pane').innerHTML = ordersView(); bindOrders(); }
    const s = S.stats;
    if (s) document.querySelectorAll('.stat .v').forEach((el, i) =>
      el.textContent = [s.todayOrders, s.pending, s.active, money(s.todayRevenue)][i]);
  }, 10000);
}
document.addEventListener('visibilitychange', () => { if (!document.hidden && S.booted) loadOrders().then(() => render()); });

store.t ? boot() : renderLogin();
