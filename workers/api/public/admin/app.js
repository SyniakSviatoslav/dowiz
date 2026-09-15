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

// `phase` is what stops an empty state from lying. 'loading' is NOT 'nothing
// here' -- an owner shown "no orders" during the first fetch believes the
// kitchen is quiet, which during a rush is the worst thing this screen can say.
let S = { tab:'orders', orders:[], stats:null, products:[], venue:null, couriers:[],
          phase:'loading', error:null, menuPhase:'idle', menuError:null,
          seen:new Set(), fresh:new Set(), booted:false };

function toast(m){ const el = $('#toast'); el.textContent = m; el.classList.add('show');
  clearTimeout(toast._t); toast._t = setTimeout(() => el.classList.remove('show'), 2600); }

// Money is an integer the server sent, formatted and SET as text. There is no
// animated path to an amount anywhere in this file (DESIGN plan §2.5).
const money = n => new Intl.NumberFormat('uk', { style:'currency', currency:'ALL', maximumFractionDigits:0 }).format(n || 0);

// EVERY button that waits on the network shows that it is waiting.
//
// Disabling alone is ambiguous -- a greyed control reads as "not allowed" as
// readily as "working" -- so the label is replaced by a spinner and restored
// afterwards, and `aria-busy` says the same thing to a screen reader. The
// original label is kept on the element rather than in a closure so that a
// re-render between start and finish cannot lose it.
async function busy(el, fn){
  if (!el || el.disabled) return;
  const had = el.innerHTML;
  el.disabled = true;
  el.setAttribute('aria-busy', 'true');
  el.innerHTML = `<i class="ti ti-loader-2 i spin" aria-hidden="true"></i>`;
  try { return await fn(); }
  finally {
    el.disabled = false;
    el.removeAttribute('aria-busy');
    el.innerHTML = had;
  }
}

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
  S.phase = 'loading';
  render();                                   // skeletons, not an empty queue
  await reload();
  poll();
}

// One load, one verdict. `loadOrders` is the one that decides the phase: the
// queue is what this screen exists for, and stats or the courier list failing
// is a degraded pane rather than a broken one.
async function reload(){
  try {
    await loadOrders({ strict:true });
    S.phase = 'ready'; S.error = null;
  } catch (e) {
    S.phase = 'error';
    S.error = String(e.message || e);
  }
  // These three may fail quietly: none of them is the reason the owner opened
  // this page, and blanking the queue because the courier list timed out would
  // be trading the important thing for the incidental one.
  await Promise.all([loadStats(), loadVenue(), loadCouriers()]);
  render();
}

const LIVE = ['PENDING','CONFIRMED','PREPARING','READY','IN_DELIVERY'];
const liveOrders = () => S.orders.filter(o => LIVE.includes(o.status));

