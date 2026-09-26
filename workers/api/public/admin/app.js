// The owner console -- the shell. Sign in, the bottom bar, the venue's state
// in the header, the poll that keeps the queue honest, and the words in the
// owner's own language. Each tab is its own module and renders into #app.
//
// A PHONE APP IN THE BROWSER: five tabs under the thumb (orders, menu, stock,
// couriers, more), one screen at a time, every action a sheet. The console
// is a view over server state and decides nothing about an order's life --
// an action names an intent, the hub's FSM answers.

import { $, $$, esc, icon, t, lang, LANGS, setLang, retranslate, store, S, api, post, withLoc, logout, whenLoggedOut,
         toast, sheet, closeSheet, bindSheetChrome, hydrate, setCurrency, displayCurrency, CURRENCIES, baseCurrency, POLL_MS, POLL_IDLE_MS, confirm, switchEl } from '/admin/core.js';
import { ui, btn, field, chips, choice, press, loading } from '/admin/parts.js';
import { safeGet, safeSet } from '/store/storage.js';

/// The five tabs, their icons, their words, their modules.
const TABS = [
  ['orders',   'scroll',          'tabOrders',   () => import('/admin/orders.js')],
  ['menu',     'bowl-chopsticks', 'tabMenu',     () => import('/admin/menu.js')],
  ['stock',    'bento',           'tabStock',    () => import('/admin/stock.js')],
  ['couriers', 'bike',            'tabCouriers', () => import('/admin/couriers.js')],
  ['more',     'lantern',         'tabMore',     () => import('/admin/more.js')],
];
const LIVE = ['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY'];
export const liveOrders = () => S.orders.filter(o => LIVE.includes(o.status));
/// The venue's state, as the header chip and the sheet say it.
const STATES = ['open', 'busy', 'closed'];
/// A new order rings: a short tone, twice, where the page may play sound.
const RING_HZ = 880;
const RING_MS = 160;

const modules = new Map();
async function module(id){
  if (!modules.has(id)) modules.set(id, await TABS.find(x => x[0] === id)[3]());
  return modules.get(id);
}

// ── login ───────────────────────────────────────────────────────────────────
function renderLogin(err){
  $('#top').hidden = true; $('#nav').hidden = true;
  $('#app').innerHTML = `<div class="login">
    <div class="login-mark" aria-hidden="true"><span>d</span></div>
    <h1>dowiz</h1><p data-t="console"></p><p class="login-line" data-t="loginLine"></p>
    ${field({ id: 'e', key: 'email', type: 'email', autocomplete: 'username', inputmode: 'email', tour: 'login.email' })}
    ${field({ id: 'p', key: 'password', type: 'password', autocomplete: 'current-password', tour: 'login.password' })}
    ${err ? ui.alert({ label: err }) : ''}
    ${btn({ id: 'go', variant: 'primary', size: 'lg', block: true, icon: 'login', key: 'signIn', tour: 'login.go' })}
    ${chips({ values: LANGS.map(l => ({ value: l, label: l.toUpperCase() })), value: lang, attr: 'l', labelKey: 'language', tour: 'login.lang' }).replace('class="chips"', 'class="chips langs"')}
  </div>`;
  retranslate($('#app'));
  const submit = async () => {
    const b = $('#go'); ui.setBusy(b, t('signingIn'));
    try {
      const r = await fetch('/api/auth/login', { method: 'POST', headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ email: $('#e').value.trim(), password: $('#p').value }) });
      const d = await r.json();
      if (!r.ok) throw new Error(d.error || d.message || 'HTTP ' + r.status);
      store.t = d.access_token; store.r = d.refresh_token; store.loc = d.user.locationId;
      boot();
    } catch (e) { renderLogin(String(e.message || e)); }
  };
  $('#go').onclick = submit;
  $('#p').onkeydown = e => { if (e.key === 'Enter') submit(); };
  for (const b of $$('[data-l]')) b.onclick = () => { setLang(b.dataset.l); renderLogin(err); };
  $('#e').focus();
}
whenLoggedOut(renderLogin);
$('#logout').onclick = async () => { try { await post('/auth/logout'); } catch {} logout(); };

