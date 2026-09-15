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

let S = { tab:'orders', orders:[], stats:null, products:[], venue:null, couriers:[], seen:new Set(), fresh:new Set(), booted:false };

function toast(m){ const el = $('#toast'); el.textContent = m; el.classList.add('show');
  clearTimeout(toast._t); toast._t = setTimeout(() => el.classList.remove('show'), 2600); }

// Money is an integer the server sent, formatted and SET as text. There is no
// animated path to an amount anywhere in this file (DESIGN plan §2.5).
const money = n => new Intl.NumberFormat('uk', { style:'currency', currency:'ALL', maximumFractionDigits:0 }).format(n || 0);

// ── SEA ─────────────────────────────────────────────────────────────────────
// The dowiz ambient layer, the same shipped module the storefront wires
// (/lib/particle-cloud.js). The design plan's owner Act 1 is "the field IS the
// business: each order a ripple, volume = amplitude". So: a new order bursts
// amber, a queue that is still waiting drifts as ember, a row marked ready
// streams teal, a rejection is turbulence. Counts are lower than the
// storefront's -- this is a tool, and the field is weather behind the window.
// The Sea carries NO text, NO price and NO decision; every status sits on an
// opaque surface above it, so nothing the owner must read can be obscured.
let sea = null;
async function initSea(){
  if (sea) return;
  const cv = document.getElementById('sea'); if (!cv) return;
  // Reduced motion is a calm sea, not no sea: the module quarters its bursts
  // and the canvas fades to its calm opacity.
  const calm = matchMedia('(prefers-reduced-motion: reduce)').matches;
  try {
    const { createParticleCloud } = await import('/lib/particle-cloud.js');
    sea = createParticleCloud();
    sea.init(cv);
    sea.setReducedMotion(calm);
    cv.classList.toggle('calm', calm);
    addEventListener('resize', () => sea.resize(), { passive:true });
    if (matchMedia('(hover: hover) and (pointer: fine)').matches) {
      addEventListener('pointermove', e => sea.setPointer(e.clientX / innerWidth, e.clientY / innerHeight), { passive:true });
    }
  } catch {
    // No WebGL2 or the module failed: the console is fully usable without the
    // Sea. It is atmosphere, never a dependency.
    sea = null;
  }
}
function seaEvent(kind, n){ try { sea && sea.burst(kind, n); } catch {} }
// What an owner ACTION does to the field. The server still decides the state;
// the ripple is fired only after the server said yes.
const SEA_FOR_ACTION = { confirm:['pending_aging', 24], preparing:['pending_aging', 24],
                         ready:['courier_assigned', 40], reject:['dispatch_failed', 40], cancel:['dispatch_failed', 40] };

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
    ${err ? `<div class="err" role="alert"><i class="ti ti-alert-circle i" aria-hidden="true"></i><span>${esc(err)}</span></div>` : ''}
    <button class="btn pri wide" id="go">Увійти</button>
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
  await Promise.all([loadStats(), loadOrders(), loadVenue(), loadCouriers()]);
  render();
  poll();
}

const LIVE = ['PENDING','CONFIRMED','PREPARING','READY','IN_DELIVERY'];
const liveOrders = () => S.orders.filter(o => LIVE.includes(o.status));