function render(){
  if (!S.booted) return;
  const s = S.stats;
  $('#app').innerHTML = `
    <div class="stats">
      ${[['todayOrders','Замовлень сьогодні'],['pending','Чекають'],
         ['active','В роботі'],['todayRevenue','Виручка']].map(([k, label]) => `
        <div class="stat"><div class="k">${label}</div>
          <div class="v" data-k="${k}">${s
            ? (k === 'todayRevenue' ? money(s[k]) : s[k])
            : `<span class="skel" style="display:inline-block;width:3rem;height:1.4rem;vertical-align:-.2em"></span>`}</div></div>`).join('')}
    </div>
    <div class="tabs" role="tablist" aria-label="Розділи">
      <button class="tab" role="tab" id="tab-orders" aria-controls="pane" data-t="orders" aria-selected="${S.tab==='orders'}" tabindex="${S.tab==='orders' ? 0 : -1}">Замовлення <span class="n" id="liveN">${liveOrders().length}</span></button>
      <button class="tab" role="tab" id="tab-menu"   aria-controls="pane" data-t="menu"   aria-selected="${S.tab==='menu'}"   tabindex="${S.tab==='menu' ? 0 : -1}">Меню</button>
      <button class="tab" role="tab" id="tab-setup" aria-controls="pane" data-t="setup" aria-selected="${S.tab==='setup'}" tabindex="${S.tab==='setup' ? 0 : -1}">Налаштування</button>
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
  $('#pane').innerHTML = S.tab === 'orders' ? ordersView() : S.tab === 'menu' ? menuView() : setupView();
  if (S.tab === 'orders') bindOrders(); else if (S.tab === 'menu') bindMenu(); else bindSetup();
  const retry = $('#retry');
  if (retry) retry.onclick = async () => {
    retry.disabled = true; S.phase = 'loading'; render(); await reload();
  };
  const retryMenu = $('#retryMenu');
  if (retryMenu) retryMenu.onclick = () => loadMenu();
  const toSetup = $('#toSetup');
  if (toSetup) toSetup.onclick = () => { S.tab = 'setup'; render(); };
  S.fresh.clear();                                // the entrance runs once, on the render that introduced the row
  paintVenue();
}

// ── orders ──
const STATUS_LABEL = { PENDING:'Нове', CONFIRMED:'Підтверджено', PREPARING:'Готується', READY:'Готове',
                       IN_DELIVERY:'В дорозі', DELIVERED:'Доставлено', REJECTED:'Відхилено', CANCELLED:'Скасовано',
                       SCHEDULED:'Заплановано', PICKED_UP:'Забрано' };
function ordersView(){
  // ORDER MATTERS. Loading and error are checked BEFORE emptiness, because an
  // empty list is only meaningful once we know the list arrived.
  if (S.phase === 'loading') return skeletonOrders();
  if (S.phase === 'error') return `
    <div class="panel"><div class="empty" role="alert">
      <i class="ti ti-alert-triangle i" aria-hidden="true"></i>
      <b>Не вдалося завантажити замовлення</b>
      <span class="reason">${esc(S.error || '')}</span>
      <button class="btn" id="retry" style="margin-top:12px">Спробувати ще раз</button>
    </div></div>`;
  const live = liveOrders();
  if (!live.length) return `<div class="panel"><div class="empty"><i class="ti ti-inbox i" aria-hidden="true"></i><b>Поки тихо</b>Нові замовлення з'являться тут автоматично</div></div>`;
  let i = 0;
  return `<div class="panel">${live.map(o => row(o, S.fresh.has(o.id) ? i++ : -1)).join('')}</div>`;
}

// Placeholders shaped like the rows they stand in for -- same height, same
// three bands -- so the queue does not jump when the real orders land.
function skeletonMenu(){
  return `<div class="panel" aria-busy="true" aria-label="Завантажуємо меню">
    <div class="panel-h"><span class="skel" style="width:8rem;height:1rem"></span></div>
    ${`<div class="prod">
        <span class="n"><span class="skel" style="width:9rem;height:1rem"></span></span>
        <span class="skel" style="width:5rem;height:var(--tap)"></span>
        <span class="skel" style="width:3rem;height:1.5rem"></span>
      </div>`.repeat(4)}
  </div>`;
}