// ── the bottom bar ──────────────────────────────────────────────────────────
function mountNav(){
  const nav = $('#nav');
  nav.innerHTML = TABS.map(([id, ic, key]) => `<button type="button" class="tab" data-tab="${id}" data-tour="nav.${id}" aria-current="${id === S.tab ? 'page' : 'false'}">
    <span class="tab-ic">${icon(ic)}${id === 'orders' ? `<span class="tab-n" id="navLiveN" hidden>0</span>` : ''}</span><span data-t="${key}"></span></button>`).join('');
  nav.onclick = e => { const b = e.target.closest('[data-tab]'); if (b) show(b.dataset.tab); };
  retranslate(nav);
}
function light(id){ for (const b of $$('#nav [data-tab]')) b.setAttribute('aria-current', b.dataset.tab === id ? 'page' : 'false'); }
export async function show(id, arg){
  S.tab = id; light(id); closeSheet();
  const m = await module(id);
  $('#app').innerHTML = `<div class="screen" id="screen"></div>`;
  await m.render($('#screen'), arg);
  retranslate($('#app')); hydrate($('#app'));
}
/// Re-render the open tab in place after data moved (the poll, an action).
export async function rerender(){
  const m = modules.get(S.tab); const host = $('#screen');
  if (m && host && !document.body.classList.contains('sheet-open')) { await m.render(host); retranslate(host); hydrate(host); }
  paintLive();
}
function paintLive(){
  const n = liveOrders().filter(o => o.status === 'PENDING').length;
  const b = $('#navLiveN'); if (!b) return;
  const grew = Number(b.textContent) < n;
  b.textContent = n; b.hidden = n === 0;
  if (grew) { b.classList.remove('pop'); void b.offsetWidth; b.classList.add('pop'); }
  paintStale();
}
// A SCREEN THAT IS NOT NOW MUST SAY SO. The copy keeps drawing through an
// outage, which is the point of it -- and a queue that looks live while the
// venue has been unreachable for twenty minutes is worse than no queue at
// all, because a cook trusts it.
function paintStale(){
  const top = $('#top'); if (!top) return;
  let tag = $('#staleTag');
  if (!S.stale) { tag?.remove(); return; }
  if (!tag) {
    tag = document.createElement('span');
    tag.id = 'staleTag';
    tag.className = 'vstate closed';
    top.appendChild(tag);
  }
  tag.textContent = t('offlineCopy');
}

// ── the header: the venue's state ───────────────────────────────────────────
function paintVenue(){
  const v = S.venue; if (!v) return;
  const st = v.deliveryPaused ? 'closed' : (v.ownerStatus || v.status || 'open');
  const chip = $('#vstate'); chip.className = `ui-chip vstate ${st}`;
  $('#vstateT').textContent = v.deliveryPaused ? t('paused') : t(st);
  $('#brandName').textContent = v.name || 'dowiz';
  const mark = $('#brandMark'); if (v.logoUrl) { mark.src = v.logoUrl; mark.hidden = false; } else mark.hidden = true;
  document.title = `${v.name || 'dowiz'} · ${t('console')}`;
}
function openState(){
  const v = S.venue || {};
  sheet(`<p class="eyebrow" data-t="venue"></p><h2 data-t="setState"></h2>
    ${ui.list(STATES.map(s => choice({ key: s, pressed: (v.ownerStatus || v.status) === s && !v.deliveryPaused, data: { state: s }, tour: 'state.' + s })), { label: t('setState') })}
    ${switchEl('pauseD', v.deliveryPaused, 'paused', null, 'state.pauseDelivery')}`, { name: 'state' });
  for (const b of $$('[data-state]')) b.onclick = async () => {
    if (b.dataset.state === 'closed' && (v.ownerStatus || v.status) !== 'closed') {
      // Closing stops every new order; the old console asked, and so does this one.
      const ok = await confirm(t('closed'), t('closeVenueHint'), { danger: true });
      if (!ok) return openState();
    }
    try { await post('/owner/location', withLoc({ status: b.dataset.state })); await loadVenue(); paintVenue(); closeSheet(); toast(t('saved')); }
    catch (e) { toast(String(e.message || e)); }
  };
  $('#pauseD').onchange = async e => {
    try { await post('/owner/location', withLoc({ delivery_paused: e.target.checked })); await loadVenue(); paintVenue(); toast(t('saved')); }
    catch (err) { toast(String(err.message || err)); }
  };
}
$('#vstate').onclick = openState;