function render(){
  if (!S.booted) return;
  const s = S.stats;
  $('#app').innerHTML = `
    <div class="stats">
      <div class="stat"><div class="k">Замовлень сьогодні</div><div class="v" data-k="todayOrders">${s ? s.todayOrders : '—'}</div></div>
      <div class="stat"><div class="k">Чекають</div><div class="v" data-k="pending">${s ? s.pending : '—'}</div></div>
      <div class="stat"><div class="k">В роботі</div><div class="v" data-k="active">${s ? s.active : '—'}</div></div>
      <div class="stat"><div class="k">Виручка</div><div class="v" data-k="todayRevenue">${s ? money(s.todayRevenue) : '—'}</div></div>
    </div>
    <div class="tabs" role="tablist" aria-label="Розділи">
      <button class="tab" role="tab" id="tab-orders" aria-controls="pane" data-t="orders" aria-selected="${S.tab==='orders'}" tabindex="${S.tab==='orders' ? 0 : -1}">Замовлення <span class="n" id="liveN">${liveOrders().length}</span></button>
      <button class="tab" role="tab" id="tab-menu"   aria-controls="pane" data-t="menu"   aria-selected="${S.tab==='menu'}"   tabindex="${S.tab==='menu' ? 0 : -1}">Меню</button>
    </div>
    <div id="pane" role="tabpanel" aria-labelledby="tab-${S.tab}"></div>`;
  const tabs = [...document.querySelectorAll('.tab')];
  tabs.forEach((b, i) => {
    b.onclick = () => { S.tab = b.dataset.t; render(); if (S.tab === 'menu' && !S.products.length) loadMenu(); };
    // Keyboard: arrows move between tabs, as a tablist is expected to.
    b.onkeydown = e => {
      if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return;
      const n = tabs[(i + (e.key === 'ArrowRight' ? 1 : tabs.length - 1)) % tabs.length];
      n.focus(); n.click(); e.preventDefault();
    };
  });
  $('#pane').innerHTML = S.tab === 'orders' ? ordersView() : menuView();
  if (S.tab === 'orders') bindOrders(); else bindMenu();
  S.fresh.clear();                                // the entrance runs once, on the render that introduced the row
  paintVenue();
}

// ── orders ──
const STATUS_LABEL = { PENDING:'Нове', CONFIRMED:'Підтверджено', PREPARING:'Готується', READY:'Готове',
                       IN_DELIVERY:'В дорозі', DELIVERED:'Доставлено', REJECTED:'Відхилено', CANCELLED:'Скасовано',
                       SCHEDULED:'Заплановано', PICKED_UP:'Забрано' };
function ordersView(){
  const live = liveOrders();
  if (!live.length) return `<div class="panel"><div class="empty"><i class="ti ti-inbox i" aria-hidden="true"></i><b>Поки тихо</b>Нові замовлення з'являться тут автоматично</div></div>`;
  let i = 0;
  return `<div class="panel">${live.map(o => row(o, S.fresh.has(o.id) ? i++ : -1)).join('')}</div>`;
}

function row(o, newIdx){
  const items = (o.items || []).map(i => `<b>${i.quantity}×</b> ${esc(i.name || shortId(i.product_id))}`).join(', ');
  const f = o.fulfilment || {}, c = o.contact || {};
  const st = esc(o.status);
  return `<article class="order ${o.status === 'PENDING' ? 'attn' : ''} ${newIdx >= 0 ? 'is-new' : ''}" ${newIdx >= 0 ? `style="--i:${newIdx}"` : ''}>
    <div class="o-h">
      <span class="oid">#${esc(String(o.id).slice(0,8))}</span>
      <span class="chip ${st}"><i aria-hidden="true"></i>${esc(STATUS_LABEL[o.status] || o.status)}</span>
      <span class="amt">${money(o.total)}</span>
    </div>
    <p class="lines">${items || '—'}</p>
    <div class="who">
      ${c.phone ? `<a class="tel" href="tel:${esc(c.phone)}"><i class="ti ti-phone i" aria-hidden="true"></i>${esc(c.phone)}</a>` : ''}
      ${c.name ? `<span><i class="ti ti-user i" aria-hidden="true"></i>${esc(c.name)}</span>` : ''}
      ${f.address?.line ? `<span><i class="ti ti-map-pin i" aria-hidden="true"></i>${esc(f.address.line)}</span>` : ''}
      ${f.address?.note ? `<span class="muted"><i class="ti ti-note i" aria-hidden="true"></i>${esc(f.address.note)}</span>` : ''}
      <span class="muted"><i class="ti ${o.payment === 'cash' ? 'ti-cash' : 'ti-credit-card'} i" aria-hidden="true"></i>${o.payment === 'cash' ? 'готівка' : esc(o.payment || '')}</span>
    </div>
    <div class="acts">${actions(o)}</div>
  </article>`;
}
const shortId = id => String(id || '').slice(0, 8);