function skeletonOrders(){
  return `<div class="panel" aria-busy="true" aria-label="Завантажуємо замовлення">
    ${`<article class="order sk">
        <div class="o-h"><span class="skel" style="width:5rem;height:1rem"></span>
          <span class="skel" style="width:7rem;height:1.5rem"></span>
          <span class="skel" style="width:4rem;height:1rem;margin-left:auto"></span></div>
        <div class="skel" style="width:70%;height:1rem;margin:10px 0"></div>
        <div class="skel" style="width:45%;height:1rem"></div>
      </article>`.repeat(3)}
  </div>`;
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
    // Every button on THIS order goes dead together -- confirming and rejecting
    // the same order are mutually exclusive, and a second tap during the first
    // request is how an order gets confirmed twice. The one that was pressed
    // shows the spinner, so it is clear WHICH action is in flight.
    const siblings = [...document.querySelectorAll('[data-o="' + CSS.escape(id) + '"]')]
      .filter(x => x !== btn);
    siblings.forEach(x => { x.disabled = true; });
    try {
      await busy(btn, () => api(`/owner/orders/${encodeURIComponent(id)}/action`, { method:'POST',
        body: JSON.stringify({ location_id: store.loc, action, reason }) }));
      const ev = SEA_FOR_ACTION[action]; if (ev) seaEvent(ev[0], ev[1]);
      await Promise.all([loadOrders(), loadStats(), loadCouriers()]);
      render();
    } catch (e) {
      toast(String(e.message || e));
      siblings.forEach(x => { x.disabled = false; });
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

async function loadOrders(opts = {}){
  try {
    const d = await api(`/owner/orders?location_id=${encodeURIComponent(store.loc)}`);
    const first = S.seen.size === 0;            // the first load is not "new orders", it is the queue
    const fresh = (d.orders || []).filter(o => o.status === 'PENDING' && !S.seen.has(o.id));
    (d.orders || []).forEach(o => S.seen.add(o.id));
    S.orders = d.orders || [];
    if (!first) fresh.forEach(o => S.fresh.add(o.id));
    if (fresh.length && S.stats) alert_new(fresh.length);
  } catch (e) {
    // `strict` is the first load, where a failure must become a visible state.
    // On a later poll it is a toast: the queue on screen is still the last
    // truth we had, and replacing it with an error panel would throw away
    // information the owner is actively using.
    if (opts.strict) throw e;
    if (String(e.message) !== 'session expired') toast(String(e.message || e));
  }
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
  S.menuPhase = 'loading'; S.menuError = null;
  if (S.tab === 'menu') render();
  try {
    const r = await fetch(`${API}/public/locations/demo/menu`);
    if (!r.ok) throw new Error('HTTP ' + r.status);
    const d = await r.json();
    S.products = (d.categories || []).flatMap(c => (c.products || []).map(p => ({ ...p, cat: c.name })));
    S.venue = d.location;
    S.menuPhase = 'ready';
  } catch (e) {
    S.menuPhase = 'error'; S.menuError = String(e.message || e);
  }
  render();
}
// One panel per category, product ROWS inside it. The old view put a card
// inside a card for every dish; nested cards are always wrong.
// ── setup ───────────────────────────────────────────────────────────────────
// Two jobs a venue does ONCE and then forgets: get the menu in, and make the
// storefront look like them. Both were previously a shell command on the VPS,
// which meant neither was something the owner could do.

function setupView(){
  const t = S.venue?.theme;
  return `
  <div class="panel setup">
    <section class="card">
      <h2>Меню з файлу</h2>
      <p class="hint">CSV із таблиці. Потрібні стовпці <b>Назва</b> та <b>Ціна</b>;
         <b>Розділ</b>, <b>Опис</b> і <b>Наявність</b> — за бажанням.
         Ціни — цілими числами, без копійок.</p>
      <div class="row">
        <label class="btn file">
          <i class="ti ti-file-spreadsheet i" aria-hidden="true"></i>
          <span>Обрати файл</span>
          <input type="file" id="csvFile" accept=".csv,text/csv,text/plain">
        </label>
        <span class="hint" id="csvName">Файл не обрано</span>
      </div>
      <div id="csvReport" class="report" hidden></div>
      <div class="row" id="csvApplyRow" hidden>
        <label class="check"><input type="checkbox" id="csvRetire">
          <span>Зняти з продажу те, чого немає у файлі</span></label>
        <button class="btn pri" id="csvApply">
          <i class="ti ti-upload i" aria-hidden="true"></i>Застосувати</button>
      </div>
    </section>

    <section class="card">
      <h2>Помічник</h2>
      <p class="hint">Працює на моделі, яку ви оберете. За замовчуванням — на цьому ж
         сервері: питання й дані не залишають вашу машину. Якщо вкажете хмарну
         модель, дані клієнтів (ім'я, телефон, адреса) до неї <b>не надсилаються</b>.</p>
      <div id="aiSettings" class="fields"></div>
      <div class="row">
        <input id="askBox" class="ask" type="text" placeholder="Запитайте про замовлення…"
               autocomplete="off" enterkeyhint="send">
        <button class="btn pri" id="askGo"><i class="ti ti-send i" aria-hidden="true"></i>Спитати</button>
      </div>
      <div id="answer" class="answer" hidden></div>
    </section>

    <section class="card">
      <h2>Ключі доступу</h2>
      <p class="hint">Для ваших власних застосунків і MCP-клієнтів. Ключ показуємо
         <b>один раз</b>. Кожен ключ можна відкликати окремо — решта працюватимуть.
         Адреса MCP: <code>${esc(location.origin)}/mcp</code></p>
      <div class="row">
        <input id="keyLabel" class="ask" type="text" placeholder="Для чого цей ключ…" autocomplete="off">
        <button class="btn" id="keyNew"><i class="ti ti-key i" aria-hidden="true"></i>Створити</button>
      </div>
      <div id="keyShown" class="report" hidden></div>
      <ul id="keyList" class="keys"></ul>
    </section>

    <section class="card">
      <h2>Кольори закладу</h2>
      <p class="hint">Завантажте логотип або фото меню — кольори візьмемо звідти.
         Контраст перевіряємо автоматично: нечитабельну пару не приймемо.</p>
      <div class="row">
        <label class="btn file">
          <i class="ti ti-photo i" aria-hidden="true"></i>
          <span>Обрати зображення</span>
          <input type="file" id="imgFile" accept="image/*">
        </label>
        <span class="hint" id="imgName">${t ? 'Поточний: ' + esc(t.seed) : 'Файл не обрано'}</span>
      </div>
      <div id="swatches" class="swatches" hidden></div>
      <div id="contrast" class="report" hidden></div>
    </section>
  </div>`;
}

/// Downsample an image file to a flat hex string of pixels.
///
/// The BROWSER decodes the image -- it already has a PNG and JPEG decoder, and
/// the server deliberately has neither. 64x64 is plenty to find dominant
/// colours and keeps the upload small; `drawImage` does the averaging.
function pixelsFromFile(file){
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => {
      try {
        const N = 64, c = document.createElement('canvas');
        c.width = N; c.height = N;
        const ctx = c.getContext('2d', { willReadFrequently: true });
        ctx.drawImage(img, 0, 0, N, N);
        const d = ctx.getImageData(0, 0, N, N).data;
        let hex = '';
        for (let i = 0; i < d.length; i += 4) {
          // A transparent pixel has no colour; including it would drag every
          // palette toward whatever the canvas was cleared to.
          if (d[i + 3] < 128) continue;
          hex += d[i].toString(16).padStart(2,'0') + d[i+1].toString(16).padStart(2,'0')
               + d[i+2].toString(16).padStart(2,'0');
        }
        resolve(hex);
      } catch (e) { reject(e); }
      finally { URL.revokeObjectURL(url); }
    };
    img.onerror = () => { URL.revokeObjectURL(url); reject(new Error('Не вдалося прочитати зображення')); };
    img.src = url;
  });
}