/// Language and reading currency, from the header.
function openPrefs(){
  sheet(`<p class="eyebrow" data-t="language"></p>
    ${chips({ id: 'langPick', values: LANGS.map(l => ({ value: l, label: l.toUpperCase() })), value: lang, attr: 'l', labelKey: 'language', tour: 'prefs.lang' })}
    <p class="eyebrow mt-3">${esc(baseCurrency())} → ${esc(displayCurrency())}</p>
    ${chips({ values: [baseCurrency(), ...CURRENCIES.filter(c => c !== baseCurrency())].map(c => ({ value: c, label: c })), value: displayCurrency(), attr: 'c', tour: 'prefs.currency' })}`, { name: 'prefs' });
  for (const b of $$('[data-l]', $('#sheetIn'))) b.onclick = async () => { setLang(b.dataset.l); mountNav(); paintVenue(); await rerender(); press($$('[data-l]', $('#sheetIn')), b); };
  for (const b of $$('[data-c]', $('#sheetIn'))) b.onclick = async () => { await setCurrency(baseCurrency(), b.dataset.c); press($$('[data-c]', $('#sheetIn')), b); await rerender(); };
}
$('#prefs').onclick = openPrefs;

// ── loading ─────────────────────────────────────────────────────────────────
//
// THE CONSOLE KEEPS ITS OWN COPY of the queue (`/lib/replica.js`), drawn on
// boot before any request and kept through an outage. A read asks for what
// CHANGED since the generation that copy is at; the server answers with the
// changes, or says it cannot and sends the whole list. Either way what lands
// in `S.orders` is what the server last said.
let copy = null;
function announce(list){
  // A new order rings once: the ids seen before are remembered per session.
  const fresh = list.filter(o => o.status === 'PENDING' && !S.seen.has(o.id));
  for (const o of list) S.seen.add(o.id);
  S.orders = list;
  if (fresh.length && S.booted && S.phase === 'ready') { S.fresh = new Set(fresh.map(o => o.id)); ring(); }
}
export async function loadOrders(){
  const R = await import('/lib/replica.js');
  copy = copy || R.load(store.loc);
  const since = copy.generation >= 0 ? `&since=${copy.generation}` : '';
  const d = await api(`/owner/orders?location_id=${encodeURIComponent(store.loc)}${since}`);
  if (d.full === false && Array.isArray(d.changes)) {
    const next = R.apply(copy, d.changes, d.generation);
    if (next) { copy = next; announce(copy.orders); return; }
    // The changes did not fit what is held -- a gap this copy cannot bridge.
    // Ask for the list rather than guess.
    const whole = await api(`/owner/orders?location_id=${encodeURIComponent(store.loc)}`);
    copy = R.replace(store.loc, whole.orders || [], whole.generation ?? -1);
    announce(copy.orders);
    return;
  }
  copy = R.replace(store.loc, d.orders || [], d.generation ?? -1);
  announce(copy.orders);
}
export async function loadStats(){ try { S.stats = await api(`/owner/dashboard?location_id=${encodeURIComponent(store.loc)}`); } catch {} }
/// The venue's storefront slug: the first label of the host on a venue
/// subdomain, else the location id (which is the slug at birth, but a venue
/// restored from a bundle may carry a different one).
const HOST_LABELS_OF_A_VENUE = 3;
const venueSlug = () => { const h = location.hostname.split('.'); return h.length >= HOST_LABELS_OF_A_VENUE ? h[0] : store.loc; };
export async function loadVenue(){
  try {
    const slug = venueSlug();
    const d = await api(`/public/locations/${encodeURIComponent(slug)}/menu?locale=${lang}&fresh=1`);
    S.venue = d.location; S.categories = d.categories || [];
    S.products = S.categories.flatMap(c => (c.products || []).map(p => ({ ...p, categoryId: c.id, categoryName: c.name, translations: {} })));
    // The other two languages, so a dish's translations can be edited: the
    // public menu answers in one language at a time, and it is the only
    // read of the catalogue this console has.
    const others = LANGS.filter(l => l !== lang);
    const rest = await Promise.all(others.map(l => api(`/public/locations/${encodeURIComponent(slug)}/menu?locale=${l}&fresh=1`).catch(() => null)));
    for (const [i, r] of rest.entries()) {
      if (!r) continue;
      for (const c of r.categories || []) for (const p of c.products || []) {
        const mine = S.products.find(x => x.id === p.id); if (mine) mine.translations[others[i]] = { name: p.name, description: p.description || '' };
      }
    }
    for (const p of S.products) p.translations[lang] = { name: p.name, description: p.description || '' };
    await setCurrency(d.location?.currencyCode || 'ALL', safeGet('dw_admin_cur') || d.location?.currencyCode || 'ALL');
  } catch {}
}
export async function loadCouriers(){ try { S.couriers = (await api('/owner/couriers')).couriers || []; } catch {} }
export async function loadStaff(){ try { S.staff = (await api('/owner/staff')).staff || []; } catch {} }