// The buttons offered are the ones that make sense next. The SERVER still
// decides: an action the FSM refuses comes back 409 and the row does not move.
function actions(o){
  const b = (a, label, cls = '', icon = '') =>
    `<button class="btn ${cls}" data-o="${esc(o.id)}" data-a="${a}">${icon ? `<i class="ti ${icon} i" aria-hidden="true"></i>` : ''}${label}</button>`;
  switch (o.status) {
    case 'PENDING':   return b('confirm','Підтвердити','pri','ti-check') + b('reject','Відхилити','dan','ti-x');
    case 'CONFIRMED': return b('preparing','Готуємо','pri','ti-flame') + b('cancel','Скасувати','dan');
    case 'PREPARING': return b('ready','Готове','pri','ti-package') + b('cancel','Скасувати','dan');
    // A READY order is the kitchen's work finished and the courier's not yet
    // started. Until now this said "waiting for a courier" and offered no way to
    // get one, which is a status message standing in for a missing control.
    case 'READY':     return courierPicker(o);
    case 'IN_DELIVERY': return courierName(o);
    default:          return '';
  }
}

/// Who is carrying this order, if anyone.
function courierName(o){
  const c = S.couriers.find(x => x.id === o.courier_id);
  return o.courier_id
    ? `<span class="wait"><i class="ti ti-bike i" aria-hidden="true"></i>${esc(c ? c.name : o.courier_id)}</span>`
    : `<span class="wait"><i class="ti ti-bike i" aria-hidden="true"></i>Без кур'єра</span>`;
}

/// Hand the order to a courier.
///
/// Couriers who are ON SHIFT come first and are the only ones enabled: assigning
/// an order to a phone that is switched off looks exactly like a lost order to
/// the person waiting for it. When nobody is on shift the control says so rather
/// than presenting an empty menu.
function courierPicker(o){
  if (o.courier_id) return courierName(o);
  const on = S.couriers.filter(c => c.active && c.onShift);
  if (!on.length) return `<span class="wait"><i class="ti ti-bike i" aria-hidden="true"></i>Немає кур'єрів на зміні</span>`;
  return `<label class="assign">
      <span class="sr">Призначити кур&#39;єра</span>
      <select data-assign="${esc(o.id)}">
        <option value="">Призначити кур'єра…</option>
        ${on.map(c => `<option value="${esc(c.id)}">${esc(c.name)}</option>`).join('')}
      </select>
    </label>`;
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
      const ev = SEA_FOR_ACTION[action]; if (ev) seaEvent(ev[0], ev[1]);
      await Promise.all([loadOrders(), loadStats(), loadCouriers()]);
      render();
    } catch (e) {
      toast(String(e.message || e));
      document.querySelectorAll('[data-o="' + CSS.escape(id) + '"]').forEach(x => x.disabled = false);
    }
  });

  document.querySelectorAll('[data-assign]').forEach(sel => {
    sel.onchange = async () => {
      const id = sel.dataset.assign, courier_id = sel.value;
      if (!courier_id) return;
      sel.disabled = true;
      try {
        await api(`/owner/orders/${encodeURIComponent(id)}/assign`, { method:'POST',
          body: JSON.stringify({ courier_id }) });
        seaEvent('courier_assigned', 40);
        toast('Кур\u2019єра призначено');
        await loadOrders(); render();
      } catch (e) {
        // Put the control back where it was: a select left showing a courier
        // who was never assigned is a lie the owner will act on.
        sel.value = ''; sel.disabled = false;
        toast(String(e.message || e));
      }
    };
  });
}