function contrastReport(pairs){
  return `<ul class="pairs">${pairs.map(p => `
    <li class="${p.passes ? 'ok' : 'bad'}">
      <i class="ti ti-${p.passes ? 'check' : 'alert-triangle'} i" aria-hidden="true"></i>
      <span>${esc(p.pair)}</span>
      <b>${p.ratio}:1</b><span class="hint">потрібно ${p.required}:1</span>
    </li>`).join('')}</ul>`;
}

// Settings are RENDERED FROM THE HUB'S OWN DECLARATIONS -- label, hint, default
// and secrecy all come from `/owner/settings`. Listing them again here would be
// a second list of settings, and the one that drifts is always the one the owner
// reads.
async function renderSettings(){
  const box = $('#aiSettings'); if (!box) return;
  let d;
  try { d = await api('/owner/settings'); } catch { return; }
  S.settings = d;
  box.innerHTML = d.known.map(k => {
    const v = d.values[k.key] || '';
    const isFlag = k.default === '0' || k.default === '1';
    if (isFlag) return `
      <label class="check"><input type="checkbox" data-set="${esc(k.key)}"
        ${(v === '1' || v === 'true' || v === 'on') ? 'checked' : ''}>
        <span>${esc(k.label)}</span></label>
      <p class="hint">${esc(k.hint)}</p>`;
    return `
      <label class="field">
        <span>${esc(k.label)}</span>
        <input type="${k.secret ? 'password' : 'text'}" data-set="${esc(k.key)}"
          placeholder="${esc(k.secret && v ? v : k.default)}"
          value="${esc(k.secret ? '' : v)}"
          autocomplete="${k.secret ? 'new-password' : 'off'}">
      </label>
      <p class="hint">${esc(k.hint)}</p>`;
  }).join('');

  box.querySelectorAll('[data-set]').forEach(el => {
    const save = async value => {
      el.disabled = true;
      try {
        await api('/owner/settings', { method:'POST',
          body: JSON.stringify({ key: el.dataset.set, value }) });
        toast('Збережено');
      } catch (e) {
        toast(String(e.message || e));
        // Put the control back: a field showing a value the hub refused is a
        // setting the owner believes is active and is not.
        await renderSettings(); return;
      }
      el.disabled = false;
    };
    if (el.type === 'checkbox') el.onchange = () => save(el.checked ? '1' : '0');
    // `change` and not `input`: saving on every keystroke would post a dozen
    // half-typed endpoints, each of which the hub correctly refuses.
    else el.onchange = () => save(el.value.trim());
  });
}

