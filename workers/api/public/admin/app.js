import { shrinkImage } from '/lib/shrink.js';
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

// THE COUNT IS SHOWN UNTIL IT IS ZERO, and then the banner disappears entirely.
//
// The publish gate refuses NEW listings and deliberately does not sweep a
// working menu -- taking fifty-two dishes off sale the moment the field arrived
// would close a restaurant to fix its paperwork. The cost of that mercy is that
// undeclared dishes keep selling, so the number says so on every screen the
// owner opens, and the button goes straight to the work.
function readinessBanner(s){
  const n = s?.onSaleUndeclared || 0;
  if (!n) return '';
  return `<div class="ready" role="status">
    <i class="ti ti-alert-triangle i" aria-hidden="true"></i>
    <span><b>${n} ${plural(n, 'страва', 'страви', 'страв')} у продажу без заяви про алергени.</b>
      Клієнт з алергією читає порожнє поле як «безпечно».</span>
    <button class="btn" id="toAllergens">Заявити</button>
  </div>`;
}

// Ukrainian needs three forms, and "52 страва" is the tell that somebody wired
// an English pluraliser into a Slavic language.
function plural(n, one, few, many){
  const m10 = n % 10, m100 = n % 100;
  if (m10 === 1 && m100 !== 11) return one;
  if (m10 >= 2 && m10 <= 4 && (m100 < 12 || m100 > 14)) return few;
  return many;
}