async function loadOrders(){
  try {
    const d = await api(`/owner/orders?location_id=${encodeURIComponent(store.loc)}`);
    const first = S.seen.size === 0;            // the first load is not "new orders", it is the queue
    const fresh = (d.orders || []).filter(o => o.status === 'PENDING' && !S.seen.has(o.id));
    (d.orders || []).forEach(o => S.seen.add(o.id));
    S.orders = d.orders || [];
    if (!first) fresh.forEach(o => S.fresh.add(o.id));
    if (fresh.length && S.stats) alert_new(fresh.length);
  } catch (e) { if (String(e.message) !== 'session expired') toast(String(e.message || e)); }
}
async function loadStats(){ try { S.stats = await api(`/owner/dashboard?location_id=${encodeURIComponent(store.loc)}`); } catch {} }
// Who is available to carry an order. Failing quietly is right here: a missing
// courier list must not blank the order queue, which is the thing the owner
// actually needs on screen.
async function loadCouriers(){ try { S.couriers = (await api('/owner/couriers')).couriers || []; } catch {} }

// A new order during a rush must be HEARD, not noticed. iOS will not play audio
// until a user gesture has unlocked the context, so the context is created lazily
// on the first interaction and reused.
let actx = null;
addEventListener('pointerdown', () => { if (!actx) { try { actx = new (AudioContext || webkitAudioContext)(); } catch {} } }, { once:true });
function alert_new(n){
  toast(n === 1 ? 'Нове замовлення' : `Нових замовлень: ${n}`);
  seaEvent('order_created', 48 * Math.min(n, 3));
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
// One panel per category, product ROWS inside it. The old view put a card
// inside a card for every dish; nested cards are always wrong.
function menuView(){
  if (!S.products.length) return `<div class="skel"></div><div class="skel"></div>`;
  let cat = null; const out = [];
  for (const p of S.products) {
    if (p.cat !== cat) {
      if (cat !== null) out.push('</div>');
      cat = p.cat;
      out.push(`<div class="panel"><h2 class="panel-h"><i class="ti ti-category i" aria-hidden="true"></i>${esc(cat)}</h2>`);
    }
    out.push(`<div class="prod ${p.available ? '' : 'off'}">
      <span class="n"><b>${esc(p.name)}</b><small>${p.available ? 'у продажу' : esc(p.unavailableNote || 'зупинено')}</small></span>
      <input type="number" min="0" step="1" value="${p.price}" data-price="${esc(p.id)}" aria-label="Ціна, ${esc(p.name)}">
      <label class="sw" title="У продажу"><input type="checkbox" data-av="${esc(p.id)}" aria-label="У продажу, ${esc(p.name)}" ${p.available ? 'checked' : ''}><span></span></label>
    </div>`);
  }
  if (cat !== null) out.push('</div>');
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
    b.disabled = true;
    try {
      await api('/owner/location', { method:'POST', body: JSON.stringify({ location_id: store.loc, status: next }) });
      S.venue.status = next; toast('Статус: ' + next);
    } catch (e) { toast(String(e.message || e)); }
    b.disabled = false; paintVenue();
  };
}

// Poll while the tab is visible. A background tab does not need to hammer the
// API, and Page Visibility is the cheap way to know.
function poll(){
  clearInterval(poll._i);
  poll._i = setInterval(async () => {
    if (document.hidden || !S.booted) return;
    await Promise.all([loadOrders(), loadStats(), loadCouriers()]);
    if (S.tab === 'orders') { $('#pane').innerHTML = ordersView(); bindOrders(); S.fresh.clear(); }
    const s = S.stats;
    if (s) {
      // Text SET, never tweened -- the revenue is a kernel integer presented, not interpolated.
      const v = { todayOrders:s.todayOrders, pending:s.pending, active:s.active, todayRevenue:money(s.todayRevenue) };
      document.querySelectorAll('.stat .v[data-k]').forEach(el => { el.textContent = v[el.dataset.k]; });
      const n = $('#liveN'); if (n) n.textContent = liveOrders().length;
      // A queue still waiting is ember drift in the field: volume = amplitude.
      if (s.pending > 0) seaEvent('pending_aging', 8 * Math.min(s.pending, 3));
    }
  }, 10000);
}
document.addEventListener('visibilitychange', () => { if (!document.hidden && S.booted) loadOrders().then(() => render()); });

initSea();
store.t ? boot() : renderLogin();