// The keys that exist, by name. The VALUE is never listed -- only the session
// id, which is enough to revoke a key and useless for authenticating with it.
async function renderKeys(){
  const el = $('#keyList'); if (!el) return;
  let d;
  try { d = await api('/owner/apikeys'); } catch { return; }
  el.innerHTML = d.keys.map(k => `
    <li>
      <span>${esc(k.label || 'ключ')}</span>
      ${k.current ? '<span class="hint">поточна сесія</span>'
                  : `<button class="btn narrow" data-revoke="${esc(k.session)}">Відкликати</button>`}
    </li>`).join('') || '<li class="hint">Ключів ще немає</li>';
  el.querySelectorAll('[data-revoke]').forEach(b => {
    b.onclick = async () => {
      // Irreversible and immediate, so it is confirmed. Anything already using
      // this key stops working the moment this returns.
      if (!confirm('Відкликати ключ? Застосунки, що ним користуються, втратять доступ.')) return;
      b.disabled = true;
      try { await api('/owner/apikeys/revoke', { method:'POST',
              body: JSON.stringify({ session: b.dataset.revoke }) });
            toast('Ключ відкликано'); await renderKeys(); }
      catch (e) { b.disabled = false; toast(String(e.message || e)); }
    };
  });
}

function bindSetup(){
  let csvText = null;
  renderSettings();

  const ask = async () => {
    const q = $('#askBox').value.trim(); if (!q) return;
    const go = $('#askGo'), out = $('#answer');
    go.disabled = true; out.hidden = false;
    out.innerHTML = `<p class="hint"><i class="ti ti-loader-2 i spin" aria-hidden="true"></i>Думає…</p>`;
    try {
      const d = await api('/owner/assist', { method:'POST', body: JSON.stringify({ question: q }) });
      // Where the data went is shown WITH EVERY ANSWER, not once in a settings
      // page. It is the owner's customers' information and the answer is the
      // moment they are thinking about it.
      out.innerHTML = `<p class="said">${esc(d.answer)}</p>
        <p class="hint"><i class="ti ti-${d.local ? 'home' : 'cloud'} i" aria-hidden="true"></i>${
          d.local ? 'Відповіла модель на цьому сервері' :
                    'Хмарна модель · дані клієнтів не надсилались'}</p>`;
    } catch (e) {
      out.innerHTML = `<p class="said bad">${esc(String(e.message || e))}</p>`;
    }
    go.disabled = false;
  };
  const go = $('#askGo'); if (go) go.onclick = ask;
  const box = $('#askBox'); if (box) box.onkeydown = e => { if (e.key === 'Enter') ask(); };


  const csv = $('#csvFile');
  if (csv) csv.onchange = async () => {
    const f = csv.files?.[0]; if (!f) return;
    $('#csvName').textContent = f.name;
    try {
      csvText = await f.text();
      // A DRY RUN first, always. The owner sees the counts and every rejected
      // row before anything touches the live menu.
      const d = await api('/owner/menu/import', { method:'POST',
        headers:{ 'content-type':'text/csv' }, body: csvText });
      $('#csvReport').hidden = false;
      $('#csvReport').innerHTML = `
        <p><b>${d.products}</b> страв у <b>${d.categories}</b> розділах</p>
        ${d.warnings.length ? `<details open><summary>Не імпортовано: ${d.warnings.length}</summary>
          <ul class="warn">${d.warnings.map(w => `<li>${esc(w)}</li>`).join('')}</ul></details>` : ''}
        ${d.notInFile.length ? `<details><summary>Є в меню, немає у файлі: ${d.notInFile.length}</summary>
          <ul class="warn">${d.notInFile.map(p => `<li>${esc(p.name)}</li>`).join('')}</ul></details>` : ''}`;
      $('#csvApplyRow').hidden = d.products === 0;
    } catch (e) { toast(String(e.message || e)); }
  };

  const apply = $('#csvApply');
  if (apply) apply.onclick = async () => {
    if (!csvText) return;
    try {
      const q = $('#csvRetire').checked ? '?apply=true&retire_missing=true' : '?apply=true';
      const d = await busy(apply, () => api('/owner/menu/import' + q, { method:'POST',
        headers:{ 'content-type':'text/csv' }, body: csvText }));
      toast(`Меню оновлено: ${d.products} страв`);
      S.products = []; await loadMenu(); await loadVenue();
    } catch (e) { toast(String(e.message || e)); }
  };

  renderKeys();
  const keyNew = $('#keyNew');
  if (keyNew) keyNew.onclick = async () => {
    try {
      const d = await busy(keyNew, () => api('/owner/apikeys', { method:'POST',
        body: JSON.stringify({ label: $('#keyLabel').value.trim() }) }));
      // Shown ONCE, in a field the owner can select and copy. Not a toast:
      // a toast disappears, and this is the only time this value exists.
      const box = $('#keyShown');
      box.hidden = false;
      box.innerHTML = `<p><b>${esc(d.label)}</b> — скопіюйте зараз, більше не покажемо:</p>
        <input class="ask" readonly value="${esc(d.key)}" id="keyValue">`;
      $('#keyValue').select();
      $('#keyLabel').value = '';
      await renderKeys();
    } catch (e) { toast(String(e.message || e)); }
  };

  const img = $('#imgFile');
  if (img) img.onchange = async () => {
    const f = img.files?.[0]; if (!f) return;
    $('#imgName').textContent = f.name;
    try {
      const pixels = await pixelsFromFile(f);
      const d = await api('/owner/branding/extract', { method:'POST',
        body: JSON.stringify({ pixels }) });
      const sw = $('#swatches');
      sw.hidden = false;
      if (!d.swatches.length) {
        sw.innerHTML = `<p class="hint">${esc(d.note || 'Кольорів не знайдено')}</p>`;
        return;
      }
      // Each candidate shows the colour AS IT WILL BE USED -- adjusted for
      // contrast if it had to be. Showing the raw colour and applying a
      // different one would be a bait and switch the owner only notices later.
      sw.innerHTML = d.swatches.map(s => `
        <button class="swatch" data-hex="${esc(s.hex)}" title="${esc(s.hex)}">
          <span class="chipc" style="background:${esc(s.theme.primary)}"></span>
          <span>${esc(s.theme.primary)}</span>
          <span class="hint">${s.sharePct}%${s.theme.primaryAdjustedPct ? ' · підсилено' : ''}</span>
        </button>`).join('');
      sw.querySelectorAll('.swatch').forEach(b => {
        b.onclick = async () => {
          sw.querySelectorAll('.swatch').forEach(x => x.disabled = true);
          try {
            const t = await api('/owner/branding', { method:'POST',
              body: JSON.stringify({ primary: b.dataset.hex }) });
            $('#contrast').hidden = false;
            $('#contrast').innerHTML = contrastReport(t.contrast);
            toast('Кольори оновлено');
            await loadVenue();
          } catch (e) { toast(String(e.message || e)); }
          sw.querySelectorAll('.swatch').forEach(x => x.disabled = false);
        };
      });
    } catch (e) { toast(String(e.message || e)); }
  };
}

function menuView(){
  // THREE STATES, and the bug this replaces is worth naming: the old code showed
  // a skeleton whenever the list was empty, so a venue that genuinely has no
  // dishes sat on a loading animation forever, and the skeleton itself had no
  // height so it collapsed to nothing anyway.
  if (S.menuPhase === 'loading' || S.menuPhase === 'idle') return skeletonMenu();
  if (S.menuPhase === 'error') return `
    <div class="panel"><div class="empty" role="alert">
      <i class="ti ti-alert-triangle i" aria-hidden="true"></i>
      <b>Меню не завантажилось</b>
      <span class="reason">${esc(S.menuError || '')}</span>
      <button class="btn" id="retryMenu" style="margin-top:12px">Спробувати ще раз</button>
    </div></div>`;
  if (!S.products.length) return `
    <div class="panel"><div class="empty">
      <i class="ti ti-tools-kitchen-2 i" aria-hidden="true"></i>
      <b>У меню ще немає страв</b>
      Завантажте CSV у розділі «Налаштування» — і меню з'явиться тут
      <button class="btn" id="toSetup" style="margin-top:12px">До налаштувань</button>
    </div></div>`;
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