function render(){
  if (!S.booted) return;
  const s = S.stats;
  $('#app').innerHTML = `
    <div class="stats">
      ${[['todayOrders','Замовлень сьогодні'],['pending','Чекають'],
         ['scheduled','На час'],['active','В роботі'],['todayRevenue','Виручка']].map(([k, label]) => `
        <div class="stat"><div class="k">${label}</div>
          <div class="v money" data-k="${k}">${s
            ? (k === 'todayRevenue' ? money(s[k]) : s[k])
            : `<span class="skel" style="display:inline-block;width:3rem;height:1.4rem;vertical-align:-.2em"></span>`}</div></div>`).join('')}
    </div>
    ${readinessBanner(s)}
    <div class="tabs" role="tablist" aria-label="Розділи">
      <button class="tab" role="tab" id="tab-orders" aria-controls="pane" data-t="orders" aria-selected="${S.tab==='orders'}" tabindex="${S.tab==='orders' ? 0 : -1}">Замовлення <span class="n" id="liveN">${liveOrders().length}</span></button>
      <button class="tab" role="tab" id="tab-menu"   aria-controls="pane" data-t="menu"   aria-selected="${S.tab==='menu'}"   tabindex="${S.tab==='menu' ? 0 : -1}">Меню</button>
      <button class="tab" role="tab" id="tab-stats" aria-controls="pane" data-t="stats" aria-selected="${S.tab==='stats'}" tabindex="${S.tab==='stats' ? 0 : -1}">Аналітика</button>
      <button class="tab" role="tab" id="tab-setup" aria-controls="pane" data-t="setup" aria-selected="${S.tab==='setup'}" tabindex="${S.tab==='setup' ? 0 : -1}">Налаштування</button>
    </div>
    <div id="pane" role="tabpanel" aria-labelledby="tab-${S.tab}"></div>`;
  const ta = $('#toAllergens');
  if (ta) ta.onclick = () => {
    S.tab = 'menu'; render();
    if (!S.products.length) loadMenu();
  };
  const tabs = [...document.querySelectorAll('.tab')];
  tabs.forEach((b, i) => {
    b.onclick = () => {
      S.tab = b.dataset.t; render();
      if (S.tab === 'menu' && !S.products.length) loadMenu();
      if (S.tab === 'stats') loadAnalytics();
    };
    // Keyboard: arrows move between tabs, as a tablist is expected to.
    b.onkeydown = e => {
      if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return;
      const n = tabs[(i + (e.key === 'ArrowRight' ? 1 : tabs.length - 1)) % tabs.length];
      n.focus(); n.click(); e.preventDefault();
    };
  });
  $('#pane').innerHTML = S.tab === 'orders' ? ordersView()
    : S.tab === 'menu' ? menuView()
    : S.tab === 'stats' ? statsView()
    : setupView();
  if (S.tab === 'orders') { bindFind(); bindOrders(); }
  else if (S.tab === 'menu') bindMenu();
  else if (S.tab === 'stats') bindStats();
  else bindSetup();
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
// ── finding an order ────────────────────────────────────────────────────────
//
// Fifty orders in an evening is a scroll. The search runs on WHAT IS ALREADY
// LOADED -- the pane already holds the day's orders -- so it answers instantly
// and keeps answering when the kitchen's connection does not.
//
// It matches the things somebody actually says on the phone: the short id read
// off a receipt, a name, the last digits of a number, a dish. Not the internal
// id, which nobody has.
function ordersMatching(){
  const all = S.view === 'history'
    ? S.orders.filter(o => !LIVE.includes(o.status))
    : liveOrders();
  const q = String(S.oq || '').trim().toLowerCase();
  if (!q) return all;
  const terms = q.split(/\s+/);
  return all.filter(o => {
    const hay = [
      shortId(o.id), o.contact?.name, o.contact?.phone,
      o.fulfilment?.address?.line, o.status,
      ...(o.items || []).map(i => i.name),
    ].filter(Boolean).join(' ').toLowerCase();
    return terms.every(t => hay.includes(t));
  });
}

// ONE ROW PER ORDER, and the money stays an integer all the way into the file.
// A spreadsheet that opens 2650 as 26.50 because somebody wrote a decimal point
// is how a day's takings get misread; the currency is its own column instead.
function ordersCsv(rows){
  const cur = S.venue?.currency || S.analytics?.currency || '';
  const cell = v => {
    const s = String(v ?? '');
    return /[",\n;]/.test(s) ? '"' + s.replace(/"/g, '""') + '"' : s;
  };
  const head = ['id', 'коли', 'статус', 'спосіб', 'клієнт', 'телефон', 'адреса',
                'страви', 'сума', 'валюта', 'знижка', 'промокод', 'кур\'єр'];
  const body = rows.map(o => [
    shortId(o.id),
    new Date(o.created_at_ms || 0).toISOString(),
    o.status,
    o.fulfilment?.kind || '',
    o.contact?.name || '',
    o.contact?.phone || '',
    o.fulfilment?.address?.line || '',
    (o.items || []).map(i => `${i.quantity}x ${i.name || i.product_id}`).join('; '),
    o.total ?? 0,
    cur,
    o.discount ?? 0,
    o.promo?.code || '',
    o.courier_id || '',
  ].map(cell).join(','));
  // A BOM, because the spreadsheet everyone actually opens this in reads a
  // bare UTF-8 file as Latin-1 and turns every Ukrainian name into mojibake.
  return '\ufeff' + [head.map(cell).join(','), ...body].join('\n');
}

function downloadCsv(name, text){
  const url = URL.createObjectURL(new Blob([text], { type:'text/csv;charset=utf-8' }));
  const a = document.createElement('a');
  a.href = url; a.download = name;
  document.body.appendChild(a); a.click(); a.remove();
  // Revoked on the next frame: revoking immediately races the download in
  // some browsers and produces an empty file.
  requestAnimationFrame(() => URL.revokeObjectURL(url));
}

function findBar(n){
  return `<div class="ofind">
    <label class="srch">
      <i class="ti ti-search" aria-hidden="true"></i>
      <input id="oq" type="search" inputmode="search" autocomplete="off"
             placeholder="Номер, ім'я, телефон, страва…" aria-label="Пошук замовлень"
             value="${esc(S.oq || '')}">
    </label>
    <div class="seg">
      <button class="btn ${S.view !== 'history' ? 'pri' : ''}" data-view="live">Живі</button>
      <button class="btn ${S.view === 'history' ? 'pri' : ''}" data-view="history">Історія</button>
    </div>
    <button class="btn" id="ocsv" ${n ? '' : 'disabled'}>
      <i class="ti ti-download i" aria-hidden="true"></i>CSV</button>
  </div>`;
}

function bindFind(){
  const q = $('#oq');
  if (q) q.oninput = () => {
    const pos = q.selectionStart;
    S.oq = q.value; render();
    const again = $('#oq');
    if (again) { again.focus(); try { again.setSelectionRange(pos, pos); } catch {} }
  };
  document.querySelectorAll('[data-view]').forEach(b => b.onclick = () => {
    S.view = b.dataset.view; render();
  });
  const c = $('#ocsv');
  if (c) c.onclick = () => {
    const rows = ordersMatching();
    const day = new Date().toISOString().slice(0, 10);
    downloadCsv(`dowiz-${S.view === 'history' ? 'history' : 'live'}-${day}.csv`, ordersCsv(rows));
  };
}

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
  const live = ordersMatching();
  const searching = Boolean(String(S.oq || '').trim());
  if (!live.length) return findBar(0) + `<div class="panel"><div class="empty">
      <i class="ti ti-${searching ? 'search-off' : 'inbox'} i" aria-hidden="true"></i>
      <b>${searching ? 'Нічого не знайшли' : S.view === 'history' ? 'Історія порожня' : 'Поки тихо'}</b>
      ${searching ? 'Спробуйте номер, ім\'я або страву'
        : S.view === 'history' ? 'Завершені замовлення з\'являться тут'
        : 'Нові замовлення з\'являться тут автоматично'}</div></div>`;
  let i = 0;
  return findBar(live.length)
    + `<div class="panel">${live.map(o => row(o, S.fresh.has(o.id) ? i++ : -1)).join('')}</div>`;
}

// Placeholders shaped like the rows they stand in for -- same height, same
// three bands -- so the queue does not jump when the real orders land.
// ── analytics ───────────────────────────────────────────────────────────────
// The chart is drawn as an inline SVG from the numbers the hub folded. No
// charting library: the same reasoning that took maplibre off the courier's
// first paint applies here, and a bar chart is a loop over rectangles.
//
// Money in the chart is LABELLED but never animated, and the axis label names a
// value the bars actually reach — a chart whose top gridline is a round number
// nothing touches invites the wrong reading.
function bars(series, valueOf, labelOf){
  const vals = series.map(valueOf);
  const peak = Math.max(1, ...vals);
  const w = 100 / Math.max(1, series.length);
  return `<svg class="chart" viewBox="0 0 100 44" preserveAspectRatio="none" role="img"
            aria-label="${esc(labelOf ? labelOf(peak) : String(peak))}">
    ${series.map((d, i) => {
      const h = (valueOf(d) / peak) * 38;
      return `<rect x="${(i * w + w * 0.15).toFixed(2)}" y="${(40 - h).toFixed(2)}"
                width="${(w * 0.7).toFixed(2)}" height="${Math.max(0.4, h).toFixed(2)}"
                rx="0.6"></rect>`;
    }).join('')}
    <line x1="0" y1="40" x2="100" y2="40" class="axis"></line>
  </svg>`;
}

function statsView(){
  if (S.statsPhase === 'loading' || S.statsPhase === undefined) return skeletonMenu();
  if (S.statsPhase === 'error') return `
    <div class="panel"><div class="empty" role="alert">
      <i class="ti ti-alert-triangle i" aria-hidden="true"></i>
      <b>Аналітика не завантажилась</b>
      <span class="reason">${esc(S.statsError || '')}</span>
      <button class="btn" id="retryStats" style="margin-top:12px">Спробувати ще раз</button>
    </div></div>`;
  const a = S.analytics;
  if (!a || !a.orders) return `
    <div class="panel"><div class="empty">
      <i class="ti ti-chart-bar i" aria-hidden="true"></i>
      <b>Ще немає даних</b>
      Числа з'являться після перших замовлень
    </div></div>`;

  const day = at => new Date(at).toLocaleDateString('uk', { day:'numeric', month:'short' });
  return `
    <div class="panel setup">
      <section class="card">
        <div class="row">
          <h2>За ${a.days} днів</h2>
          <span class="spacer"></span>
          <div class="seg">
            <button class="btn ${a.days === 7 ? 'pri' : ''}" data-days="7">7</button>
            <button class="btn ${a.days === 30 ? 'pri' : ''}" data-days="30">30</button>
          </div>
        </div>
        <div class="stats">
          <div class="stat"><div class="k">Замовлень</div><div class="v">${a.orders}</div></div>
          <div class="stat"><div class="k">Виручка</div><div class="v money">${money(a.revenue)}</div></div>
          <div class="stat"><div class="k">Середній чек</div><div class="v money">${money(a.averageOrder)}</div></div>
          <div class="stat"><div class="k">Відхилено</div><div class="v">${a.rejected}</div></div>
        </div>
      </section>

      <section class="card">
        <h2>Виручка по днях</h2>
        ${bars(a.byDay, d => d.revenue, p => `максимум ${p}`)}
        <div class="xaxis"><span>${esc(day(a.byDay[0].at))}</span>
          <span>${esc(day(a.byDay[a.byDay.length - 1].at))}</span></div>
      </section>

      <section class="card">
        <h2>Коли замовляють</h2>
        ${bars(a.byHour.map((n, h) => ({ n, h })), d => d.n, p => `максимум ${p}`)}
        <div class="xaxis"><span>00</span><span>12</span><span>23</span></div>
      </section>

      <section class="card">
        <h2>Що беруть</h2>
        <div class="elist">
          ${a.topProducts.map(p => `
            <div class="erow"><span>${esc(p.name)}<br>
              <small class="hint">${p.quantity} порцій</small></span>
              <span class="money">${money(p.revenue)}</span></div>`).join('')}
        </div>
      </section>

      <section class="card">
        <h2>Як забирають</h2>
        <div class="elist">
          <div class="erow"><span>Доставка</span><span>${a.delivery}</span></div>
          <div class="erow"><span>Самовивіз</span><span>${a.pickup}</span></div>
        </div>
      </section>
    </div>`;
}

async function loadAnalytics(days){
  S.statsPhase = 'loading'; if (S.tab === 'stats') render();
  try {
    S.analytics = await api(`/owner/analytics?days=${days || S.analytics?.days || 7}`);
    S.statsPhase = 'ready';
  } catch (e) { S.statsPhase = 'error'; S.statsError = String(e.message || e); }
  if (S.tab === 'stats') render();
}

function bindStats(){
  document.querySelectorAll('[data-days]').forEach(b =>
    b.onclick = () => loadAnalytics(parseInt(b.dataset.days, 10)));
  const r = $('#retryStats'); if (r) r.onclick = () => loadAnalytics();
}

// ── customers ───────────────────────────────────────────────────────────────
//
// The list never holds a phone number: the hub sends it masked and the reveal
// is a separate, reasoned request. Nothing here un-masks locally, because a
// client that could would make the audit log a formality.
function customerRow(c){
  return `<div class="erow" data-cu="${esc(c.key)}">
    <span><b>${esc(c.name)}</b> <small class="hint">${esc(c.phone)}</small>
      <br><small class="hint">${c.orders} ${plural(c.orders, 'замовлення', 'замовлення', 'замовлень')}
        · ${new Date(c.lastAt).toLocaleDateString('uk', { day:'numeric', month:'short' })}</small></span>
    <span class="row"><span class="money">${money(c.spent)}</span>
      <button class="icon-btn" data-cu-show aria-label="Показати контакти">
        <i class="ti ti-eye i" aria-hidden="true"></i></button></span>
  </div>`;
}

function renderCustomers(){
  const box = $('#cuList'); if (!box) return;
  const rows = S.customers || [];
  box.innerHTML = rows.length
    ? rows.map(customerRow).join('')
    : `<p class="hint">Ще нікого. Список складається з ваших замовлень.</p>`;
  box.querySelectorAll('[data-cu-show]').forEach(b => b.onclick = () => {
    const key = b.closest('[data-cu]').dataset.cu;
    // The reason is REQUIRED and goes into the log. A prompt is blunt, and
    // blunt is right: the point is that looking is a deliberate act.
    const reason = prompt('Навіщо потрібні контакти? (запишемо в журнал)');
    if (!reason || reason.trim().length < 3) return;
    revealCustomer(key, reason.trim());
  });
}

async function revealCustomer(key, reason){
  const box = $('#cuShown');
  box.hidden = false; box.textContent = 'Показуємо…';
  try {
    const d = await api(`/owner/customers/${encodeURIComponent(key)}/reveal`, { reason });
    box.innerHTML = `<b>${esc(d.name)}</b> · <a href="tel:${esc(d.phone)}">${esc(d.phone)}</a>
      <div class="elist" style="margin-top:8px">
        ${d.orders.slice(0, 8).map(o => `<div class="erow">
          <span>${new Date(o.at).toLocaleDateString('uk', { day:'numeric', month:'short' })}
            ${o.address ? `<br><small class="hint">${esc(o.address)}</small>` : ''}</span>
          <span class="money">${money(o.total)}</span></div>`).join('')}
      </div>`;
  } catch (e) { box.textContent = String(e.message || e); }
}

async function loadCustomers(){
  try {
    const d = await api(`/owner/customers${S.cSort ? '?sort=' + S.cSort : ''}`);
    S.customers = d.customers || [];
  } catch { S.customers = []; }
  renderCustomers();
}

function bindCustomers(){
  if (!$('#cuList')) return;
  loadCustomers();
  document.querySelectorAll('[data-csort]').forEach(b => b.onclick = () => {
    S.cSort = b.dataset.csort; render();
  });
  $('#cuCsv').onclick = () => {
    // THE EXPORT IS THE MASKED LIST. A file is the easiest thing in the world
    // to forward, and an export that un-masked would undo every other decision
    // on this screen in one click.
    const cell = v => { const x = String(v ?? ''); return /[",\n;]/.test(x) ? '"' + x.replace(/"/g,'""') + '"' : x; };
    const head = ['клієнт', 'телефон', 'замовлень', 'сума', 'останнє'];
    const body = (S.customers || []).map(c => [c.name, c.phone, c.orders, c.spent,
      new Date(c.lastAt).toISOString()].map(cell).join(','));
    downloadCsv(`dowiz-customers-${new Date().toISOString().slice(0,10)}.csv`,
                '\ufeff' + [head.join(','), ...body].join('\n'));
  };
  $('#cuLog').onclick = async () => {
    const box = $('#cuShown');
    box.hidden = false; box.textContent = 'Завантажуємо…';
    try {
      const d = await api('/owner/customers/reveals');
      box.innerHTML = d.reveals.length
        ? `<div class="elist">${d.reveals.slice(0, 20).map(r => `<div class="erow">
            <span>${esc(r.by)}<br><small class="hint">${esc(r.reason || '')}</small></span>
            <span class="hint">${new Date(r.at).toLocaleString('uk', {
              day:'numeric', month:'short', hour:'2-digit', minute:'2-digit' })}</span>
          </div>`).join('')}</div>`
        : '<p class="hint">Контактів ще ніхто не дивився.</p>';
    } catch (e) { box.textContent = String(e.message || e); }
  };
}

// ── activation ──────────────────────────────────────────────────────────────
//
// The same three facts the hub gates on, shown before the owner presses open
// rather than as a refusal after. The list is read from the hub, never worked
// out here: two copies of a rule are two rules, and the one on the screen is
// always the one that goes stale.
async function loadActivation(){
  const box = $('#actList'); if (!box) return;
  try {
    const a = await api('/owner/activation');
    S.activation = a;
    const done = k => !a.missing.some(m => m.key === k);
    const rows = [
      ['menu', 'Є що продати', `${a.facts.sellableDishes} страв у продажу`],
      ['notifications', 'Є кому почути', a.facts.telegramChats
        ? `Telegram: ${a.facts.telegramChats}` : a.facts.hasVenuePhone ? 'телефон закладу' : ''],
      ['fulfilment', 'Є як віддати', [a.facts.deliveryConfigured && 'доставка',
        a.facts.pickupEnabled && 'самовивіз'].filter(Boolean).join(' · ')],
    ];
    box.innerHTML = rows.map(([key, label, detail]) => {
      const ok = done(key);
      const why = a.missing.find(m => m.key === key)?.why || '';
      return `<div class="erow">
        <span><i class="ti ti-${ok ? 'circle-check' : 'circle-dashed'} i"
                 aria-hidden="true"></i> ${label}
          ${detail ? `<br><small class="hint">${esc(detail)}</small>` : ''}
          ${ok ? '' : `<br><small class="hint">${esc(why)}</small>`}</span>
        <span class="chip ${ok ? 'ok' : 'warn'}">${ok ? 'готово' : 'бракує'}</span>
      </div>`;
    }).join('');
    const ph = $('#vnPhone'), pk = $('#vnPickup');
    if (ph && !ph.value) ph.value = S.venue?.phone || '';
    if (pk) pk.checked = Boolean(a.facts.pickupEnabled);
  } catch (e) { box.innerHTML = `<p class="hint">${esc(String(e.message || e))}</p>`; }
}

function bindActivation(){
  if (!$('#vnSave')) return;
  loadActivation();
  $('#vnSave').onclick = async () => {
    const err = $('#vnErr'); err.hidden = true;
    try {
      await api('/owner/location',
        { phone: $('#vnPhone').value.trim(), pickup: $('#vnPickup').checked });
      toast('Збережено');
      loadActivation();
    } catch (e) { err.hidden = false; err.textContent = String(e.message || e); }
  };
}

// ── couriers ────────────────────────────────────────────────────────────────
//
// People and pending invites in ONE list. An owner asking who delivers for them
// counts the person they invited yesterday among the answer, and a separate
// panel for invites is a panel nobody opens.
function courierRow(c){
  const state = c.pending
    ? (c.expired ? { label: 'код прострочено', tone: '' } : { label: 'чекає на код', tone: 'wait' })
    : c.active
      ? (c.onShift ? { label: 'на зміні', tone: 'ok' } : { label: 'не на зміні', tone: '' })
      : { label: 'пішов', tone: '' };
  return `<div class="erow courier" data-id="${esc(c.id)}" data-pending="${c.pending ? 1 : 0}">
    <span><b>${esc(c.name || c.id)}</b>
      <span class="chip ${state.tone}">${state.label}</span>
      <br><small class="hint">${esc(c.id)}</small></span>
    <span class="row">
      ${c.pending
        ? `<button class="icon-btn" data-cv-drop aria-label="Скасувати запрошення">
             <i class="ti ti-x i" aria-hidden="true"></i></button>`
        : `<button class="icon-btn" data-cv-open aria-label="Показати ${esc(c.name || c.id)}">
             <i class="ti ti-chevron-right i" aria-hidden="true"></i></button>
           <button class="icon-btn" data-cv-active aria-label="${c.active ? 'Звільнити' : 'Повернути'}">
             <i class="ti ti-${c.active ? 'user-off' : 'user-check'} i" aria-hidden="true"></i></button>`}
    </span>
  </div>`;
}

function renderCouriers(){
  const box = $('#cvList'); if (!box) return;
  const people = (S.couriers || []).map(c => ({ ...c, pending: false }));
  const waiting = (S.invites || []).map(i => ({ ...i, pending: true }));
  const rows = [...people, ...waiting];
  box.innerHTML = rows.length
    ? rows.map(courierRow).join('')
    : `<p class="hint">Ще нікого. Перший рядок вище створює запрошення.</p>`;
  box.querySelectorAll('.courier').forEach(row => {
    const id = row.dataset.id;
    const c = rows.find(x => x.id === id);
    const drop = row.querySelector('[data-cv-drop]');
    if (drop) drop.onclick = async () => {
      await api(`/owner/couriers/${encodeURIComponent(id)}/uninvite`, {});
      loadCouriers();
    };
    const act = row.querySelector('[data-cv-active]');
    if (act) act.onclick = async () => {
      // Turning a courier off kills every session they hold, so it asks.
      if (c.active && !confirm(`${c.name || id} більше не зможе увійти. Продовжити?`)) return;
      await api(`/owner/couriers/${encodeURIComponent(id)}/active`, { active: !c.active });
      loadCouriers();
    };
    const open = row.querySelector('[data-cv-open]');
    if (open) open.onclick = () => showCourier(id);
  });
}

async function showCourier(id){
  const box = $('#cvShown');
  box.hidden = false; box.textContent = 'Завантажуємо…';
  try {
    const d = await api(`/owner/couriers/${encodeURIComponent(id)}`);
    // Work, cash and shifts. NO SCORE: dowiz does not rank the people who
    // deliver for it, and an average-minutes figure is a ranking in disguise.
    box.innerHTML = `<b>${esc(d.name)}</b>
      <div class="stats">
        <div class="stat"><div class="k">Доставок за 30 днів</div><div class="v">${d.delivered30d}</div></div>
        <div class="stat"><div class="k">Зараз у роботі</div><div class="v">${d.inFlight}</div></div>
        <div class="stat"><div class="k">Готівка на руках</div><div class="v money">${money(d.cashHeld)}</div></div>
      </div>`;
  } catch (e) { box.textContent = String(e.message || e); }
}

// ONE request, two consumers: the assign dropdown on the orders tab and the
// couriers pane in settings. There were two loaders for a while -- the second
// silently shadowed the first, which in a module is not a shadow but a fatal
// redeclaration, and the whole admin pane failed to parse.
async function loadCouriers(){
  try {
    const d = await api('/owner/couriers');
    S.couriers = d.couriers || []; S.invites = d.invites || [];
  } catch { S.couriers = []; S.invites = []; }
  renderCouriers();
}

function bindCouriers(){
  if (!$('#cvGo')) return;
  loadCouriers();
  $('#cvGo').onclick = async () => {
    const err = $('#cvErr'), shown = $('#cvShown');
    err.hidden = true; shown.hidden = true;
    try {
      const d = await api('/owner/couriers/invite',
        { phone: $('#cvPhone').value.trim(), name: $('#cvName').value.trim() });
      // ONCE. The hub stores it hashed and cannot show it again, so the screen
      // says so rather than letting the owner assume they can come back for it.
      shown.hidden = false;
      shown.innerHTML = `<b class="code">${esc(d.code)}</b>
        <p class="hint">Передайте цей код кур'єру. Більше ми його не покажемо.</p>`;
      $('#cvPhone').value = ''; $('#cvName').value = '';
      loadCouriers();
    } catch (e) { err.hidden = false; err.textContent = String(e.message || e); }
  };
}

// ── promo codes ─────────────────────────────────────────────────────────────
//
// The DATE inputs speak in whole local days, which is what the owner means, and
// the API speaks in half-open millisecond windows, which is what a comparison
// needs. "діє по 31 березня" therefore becomes midnight on 1 April: the code
// works all through the 31st and stops the instant the day does. Translating in
// the other direction -- midnight on the 31st -- is the off-by-one that shows
// up as a customer complaint at nine in the evening.
const DAY_MS = 86400000;

function dayToMs(v, endOfDay){
  if (!v) return null;
  const t = new Date(v + 'T00:00').getTime();
  return Number.isFinite(t) ? t + (endOfDay ? DAY_MS : 0) : null;
}

function msToDay(ms, endOfDay){
  if (ms === null || ms === undefined) return '';
  const d = new Date(ms - (endOfDay ? DAY_MS : 0));
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

const PM_STATUS = {
  active:    { label: 'діє',         tone: 'ok' },
  inactive:  { label: 'вимкнено',    tone: '' },
  scheduled: { label: 'ще не почав', tone: 'wait' },
  expired:   { label: 'скінчився',   tone: '' },
  exhausted: { label: 'вичерпано',   tone: '' },
};

function promoRow(p){
  const st = PM_STATUS[p.status] || { label: p.status, tone: '' };
  const cut = p.kind === 'percent'
    ? `−${p.value}%`
    : `−<span class="money">${money(p.value)}</span>`;
  const uses = p.maxUses ? `${p.used}/${p.maxUses}` : `${p.used}`;
  const when = [
    p.fromMs ? `з ${msToDay(p.fromMs, false)}` : '',
    p.untilMs ? `по ${msToDay(p.untilMs, true)}` : '',
    p.minOrder ? `від <span class="money">${money(p.minOrder)}</span>` : '',
  ].filter(Boolean).join(' · ');
  return `<div class="erow promo" data-code="${esc(p.code)}">
    <span><b>${esc(p.code)}</b> ${cut}
      <span class="chip ${st.tone}">${st.label}</span>
      ${when ? `<br><small class="hint">${when}</small>` : ''}</span>
    <span class="row">
      <small class="hint" title="використань">${uses}</small>
      <button class="icon-btn" data-pm-toggle aria-label="${p.active ? 'Вимкнути' : 'Увімкнути'}">
        <i class="ti ti-${p.active ? 'player-pause' : 'player-play'} i" aria-hidden="true"></i></button>
      <button class="icon-btn" data-pm-del aria-label="Видалити ${esc(p.code)}">
        <i class="ti ti-trash i" aria-hidden="true"></i></button>
    </span>
  </div>`;
}

function renderPromos(){
  const box = $('#pmList'); if (!box) return;
  const rows = S.promos || [];
  box.innerHTML = rows.length
    ? rows.map(promoRow).join('')
    : `<p class="hint">Жодного коду. Перший рядок вище створює його.</p>`;
  box.querySelectorAll('.promo').forEach(row => {
    const code = row.dataset.code;
    const p = rows.find(x => x.code === code);
    row.querySelector('[data-pm-toggle]').onclick = () => savePromo({ ...p, active: !p.active });
    // A delete is not undoable and the word becomes free again, so it asks.
    row.querySelector('[data-pm-del]').onclick = async () => {
      if (!confirm(`Видалити ${code}? Код перестане діяти.`)) return;
      await api(`/owner/promotions/${encodeURIComponent(code)}/delete`, {});
      loadPromos();
    };
  });
}

async function loadPromos(){
  try { S.promos = (await api('/owner/promotions')).promotions || []; }
  catch { S.promos = []; }
  renderPromos();
}

function pmFail(msg){
  const box = $('#pmErr'); if (!box) return;
  box.textContent = msg || '';
  box.hidden = !msg;
}

async function savePromo(p){
  pmFail('');
  try {
    await api('/owner/promotions', {
      code: p.code, kind: p.kind, value: p.value,
      minOrder: p.minOrder || 0, fromMs: p.fromMs ?? null,
      untilMs: p.untilMs ?? null, maxUses: p.maxUses ?? null, active: p.active !== false,
    });
    loadPromos();
    return true;
  } catch (e) { pmFail(String(e.message || e)); return false; }
}

function bindPromos(){
  if (!$('#pmSave')) return;
  loadPromos();
  $('#pmSave').onclick = async () => {
    const ok = await savePromo({
      code: $('#pmCode').value,
      kind: $('#pmKind').value,
      value: parseInt($('#pmValue').value, 10) || 0,
      minOrder: parseInt($('#pmMin').value, 10) || 0,
      fromMs: dayToMs($('#pmFrom').value, false),
      untilMs: dayToMs($('#pmUntil').value, true),
      maxUses: parseInt($('#pmMax').value, 10) || null,
      active: true,
    });
    if (ok) ['#pmCode', '#pmValue', '#pmMin', '#pmFrom', '#pmUntil', '#pmMax']
      .forEach(id => { $(id).value = ''; });
  };
}

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
  // The customer's own words, on the row rather than behind a tap: a note that
  // takes a click to find is a note nobody reads, and this is the only channel
  // they have after the order is over.
  const said = o.feedback?.text
    ? `<div class="said"><i class="ti ti-message-2 i" aria-hidden="true"></i>${esc(o.feedback.text)}</div>`
    : '';
  const f = o.fulfilment || {}, c = o.contact || {};
  const st = esc(o.status);
  // A scheduled order looks exactly like a live one in a queue, which is how a
  // kitchen starts cooking something due in three hours. The time is the whole
  // difference, so it is on the row rather than behind a tap.
  const due = o.scheduled_for_ms
    ? new Date(o.scheduled_for_ms).toLocaleString('uk', { day:'numeric', month:'short',
        hour:'2-digit', minute:'2-digit' })
    : null;
  return `<article class="order ${o.status === 'PENDING' ? 'attn' : ''} ${newIdx >= 0 ? 'is-new' : ''}" ${newIdx >= 0 ? `style="--i:${newIdx}"` : ''}>
    <div class="o-h">
      <span class="oid">#${esc(String(o.id).slice(0,8))}</span>
      <span class="chip ${st}"><i aria-hidden="true"></i>${esc(STATUS_LABEL[o.status] || o.status)}</span>
      ${due ? `<span class="due" title="Замовлення на визначений час">
        <i class="ti ti-clock-hour-4 i" aria-hidden="true"></i>${esc(due)}</span>` : ''}
      <span class="amt money">${money(o.total)}</span>
    </div>
    <p class="lines">${items || '—'}</p>
    <div class="who">
      ${c.phone ? `<a class="tel" href="tel:${esc(c.phone)}"><i class="ti ti-phone i" aria-hidden="true"></i>${esc(c.phone)}</a>` : ''}
      ${c.name ? `<span><i class="ti ti-user i" aria-hidden="true"></i>${esc(c.name)}</span>` : ''}
      ${f.address?.line ? `<span><i class="ti ti-map-pin i" aria-hidden="true"></i>${esc(f.address.line)}</span>` : ''}
      ${f.address?.note ? `<span class="muted"><i class="ti ti-note i" aria-hidden="true"></i>${esc(f.address.note)}</span>` : ''}
      <span class="muted"><i class="ti ${o.payment === 'cash' ? 'ti-cash' : 'ti-credit-card'} i" aria-hidden="true"></i>${o.payment === 'cash' ? 'готівка' : esc(o.payment || '')}</span>
      ${f.note ? `<span class="muted"><i class="ti ti-note i" aria-hidden="true"></i>${esc(f.note)}</span>` : ''}
      ${o.promo?.code ? `<span class="muted"><i class="ti ti-ticket i" aria-hidden="true"></i>${esc(o.promo.code)} −<span class="money">${money(o.discount || 0)}</span></span>` : ''}
    </div>
    ${said}
    ${o.proof?.url ? `<a class="proof" href="${esc(o.proof.url)}" target="_blank" rel="noopener">
      <img src="${esc(o.proof.url)}" alt="Фото біля дверей" loading="lazy" decoding="async"
           width="56" height="56">
      <span class="hint">фото біля дверей</span></a>` : ''}
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
// (The couriers loader lives with the couriers pane above: it fills both the
// assign dropdown and that pane, from one request.)

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
      <h2>Заклад</h2>
      <p class="hint">Три речі, без яких замовлення не має сенсу: є що продати,
         є кому почути замовлення, є як його віддати. Поки бракує хоч однієї —
         заклад не відчиняється.</p>
      <div id="actList" class="elist"></div>
      <div class="row">
        <input id="vnPhone" class="ask" type="tel" inputmode="tel" placeholder="Телефон закладу"
               autocomplete="off" aria-label="Телефон закладу">
        <label class="check"><input type="checkbox" id="vnPickup">
          <span>Можна забрати самому</span></label>
        <button class="btn" id="vnSave">
          <i class="ti ti-check i" aria-hidden="true"></i>Зберегти</button>
      </div>
      <div id="vnErr" class="report" role="alert" hidden></div>
    </section>

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
      <h2>Робочі години</h2>
      <p class="hint">Заклад відчинятиметься й зачинятиметься сам. Вручну можна
         закрити раніше — але не залишитись відчиненим поза графіком.
         Порожній рядок = вихідний.</p>
      <div id="hoursBox" class="hours"></div>
      <div class="row">
        <button class="btn pri" id="hoursSave"><i class="ti ti-check i" aria-hidden="true"></i>Зберегти</button>
        <button class="btn" id="hoursOff">Без графіка</button>
      </div>
    </section>

    <section class="card">
      <h2>Клієнти</h2>
      <p class="hint">Це не база клієнтів — це те, що видно з ваших замовлень.
         Імена й номери приховані, поки ви не попросите конкретний. Кожне
         розкриття записується в журнал, який неможливо стерти.</p>
      <div class="row">
        <span class="hint">Сортувати:</span>
        <div class="seg">
          <button class="btn ${S.cSort === 'spent' ? 'pri' : ''}" data-csort="spent">За сумою</button>
          <button class="btn ${S.cSort === 'orders' ? 'pri' : ''}" data-csort="orders">За кількістю</button>
          <button class="btn ${!S.cSort ? 'pri' : ''}" data-csort="">Нещодавні</button>
        </div>
      </div>
      <div id="cuShown" class="report" hidden></div>
      <div id="cuList" class="elist"></div>
      <div class="row">
        <button class="btn" id="cuCsv">
          <i class="ti ti-download i" aria-hidden="true"></i>CSV (приховано)</button>
        <button class="btn" id="cuLog">
          <i class="ti ti-history i" aria-hidden="true"></i>Журнал переглядів</button>
      </div>
    </section>

    <section class="card">
      <h2>Кур'єри</h2>
      <p class="hint">Запрошення — це код на 16 знаків, який діє тиждень і
         спрацьовує <b>один раз</b>. Кур'єр сам придумає пароль: ви його не
         побачите. Код показуємо теж один раз.</p>
      <div class="row">
        <input id="cvPhone" class="ask" type="tel" inputmode="tel"
               placeholder="+355…" autocomplete="off" aria-label="Телефон кур'єра">
        <input id="cvName" class="ask" type="text" placeholder="Ім'я"
               autocomplete="off" aria-label="Ім'я кур'єра">
        <button class="btn" id="cvGo">
          <i class="ti ti-user-plus i" aria-hidden="true"></i>Запросити</button>
      </div>
      <div id="cvShown" class="report" hidden></div>
      <div id="cvErr" class="report" role="alert" hidden></div>
      <div id="cvList" class="elist"></div>
    </section>

    <section class="card">
      <h2>Промокоди</h2>
      <p class="hint">Знижка йде з їжі, не з доставки — кур'єру платять однаково.
         Відсоток округлюємо вниз. Статус рахується сам: код не треба
         вимикати вручну, коли скінчився термін.</p>
      <div class="promo-form">
        <input id="pmCode" class="ask" type="text" placeholder="КОД" autocomplete="off"
               maxlength="16" aria-label="Код">
        <select id="pmKind" class="ask" aria-label="Тип знижки">
          <option value="percent">відсоток</option>
          <option value="fixed">сума</option>
        </select>
        <input id="pmValue" class="ask" type="number" min="1" placeholder="10"
               inputmode="numeric" aria-label="Розмір знижки">
        <input id="pmMin" class="ask" type="number" min="0" placeholder="від суми"
               inputmode="numeric" aria-label="Мінімальне замовлення">
        <input id="pmFrom" class="ask" type="date" aria-label="Діє з">
        <input id="pmUntil" class="ask" type="date" aria-label="Діє по">
        <input id="pmMax" class="ask" type="number" min="1" placeholder="разів"
               inputmode="numeric" aria-label="Скільки разів можна використати">
        <button class="btn pri" id="pmSave">
          <i class="ti ti-plus i" aria-hidden="true"></i>Додати</button>
      </div>
      <div id="pmErr" class="report" role="alert" hidden></div>
      <div id="pmList" class="elist"></div>
    </section>

    <section class="card">
      <h2>Пости</h2>
      <p class="hint">dowiz помічає справжні зміни у вашому меню й пропонує короткий
         пост. <b>Нічого не публікується, поки ви не погодите.</b> Вигадані знижки
         й «встигніть сьогодні» не проходять — ні від моделі, ні від вас.</p>
      <div class="row">
        <button class="btn" id="postDraft">
          <i class="ti ti-sparkles i" aria-hidden="true"></i>Що можна написати?</button>
        <span class="hint" id="postChannel"></span>
      </div>
      <div id="postList" class="posts"></div>
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

// Re-encode a photograph in the browser before it is uploaded.
//
// THREE THINGS HAPPEN HERE, and the second is the one worth stating plainly:
//
//   1. The image is capped at 1600px on its long edge, which turns a 6 MB
//      camera original into something around 300 KB. The venue's own phone does
//      the work rather than their VPS.
//   2. EXIF IS STRIPPED, because a canvas re-encode carries no metadata. A
//      photo taken in the kitchen with a phone carries GPS coordinates, the
//      device model and a timestamp; publishing that on a public menu page
//      would hand anyone the restaurant's exact position and their staff's
//      phone. Nobody asks for this and it happens by default everywhere else.
//   3. It becomes a JPEG regardless of what it started as, so one format
//      reaches the hub and the hub's sniffer has one less thing to be right
//      about.
//
// A failure here REJECTS rather than falling back to the original bytes: the
// fallback would be the unresized, EXIF-carrying file, which is exactly what
// this exists to prevent.

// The drafts, with the two decisions that matter on each one.
//
// The TEXT IS EDITABLE before approval. An assistant that cannot be overruled is
// one that gets switched off, and the owner knows how their own restaurant
// speaks better than a model does.
async function renderPosts(){
  const box = $('#postList'); if (!box) return;
  let d;
  try { d = await api('/owner/posts'); } catch { return; }
  const ch = $('#postChannel');
  if (ch) ch.textContent = d.enabled
    ? (d.channel ? 'Канал: ' + d.channel : 'Канал не вказано — додайте його вище')
    : 'Вимкнено — увімкніть «Пости» у налаштуваннях вище';

  if (!d.posts.length) {
    box.innerHTML = `<p class="hint">Чернеток ще немає.</p>`;
    return;
  }
  box.innerHTML = d.posts.map(p => `
    <article class="post ${esc(p.state)}">
      <p class="about">${esc(p.about)}</p>
      ${p.state === 'draft'
        ? `<textarea class="ptext" data-text="${esc(p.id)}" rows="3">${esc(p.text)}</textarea>
           <div class="row">
             <button class="btn pri" data-approve="${esc(p.id)}">
               <i class="ti ti-send i" aria-hidden="true"></i>Опублікувати</button>
             <button class="btn" data-reject="${esc(p.id)}">Не треба</button>
           </div>`
        : `<p class="said">${esc(p.text)}</p>
           <p class="hint">${p.state === 'published' ? 'Опубліковано'
              : p.state === 'rejected' ? 'Відхилено'
              : 'Не вдалося: ' + esc(p.error)}</p>`}
    </article>`).join('');

  box.querySelectorAll('[data-approve]').forEach(b => b.onclick = async () => {
    const id = b.dataset.approve;
    const text = box.querySelector(`[data-text="${CSS.escape(id)}"]`)?.value?.trim();
    try {
      await busy(b, () => api(`/owner/posts/${encodeURIComponent(id)}/approve`,
        { method:'POST', body: JSON.stringify({ text }) }));
      toast('Опубліковано');
    } catch (e) { toast(String(e.message || e)); }
    await renderPosts();
  });
  box.querySelectorAll('[data-reject]').forEach(b => b.onclick = async () => {
    try { await busy(b, () => api(`/owner/posts/${encodeURIComponent(b.dataset.reject)}/reject`,
            { method:'POST' })); }
    catch (e) { toast(String(e.message || e)); }
    await renderPosts();
  });
}

const DAY_NAMES = ['Понеділок','Вівторок','Середа','Четвер','П\u2019ятниця','Субота','Неділя'];
const toHM = m => `${String(Math.floor(m/60)).padStart(2,'0')}:${String(m%60).padStart(2,'0')}`;
const toMin = v => { const [h,m] = String(v||'').split(':').map(Number);
                     return Number.isFinite(h) && Number.isFinite(m) ? h*60+m : null; };

// One row per day, with the venue's existing windows filled in. Only the FIRST
// window per day is editable here: a split day (lunch, then dinner) is real but
// rare, and a grid that can express it costs every owner the complexity. The
// API takes any number, so a second window set elsewhere survives a save
// untouched — which is why the row keeps the rest of the day's windows.
function renderHours(){
  const box = $('#hoursBox'); if (!box) return;
  const hours = Array.isArray(S.venue?.hours) ? S.venue.hours : [];
  box.innerHTML = DAY_NAMES.map((name, i) => {
    const w = (hours[i] || [])[0];
    return `<div class="hrow">
      <span>${name}</span>
      <input type="time" data-h-open="${i}" value="${w ? toHM(w.open) : ''}">
      <span class="hint">—</span>
      <input type="time" data-h-close="${i}" value="${w ? toHM(w.close) : ''}">
    </div>`;
  }).join('');
}

function bindSetup(){
  bindActivation();
  bindCustomers();
  bindCouriers();
  bindPromos();
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
  renderPosts();
  renderHours();
  const hs = $('#hoursSave');
  if (hs) hs.onclick = async () => {
    const existing = Array.isArray(S.venue?.hours) ? S.venue.hours : [];
    const hours = DAY_NAMES.map((_, i) => {
      const o = toMin($(`[data-h-open="${i}"]`)?.value);
      const c = toMin($(`[data-h-close="${i}"]`)?.value);
      // Windows beyond the first are preserved rather than dropped: this editor
      // shows one and must not silently delete a split day set elsewhere.
      const rest = (existing[i] || []).slice(1);
      // A day with one field filled is a mistake, not a window; sending it
      // would be refused by the hub anyway, and saying so here is quicker.
      if (o == null || c == null || o === c) return rest;
      return [{ open: o, close: c }, ...rest];
    });
    try {
      await busy(hs, () => api('/owner/location', { method:'POST',
        body: JSON.stringify({ location_id: store.loc, hours }) }));
      toast('Графік збережено'); await loadVenue(); renderHours();
    } catch (e) { toast(String(e.message || e)); }
  };
  const ho = $('#hoursOff');
  if (ho) ho.onclick = async () => {
    if (!confirm('Прибрати графік? Заклад керуватиметься лише кнопкою «відчинено».')) return;
    try {
      await busy(ho, () => api('/owner/location', { method:'POST',
        body: JSON.stringify({ location_id: store.loc, hours: [[],[],[],[],[],[],[]] }) }));
      toast('Графік прибрано'); await loadVenue(); renderHours();
    } catch (e) { toast(String(e.message || e)); }
  };
  const pd = $('#postDraft');
  if (pd) pd.onclick = async () => {
    try {
      const d = await busy(pd, () => api('/owner/posts/draft', { method:'POST' }));
      // "Nothing new to say" is an ANSWER, not a failure. Saying so beats an
      // empty list that looks like something broke.
      toast(d.drafted ? `Чернеток: ${d.drafted}` : 'Поки нема про що писати');
    } catch (e) { toast(String(e.message || e)); }
    await renderPosts();
  };
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

// ── allergens ───────────────────────────────────────────────────────────────
//
// THE THREE STATES ARE NOT TWO. A dish with no list is not a dish with an empty
// list: the first means nobody has said, the second means somebody said "none
// of the fourteen". Rendering both as no warning is how a customer with an
// allergy reads an unfilled field as a safety claim, so the undeclared state
// gets the loudest treatment on this screen.
const ALLERGENS = [
  ['gluten', 'глютен'], ['crustaceans', 'ракоподібні'], ['eggs', 'яйця'],
  ['fish', 'риба'], ['peanuts', 'арахіс'], ['soy', 'соя'],
  ['milk', 'молоко'], ['nuts', 'горіхи'], ['celery', 'селера'],
  ['mustard', 'гірчиця'], ['sesame', 'кунжут'], ['sulphites', 'сульфіти'],
  ['lupin', 'люпин'], ['molluscs', 'молюски'],
];
const allergenName = c => (ALLERGENS.find(a => a[0] === c) || [c, c])[1];

function allergenChip(p){
  if (!Array.isArray(p.allergens)) return `<span class="chip warn">не заявлено</span>`;
  if (!p.allergens.length) return `<span class="chip ok">без алергенів</span>`;
  return `<span class="chip">${p.allergens.map(c => esc(allergenName(c))).join(', ')}</span>`;
}

function allergenPanel(p){
  const has = Array.isArray(p.allergens) ? p.allergens : [];
  return `<div class="algn" data-algn="${esc(p.id)}" hidden>
    <p class="hint">Позначте те, що є у страві. Якщо немає нічого з переліку —
       натисніть «Нічого з переліку»: порожнє поле не є відповіддю.</p>
    <div class="algn-grid">
      ${ALLERGENS.map(([code, name]) => `
        <label class="algn-one"><input type="checkbox" value="${code}"
          ${has.includes(code) ? 'checked' : ''}><span>${esc(name)}</span></label>`).join('')}
    </div>
    <div class="row">
      <button class="btn pri" data-algn-save="${esc(p.id)}">Зберегти</button>
      <button class="btn" data-algn-none="${esc(p.id)}">Нічого з переліку</button>
    </div>
  </div>`;
}

async function saveAllergens(id, list){
  try {
    await api(`/owner/products/${encodeURIComponent(id)}`, { method:'POST',
      body: JSON.stringify({ location_id: store.loc, allergens: list }) });
    const p = S.products.find(x => x.id === id); if (p) p.allergens = list;
    toast('Заявлено');
    render();
  } catch (e) { toast(String(e.message || e)); }
}

function bindAllergens(){
  document.querySelectorAll('[data-algn-open]').forEach(b => b.onclick = () => {
    const box = document.querySelector(`[data-algn="${CSS.escape(b.dataset.algnOpen)}"]`);
    if (box) box.hidden = !box.hidden;
  });
  document.querySelectorAll('[data-algn-save]').forEach(b => b.onclick = () => {
    const id = b.dataset.algnSave;
    const box = document.querySelector(`[data-algn="${CSS.escape(id)}"]`);
    saveAllergens(id, [...box.querySelectorAll('input:checked')].map(i => i.value));
  });
  document.querySelectorAll('[data-algn-none]').forEach(b => b.onclick = () => {
    // A deliberate answer, not an empty form. It is the same call with an empty
    // list, and it is a separate button so nobody submits it by accident.
    saveAllergens(b.dataset.algnNone, []);
  });
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
      <label class="pic" title="Фото страви">
        ${p.imageUrl
          ? `<img src="${esc(p.imageUrl)}" alt="" loading="lazy" decoding="async" width="44" height="44">`
          : `<i class="ti ti-camera-plus i" aria-hidden="true"></i>`}
        <input type="file" accept="image/*" data-pic="${esc(p.id)}">
        <span class="sr">Фото: ${esc(p.name)}</span>
      </label>
      <span class="n"><b>${esc(p.name)}</b><small>${p.available ? 'у продажу' : esc(p.unavailableNote || 'зупинено')}</small></span>
      <input type="number" min="0" step="1" value="${p.price}" data-price="${esc(p.id)}" aria-label="Ціна, ${esc(p.name)}">
      <input type="number" min="3" max="120" step="1" class="size" placeholder="см"
             value="${p.sizeCm ?? ''}" data-size="${esc(p.id)}"
             title="Ширина страви в сантиметрах — вмикає перегляд на столі"
             aria-label="Розмір у см, ${esc(p.name)}">
      <label class="sw" title="У продажу"><input type="checkbox" data-av="${esc(p.id)}" aria-label="У продажу, ${esc(p.name)}" ${p.available ? 'checked' : ''}><span></span></label>
      <button class="algn-btn" data-algn-open="${esc(p.id)}"
              aria-label="Алергени: ${esc(p.name)}">${allergenChip(p)}</button>
    </div>`);
    out.push(allergenPanel(p));
  }
  if (cat !== null) out.push('</div>');
  return out.join('');
}
function bindMenu(){
  bindAllergens();
  document.querySelectorAll('[data-pic]').forEach(el => el.onchange = async () => {
    const file = el.files?.[0]; if (!file) return;
    const id = el.dataset.pic, cell = el.closest('.pic');
    cell.classList.add('busy');
    try {
      const blob = await shrinkImage(file);
      // The re-encoded BLOB, not the original file. Sending the original would
      // undo both the resize and the EXIF strip.
      const d = await api(`/owner/products/${encodeURIComponent(id)}/image`, {
        method:'POST', headers:{ 'content-type':'image/jpeg' }, body: blob });
      const p = S.products.find(x => x.id === id);
      if (p) p.imageUrl = d.imageUrl;
      toast('Фото збережено');
      render();
    } catch (e) {
      cell.classList.remove('busy');
      toast(String(e.message || e));
    }
  });

  document.querySelectorAll('[data-size]').forEach(el => el.onchange = async () => {
    const id = el.dataset.size, raw = el.value.trim();
    if (!raw) return;                       // clearing is not "set it to zero"
    const size_cm = parseInt(raw, 10);
    if (!Number.isFinite(size_cm)) { toast('Некоректний розмір'); return; }
    try {
      await api(`/owner/products/${encodeURIComponent(id)}`, { method:'POST',
        body: JSON.stringify({ location_id: store.loc, size_cm }) });
      const p = S.products.find(x => x.id === id); if (p) p.sizeCm = size_cm;
      toast('Розмір збережено');
    } catch (e) {
      // Put the old value back: a field showing a size the server refused is a
      // dish the owner believes is measured and is not.
      const p = S.products.find(x => x.id === id);
      el.value = p?.sizeCm ?? '';
      toast(String(e.message || e));
    }
  });

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
      const v = { todayOrders:s.todayOrders, pending:s.pending, scheduled:s.scheduled,
                  active:s.active, todayRevenue:money(s.todayRevenue) };
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