function ring(){
  try {
    const ac = new (window.AudioContext || window.webkitAudioContext)();
    for (let i = 0; i < 2; i++) {
      const o = ac.createOscillator(), g = ac.createGain();
      o.frequency.value = RING_HZ; o.connect(g); g.connect(ac.destination);
      const at = ac.currentTime + i * (RING_MS * 2) / 1000;
      g.gain.setValueAtTime(0.0001, at); g.gain.exponentialRampToValueAtTime(0.4, at + 0.01); g.gain.exponentialRampToValueAtTime(0.0001, at + RING_MS / 1000);
      o.start(at); o.stop(at + RING_MS / 1000 + 0.02);
    }
    navigator.vibrate?.([80, 60, 80]);
  } catch {}
}

async function boot(){
  if (!store.t || !store.loc) return renderLogin();
  document.documentElement.lang = lang;
  $('#top').hidden = false; $('#nav').hidden = false;
  S.booted = true; S.phase = 'loading';
  mountNav();
  // THE COPY IS DRAWN BEFORE ANYTHING IS ASKED. A console reopened at the
  // pass shows the queue it had, immediately; the reconciliation below
  // replaces it a moment later. An empty copy falls back to the skeleton.
  try {
    const R = await import('/lib/replica.js');
    copy = R.load(store.loc);
    if (copy.orders.length) {
      S.orders = copy.orders;
      S.stale = R.isStale(copy);
      await show(S.tab);
    }
  } catch { /* no replica: the skeleton below is what a first visit sees */ }
  if (!S.orders?.length) {
    $('#app').innerHTML = `<div class="screen">${loading(3)}</div>`;
  }
  await Promise.all([loadVenue(), loadStats(), loadCouriers()]);
  try { await loadOrders(); S.phase = 'ready'; S.error = null; } catch (e) { S.phase = 'error'; S.error = String(e.message || e); }
  paintVenue();
  await show(S.tab);
  paintLive();
  poll();
  openSocket();
  // Voice (admin/voice.js): the mic before the language button, where the
  // browser can recognise speech. A confirmed action re-reads what it moved.
  import('/admin/voice.js').then(m => m.mountVoice($('#top .top-in'), $('#prefs'), async () => {
    await Promise.all([loadOrders().catch(() => {}), loadVenue()]); paintVenue(); await rerender();
  })).catch(() => {});
  import('/admin/more.js').then(m => m.learnFromHash()).catch(() => {});
}
let pollTimer = null, pollN = 0;
// The queue moves in seconds; the dashboard's totals move in minutes. Reading
// both every 15 s doubled every poll for a number nobody watches that closely.
const STATS_EVERY = 4;
// THE HUB TELLS THIS CONSOLE when an order moves; the interval below is what
// catches whatever the socket missed. It is never switched off: a socket dies
// for reasons a kitchen cannot do anything about, and a queue that stops
// updating during service is worse than any number of requests.
let socket = null;
function openSocket(){
  import('/lib/live.js').then(({ live }) => {
    socket = live({
      token: store.t,
      // One nudge, one read. The message says an order moved; what an order
      // IS still comes from the same place it always did.
      onEvent: () => { if (!document.hidden) refreshNow(); },
    });
  }).catch(() => { /* no socket: the poll is the whole story */ });
}
let refreshing = false;
async function refreshNow(){
  if (refreshing || !S.booted) return;
  refreshing = true;
  try { await loadOrders(); S.phase = 'ready'; socket?.polled(); await rerender(); }
  catch {}
  finally { refreshing = false; }
}
function poll(){
  clearTimeout(pollTimer);
  pollTimer = setTimeout(async () => {
    if (!S.booted) return;
    if (!document.hidden && (!socket || socket.due())) {
      try { await loadOrders(); S.phase = 'ready'; S.stale = false; socket?.polled(); }
      catch {
        // OFFLINE IS A STATE, NOT A BLANK SCREEN. The copy stays on screen and
        // says how old it is; `paintLive` reads `S.stale`.
        const R = await import('/lib/replica.js').catch(() => null);
        if (R && copy) S.stale = R.isStale(copy);
      }
      if (++pollN % STATS_EVERY === 0) await loadStats();
      await rerender();
    }
    poll();
  // Idle venues poll slowly; when there's live activity, stay in sync.
  }, liveOrders().length ? POLL_MS : POLL_IDLE_MS);
}
document.addEventListener('visibilitychange', async () => { if (!document.hidden && S.booted) { try { await loadOrders(); } catch {} await rerender(); } });

bindSheetChrome();
retranslate(document);
boot();
