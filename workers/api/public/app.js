// Storefront. Renders the menu, holds a cart, places an order through the kernel
// and tracks it. It decides NOTHING about money or order state: prices come from
// the server and every status transition is the kernel's answer. The only sums
// computed here are for display, and they are recomputed server-side before the
// order is accepted -- a client total is never trusted.
const API = '/api';
const SLUG = new URLSearchParams(location.search).get('s') || 'demo';
const $ = (s, r = document) => r.querySelector(s);

// ── i18n. sq is the default: the diners are in Durrës. ──
const T = {
  sq: { cart:'Shporta', add:'Shto', total:'Totali', checkout:'Vazhdo', empty:'Shporta është bosh',
        emptyHint:'Zgjidhni një pjatë nga menuja', name:'Emri', phone:'Telefoni', address:'Adresa',
        note:'Shënim për korrierin', pay:'Mënyra e pagesës', cash:'Para në dorë', card:'Kartë',
        cashNote:'Paguani korrierit në dorëzim', cardNote:'Kartë ose Apple/Google Pay', place:'Porosit',
        subtotal:'Nëntotali', delivery:'Dërgesa', free:'Falas', closed:'Mbyllur tani',
        closedHint:'Telefononi për të porositur', soldOut:'S’ka', min:'Porosia minimale',
        sent:'Porosia u dërgua', track:'Ndiqni porosinë', offline:'Jeni offline — telefononi',
        required:'E detyrueshme', badPhone:'Numër i pavlefshëm', ordering:'Duke dërguar…',
        checkArea:'Kontrolloni adresën', checking:'Po kontrollojmë…',
        retry:'Provo përsëri', loadFail:'Menuja nuk u ngarkua', loading:'Po ngarkohet…',
        myOrders:'Porositë e mia', noOrders:'Ende asnjë porosi', when:'Kur', asap:'Sa më shpejt',
        onTable:'Shikoni në tryezë', arScan:'Drejtojeni nga tryeza…', arTap:'Prekni për ta vendosur', arFail:'Nuk u hap',
        later:'Në një orë tjetër', schedFail:'Koha nuk vlen',
        inArea:'Ne dërgojmë këtu', outArea:'Jashtë zonës sonë të dërgesës', noGeo:'Nuk morëm dot vendndodhjen',
        notify:'Merrni njoftime në Telegram', notifyHint:'Ju njoftojmë sa herë ndryshon porosia',
        st:{PENDING:'Duke pritur konfirmimin',CONFIRMED:'U konfirmua',PREPARING:'Po gatuhet',
            READY:'Gati',IN_DELIVERY:'Në rrugë',DELIVERED:'U dorëzua',
            REJECTED:'U refuzua',CANCELLED:'U anulua'} },
  en: { cart:'Cart', add:'Add', total:'Total', checkout:'Checkout', empty:'Your cart is empty',
        emptyHint:'Pick a dish from the menu', name:'Name', phone:'Phone', address:'Address',
        note:'Note for the courier', pay:'Payment', cash:'Cash', card:'Card',
        cashNote:'Pay the courier on delivery', cardNote:'Card or Apple/Google Pay', place:'Place order',
        subtotal:'Subtotal', delivery:'Delivery', free:'Free', closed:'Closed right now',
        closedHint:'Call to order', soldOut:'Sold out', min:'Minimum order',
        sent:'Order placed', track:'Track your order', offline:'You are offline — call instead',
        required:'Required', badPhone:'Invalid number', ordering:'Sending…',
        checkArea:'Check this address', checking:'Checking…',
        retry:'Try again', loadFail:'The menu did not load', loading:'Loading…',
        myOrders:'My orders', noOrders:'No orders yet', when:'When', asap:'As soon as possible',
        onTable:'See it on your table', arScan:'Point at your table…', arTap:'Tap to place it', arFail:'Could not open',
        later:'At a later time', schedFail:'That time will not work',
        inArea:'We deliver here', outArea:'Outside our delivery area', noGeo:'Could not get your location',
        notify:'Get updates on Telegram', notifyHint:'We\u2019ll message you each time this order moves',
        st:{PENDING:'Awaiting confirmation',CONFIRMED:'Confirmed',PREPARING:'Being prepared',
            READY:'Ready',IN_DELIVERY:'On the way',DELIVERED:'Delivered',
            REJECTED:'Rejected',CANCELLED:'Cancelled'} },
  uk: { cart:'Кошик', add:'Додати', total:'Разом', checkout:'Оформити', empty:'Кошик порожній',
        emptyHint:'Оберіть страву з меню', name:'Ім’я', phone:'Телефон', address:'Адреса',
        note:'Коментар кур’єру', pay:'Оплата', cash:'Готівка', card:'Картка',
        cashNote:'Оплата кур’єру при отриманні', cardNote:'Картка або Apple/Google Pay', place:'Замовити',
        subtotal:'Сума', delivery:'Доставка', free:'Безкоштовно', closed:'Зараз зачинено',
        closedHint:'Зателефонуйте, щоб замовити', soldOut:'Немає', min:'Мінімальне замовлення',
        sent:'Замовлення прийнято', track:'Стежити за замовленням', offline:'Немає зв’язку — телефонуйте',
        required:'Обов’язкове поле', badPhone:'Некоректний номер', ordering:'Надсилаємо…',
        checkArea:'Перевірити адресу', checking:'Перевіряємо…',
        retry:'Спробувати ще раз', loadFail:'Меню не завантажилось', loading:'Завантажуємо…',
        myOrders:'Мої замовлення', noOrders:'Замовлень ще немає', when:'Коли', asap:'Якнайшвидше',
        onTable:'Подивитись на столі', arScan:'Наведіть на стіл…', arTap:'Торкніться, щоб поставити', arFail:'Не вдалося відкрити',
        later:'На інший час', schedFail:'Такий час не підходить',
        inArea:'Сюди доставляємо', outArea:'Поза зоною доставки', noGeo:'Не вдалося визначити місце',
        notify:'Сповіщення в Telegram', notifyHint:'Напишемо щоразу, коли статус зміниться',
        st:{PENDING:'Очікує підтвердження',CONFIRMED:'Підтверджено',PREPARING:'Готується',
            READY:'Готове',IN_DELIVERY:'У дорозі',DELIVERED:'Доставлено',
            REJECTED:'Відхилено',CANCELLED:'Скасовано'} },
};
let lang = safeGet('dw_lang') || 'sq';
const t = k => (T[lang] && T[lang][k]) ?? T.en[k] ?? k;

// localStorage throws in private mode / blocked-site-data. Never let that break
// the page: an unreadable cart is an empty cart, not a crash.
function safeGet(k){ try { return localStorage.getItem(k); } catch { return null; } }
function safeSet(k,v){ try { localStorage.setItem(k,v); } catch {} }

let state = { loc:null, cats:[], cart:loadCart(), placing:false };
function loadCart(){ try { return JSON.parse(safeGet('dw_cart_'+SLUG) || '{}'); } catch { return {}; } }
function saveCart(){ safeSet('dw_cart_'+SLUG, JSON.stringify(state.cart)); }

const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
// Money arrives as integer minor units and is formatted only here. It is never
// parsed back out of the DOM.
const money = n => new Intl.NumberFormat(lang === 'uk' ? 'uk' : lang === 'en' ? 'en' : 'sq',
  { style:'currency', currency: state.loc?.currencyCode || 'ALL', maximumFractionDigits:0 }).format(n);

// A dish without a photo gets a deliberate mark, not a grey rectangle. Hue is a
// stable hash of the name so the same dish always looks the same.
function fallbackArt(name){
  let h = 0; for (const ch of name) h = (h * 31 + ch.codePointAt(0)) >>> 0;
  const hue = h % 360;
  return `<div class="fallback" style="background:linear-gradient(140deg,
    hsl(${hue} 46% 58%),hsl(${(hue + 38) % 360} 52% 42%))" aria-hidden="true">${esc(name.trim()[0] || '·')}</div>`;
}

// ── SEA ─────────────────────────────────────────────────────────────────────
// The dowiz ambient layer, wired rather than rewritten: webgl/particle-cloud is
// an existing 319-line WebGL2 field whose event vocabulary already maps order
// events to physics (order_created -> amber burst, courier_assigned -> teal
// stream, delivered -> gold bloom, dispatch_failed -> blood turbulence,
// pending_aging -> slow ember drift). The design language's first truth is that
// the engine already exists and the work is wiring it to every screen.
//
// The Sea carries NO text, NO price and NO decision. Those are Sheet-owned by
// contract, which is also why money never tweens: `money()` formats an integer
// the server sent and there is no animated path to it anywhere in this file.
let sea = null;
const SEA_FOR_STATUS = {
  PENDING:'pending_aging', CONFIRMED:'order_created', PREPARING:'order_created',
  READY:'courier_assigned', IN_DELIVERY:'courier_assigned',
  DELIVERED:'delivered', REJECTED:'dispatch_failed', CANCELLED:'dispatch_failed',
};
async function initSea(){
  if (sea) return;
  // Reduced motion does not mean "no sea" -- it means a calm one. The canvas
  // still renders; it simply stops moving at you.
  const calm = matchMedia('(prefers-reduced-motion: reduce)').matches;
  try {
    const { createParticleCloud } = await import('/lib/particle-cloud.js');
    sea = createParticleCloud();
    sea.init(document.getElementById('sea'));
    sea.setReducedMotion(calm);
    document.getElementById('sea').classList.toggle('calm', calm);
    addEventListener('resize', () => sea.resize(), { passive:true });
    // Touch makes ripples. Gated to real pointers so it is not driven by a
    // synthetic move on a tap.
    if (matchMedia('(hover: hover) and (pointer: fine)').matches) {
      addEventListener('pointermove', e => sea.setPointer(e.clientX / innerWidth, e.clientY / innerHeight), { passive:true });
    } else {
      addEventListener('touchstart', e => {
        const t = e.touches[0]; if (t) sea.setPointer(t.clientX / innerWidth, t.clientY / innerHeight);
      }, { passive:true });
    }
  } catch {
    // No WebGL2, or the module failed: the storefront is fully usable without
    // the Sea. It is atmosphere, never a dependency.
    sea = null;
  }
}
function seaEvent(kind, n){ try { sea && sea.burst(kind, n); } catch {} }
/// The sea matures WITH the order: amber at creation, teal once a courier has
/// it, gold on arrival. That progression is the design language's "море дозріває
/// зі станом", driven by the status the SERVER reports -- never guessed locally.
function seaForOrder(status){
  const kind = SEA_FOR_STATUS[status];
  if (kind) seaEvent(kind, status === 'DELIVERED' ? 220 : 90);
}

function toast(msg){ const el = $('#toast'); el.textContent = msg; el.classList.add('show');
  clearTimeout(toast._t); toast._t = setTimeout(() => el.classList.remove('show'), 2600); }

// ── data ──
// The venue's own colours.
//
// APPLIED AS TOKENS on the root element, never as per-element styles: every
// rule in this page already reads these tokens, so one assignment repaints the
// whole storefront and nothing can be left behind wearing the old palette.
//
// The server derives and CONTRAST-CHECKS the token set; the client only wears
// it. Deriving here would put the check somewhere a page can skip.
//
// Absent theme means the shipped palette stays, which is a complete
// contrast-checked design in its own right and not a placeholder.
function applyTheme(theme){
  const el = document.getElementById('venue-theme') || (() => {
    const s = document.createElement('style'); s.id = 'venue-theme';
    document.head.appendChild(s); return s;
  })();
  if (!theme || !theme.light) { el.textContent = ''; return; }
  // Only tokens, and only ones that look like tokens. The string arrives from
  // this venue's own hub, but a stylesheet is the wrong place to relax about
  // what goes into it.
  const safe = s => String(s || '').split(';')
    .map(d => d.trim())
    .filter(d => /^--brand-[a-z-]+:\s*#[0-9a-fA-F]{3,8}$/.test(d))
    .join(';');
  const light = safe(theme.light), dark = safe(theme.dark);
  if (!light) { el.textContent = ''; return; }
  // Same three-state structure the shipped palette uses: bare :root, then the
  // system preference guarded so an explicit light choice still wins, then the
  // explicit dark choice.
  el.textContent =
    `:root{${light}}` +
    (dark ? `@media (prefers-color-scheme:dark){:root:not([data-theme="light"]){${dark}}}` +
            `:root[data-theme="dark"]{${dark}}` : '');
}

// ── my orders ───────────────────────────────────────────────────────────────
// NO ACCOUNT, and that is the design rather than a shortcut. The hub keeps no
// customer registry: one would be a list of names, phones and addresses the
// venue does not need, cannot protect better than a browser can, and could be
// compelled to hand over if it existed.
//
// Instead each order comes back with a token scoped to that ONE order, and the
// browser keeps the list. The consequence is stated plainly rather than hidden:
// clear your browser data and the history goes with it. That is the same deal
// as every guest checkout, and it is the honest one -- linking history by phone
// number would mean anyone who knows your number can read your address.
const HIST_KEY = 'dw_orders';
const history = () => { try { return JSON.parse(safeGet(HIST_KEY) || '[]'); } catch { return []; } };
function remember(order){
  if (!order?.id || !order?.access_token) return;
  const list = history().filter(o => o.id !== order.id);
  list.unshift({ id: order.id, t: order.access_token, at: Date.now(), total: order.total ?? 0 });
  // Twenty is a year of ordering for a regular customer and keeps localStorage
  // small. The oldest fall off; the tokens expire at thirty days anyway.
  safeSet(HIST_KEY, JSON.stringify(list.slice(0, 20)));
}
/// Fetch one remembered order, sending ITS OWN token.
async function fetchRemembered(entry){
  const r = await fetch(`${API}/order/${encodeURIComponent(entry.id)}`,
                        { headers:{ authorization:'Bearer ' + entry.t } });
  if (!r.ok) throw new Error('HTTP ' + r.status);
  return r.json();
}
async function openHistory(){
  const list = history();
  if (!list.length) {
    sheet(`<h2>${esc(t('myOrders'))}</h2>
      <div class="empty"><i class="ti ti-receipt" aria-hidden="true"
        style="font-size:2rem;display:block;margin-bottom:8px"></i>
        <b>${esc(t('noOrders'))}</b><span>${esc(t('emptyHint'))}</span></div>
      <button class="btn btn-ghost" id="closeHist">OK</button>`);
    $('#closeHist').onclick = closeSheet;
    return;
  }
  sheet(`<h2>${esc(t('myOrders'))}</h2>
    <div id="histList">${list.map(() =>
      `<div class="skel" style="height:56px;margin-bottom:8px"></div>`).join('')}</div>
    <button class="btn btn-ghost" id="closeHist">OK</button>`);
  $('#closeHist').onclick = closeSheet;

  // Settled, not all-or-nothing: one expired token must not blank the list.
  const got = await Promise.allSettled(list.map(fetchRemembered));
  const rows = got.map((r, i) => {
    const e = list[i];
    if (r.status !== 'fulfilled') {
      return `<button class="hist gone" disabled>
        <span>#${esc(String(e.id).slice(-4))}</span>
        <span class="muted">${esc(t('loadFail'))}</span></button>`;
    }
    const o = r.value;
    return `<button class="hist" data-open="${esc(o.id)}">
      <span>#${esc(String(o.id).slice(-4))} · ${esc(t('st')[o.status] || o.status)}</span>
      <span>${money(o.total ?? 0)}</span></button>`;
  }).join('');
  const el = $('#histList'); if (el) el.innerHTML = rows;
  document.querySelectorAll('[data-open]').forEach(b => b.onclick = async () => {
    const e = history().find(x => x.id === b.dataset.open); if (!e) return;
    try { openTracking(await fetchRemembered(e)); } catch { toast(t('loadFail')); }
  });
}

async function load(){
  render(skeleton());
  try {
    const r = await fetch(`${API}/public/locations/${encodeURIComponent(SLUG)}/menu?locale=${lang}`);
    if (!r.ok) throw new Error('HTTP ' + r.status);
    const d = await r.json();
    state.loc = d.location; state.cats = d.categories || [];
    applyTheme(d.location.theme);
    document.title = d.location.name;
    $('#brandName').textContent = d.location.name;
    document.documentElement.lang = lang;
    renderMenu();
    initSea().then(() => seaEvent('pending_aging', 40));
  } catch (e) {
    // AN ERROR STATE, not an empty one. The distinction matters: "we have no
    // menu" and "we could not fetch the menu" look identical to a customer and
    // lead to opposite actions. This one says which, offers the way out that
    // actually works, and keeps the reason instead of swallowing it.
    //
    // Being offline is named separately because it is the one cause the
    // customer can fix themselves, and because a retry button is useless until
    // they do.
    const off = !navigator.onLine;
    render(`<div class="empty" role="alert">
      <i class="ti ti-${off ? 'wifi-off' : 'alert-triangle'}" aria-hidden="true"
         style="font-size:2rem;display:block;margin-bottom:8px"></i>
      <b>${esc(off ? t('offline') : t('loadFail'))}</b>
      <span class="reason">${esc(String(e.message || e))}</span>
      <button class="btn" style="margin-top:14px" id="retry">${esc(t('retry'))}</button>
      ${state.loc?.phone ? `<a class="btn btn-ghost" style="margin-top:8px" href="tel:${esc(state.loc.phone)}">${esc(state.loc.phone)}</a>` : ''}
    </div>`);
    $('#retry').onclick = load;
  }
}
const render = html => { $('#app').innerHTML = html; };
const skeleton = () => `<div class="hero"><div class="skel" style="height:44px;width:70%"></div>
  <div class="skel" style="height:18px;width:45%;margin-top:10px"></div></div>
  ${'<div class="skel" style="height:132px;margin-bottom:10px"></div>'.repeat(4)}`;

function renderMenu(){
  const L = state.loc, open = L.status === 'open';
  const cats = state.cats.filter(c => c.products?.length);
  render(`
    <section class="hero">
      <h1>${esc(L.name)}</h1>
      <div class="meta">
        <span class="status-dot ${open ? '' : L.status === 'busy' ? 'busy' : 'closed'}">
          ${open ? `${esc(L.deliveryEta)} min` : esc(t('closed'))}</span>
        ${L.address ? `<span>${esc(L.address)}</span>` : ''}
        ${L.deliveryFee ? `<span><b>${money(L.deliveryFee)}</b> ${esc(t('delivery').toLowerCase())}</span>`
                        : `<span><b>${esc(t('free'))}</b> ${esc(t('delivery').toLowerCase())}</span>`}
      </div>
      ${open ? '' : `<div class="notice"><i class="ti ti-clock-hour-9 i" aria-hidden="true"></i><div>${esc(t('closedHint'))}
        <a href="tel:${esc(L.phone)}">${esc(L.phone)}</a></div></div>`}
    </section>
    <nav class="cats"><div class="cats-in">${cats.map((c,i) =>
      `<button class="cat" data-c="${esc(c.id)}" ${i===0?'aria-current="true"':''}>${esc(c.name)}</button>`).join('')}</div></nav>
    ${cats.map(c => `<h2 class="sec-h" id="c-${esc(c.id)}">${esc(c.name)}</h2>
      <div class="dishes">${c.products.map(dish).join('')}</div>`).join('')}
    <div style="height:28px"></div>`);
  bindMenu(); updateBar();
}

function dish(p){
  const out = !p.available;
  return `<button class="dish ${out ? 'sold-out' : ''}" data-p="${esc(p.id)}" ${out ? 'aria-disabled="true"' : ''}>
    <span>
      <span class="dish-name">${esc(p.name)}</span>
      ${p.description ? `<span class="dish-desc">${esc(p.description)}</span>` : ''}
      <span class="dish-price">${money(p.price)}</span>
      ${out ? `<span class="badge">${esc(p.unavailableNote || t('soldOut'))}</span>` : ''}
    </span>
    <span class="dish-media">${p.imageUrl
      ? `<img src="${esc(p.imageUrl)}" alt="" loading="lazy" decoding="async"
           onerror="this.parentNode.innerHTML=this.dataset.fb" data-fb='${fallbackArt(p.name).replace(/'/g,"&#39;")}'>`
      : fallbackArt(p.name)}</span>
  </button>`;
}

function bindMenu(){
  // The category chip scrolls to its section and the chips follow the scroll.
  const secs = state.cats.filter(c=>c.products?.length).map(c => $('#c-' + CSS.escape(c.id))).filter(Boolean);
  document.querySelectorAll('.cat').forEach(b => b.onclick = () => {
    $('#c-' + CSS.escape(b.dataset.c))?.scrollIntoView({ behavior:'smooth', block:'start' });
  });
  const io = new IntersectionObserver(es => {
    const vis = es.filter(e => e.isIntersecting).sort((a,b) => a.boundingClientRect.top - b.boundingClientRect.top)[0];
    if (!vis) return;
    const id = vis.target.id.slice(2);
    document.querySelectorAll('.cat').forEach(c => c.setAttribute('aria-current', String(c.dataset.c === id)));
    document.querySelector('.cat[aria-current="true"]')?.scrollIntoView({ block:'nearest', inline:'center', behavior:'smooth' });
  }, { rootMargin:'-120px 0px -70% 0px' });
  secs.forEach(s => io.observe(s));

  document.querySelectorAll('.dish').forEach(b => b.onclick = () => {
    const p = findProduct(b.dataset.p);
    if (p && p.available) openDish(p);
  });
}
const findProduct = id => state.cats.flatMap(c => c.products || []).find(p => p.id === id);

// ── sheets ──
function sheet(html){ $('#sheetIn').innerHTML = html; $('#sheet').classList.add('show'); $('#scrim').classList.add('show'); }
function closeSheet(){ $('#sheet').classList.remove('show'); $('#scrim').classList.remove('show'); }
$('#scrim').onclick = closeSheet;
addEventListener('keydown', e => { if (e.key === 'Escape') closeSheet(); });

// "How big is it, actually?"
//
// The one question a photograph of food cannot answer. It appears only when the
// venue has BOTH uploaded a photo and measured the dish, and only on a device
// that can actually do it — WebXR's immersive-ar is Android Chrome and some
// headsets, and iOS Safari has none. A button that fails on tap is worse than
// no button.
//
// The module is imported ON DEMAND. A customer scrolling a menu should not pay
// for a WebGL renderer they will not open.
async function bindAr(p){
  const b = document.getElementById('dar');
  if (!b || !p.imageUrl || !(p.sizeCm > 0)) return;
  let ar;
  try { ar = await import('/lib/ar.js'); } catch { return; }
  if (!await ar.supported()) return;

  b.hidden = false;
  b.onclick = async () => {
    const note = document.getElementById('darNote');
    b.disabled = true;
    try {
      await ar.show({
        imageUrl: p.imageUrl,
        sizeCm: p.sizeCm,
        // The sheet stays on screen during the session, so the customer can add
        // the dish to the basket while looking at it on their table.
        overlay: document.getElementById('sheet'),
        onStatus: st => {
          if (!note) return;
          note.hidden = false;
          note.className = 'geo';
          note.textContent = st === 'ready' ? t('arTap')
            : st === 'placed' ? `${p.name} · ${p.sizeCm} cm`
            : t('arScan');
        },
        onEnd: () => { b.disabled = false; if (note) note.hidden = true; },
      });
    } catch {
      b.disabled = false;
      if (note) { note.hidden = false; note.className = 'geo bad'; note.textContent = t('arFail'); }
    }
  };
}

function openDish(p){
  sheet(`<h2>${esc(p.name)}</h2>
    ${p.description ? `<p style="color:var(--brand-text-muted);margin:8px 0 4px">${esc(p.description)}</p>` : ''}
    <div class="row"><span class="dish-price">${money(p.price)}</span>
      <span class="qty"><button id="dm" aria-label="−">−</button><span id="dq">1</span><button id="dp" aria-label="+">+</button></span></div>
    <button class="btn" id="dadd" style="margin:14px 0">${esc(t('add'))}</button>
    <button class="btn btn-ghost" id="dar" style="margin-bottom:14px" hidden>
      <i class="ti ti-cube-3d-sphere" aria-hidden="true"></i><span>${esc(t('onTable'))}</span></button>
    <p id="darNote" class="geo" hidden></p>`);
  bindAr(p);
  let q = 1;
  $('#dm').onclick = () => { q = Math.max(1, q - 1); $('#dq').textContent = q; };
  $('#dp').onclick = () => { q = Math.min(99, q + 1); $('#dq').textContent = q; };
  $('#dadd').onclick = () => { state.cart[p.id] = (state.cart[p.id] || 0) + q; saveCart(); updateBar(); closeSheet();
    seaEvent('order_created', 24); toast(`${p.name} · ${q}`); };
}

const cartLines = () => Object.entries(state.cart)
  .map(([id, q]) => ({ p: findProduct(id), q })).filter(x => x.p && x.p.available);
const subtotal = () => cartLines().reduce((s, l) => s + l.p.price * l.q, 0);
function deliveryFee(){
  const L = state.loc; if (!L) return 0;
  if (L.freeDeliveryThreshold != null && subtotal() >= L.freeDeliveryThreshold) return 0;
  return L.deliveryFee || 0;
}

function updateBar(){
  const n = cartLines().reduce((s, l) => s + l.q, 0);
  $('#bar').classList.toggle('show', n > 0);
  $('#barCount').textContent = n;
  $('#barLabel').textContent = t('cart');
  $('#barTotal').textContent = money(subtotal());
  $('#barBtn').onclick = openCart;
}

function openCart(){
  const lines = cartLines();
  if (!lines.length) return sheet(`<div class="empty"><b>${esc(t('empty'))}</b>${esc(t('emptyHint'))}</div>`);
  sheet(`<h2>${esc(t('cart'))}</h2>
    ${lines.map(l => `<div class="row"><span><b>${esc(l.p.name)}</b><br>
      <small style="color:var(--brand-text-muted)">${money(l.p.price)}</small></span>
      <span class="qty"><button data-m="${esc(l.p.id)}" aria-label="−">−</button>
      <span>${l.q}</span><button data-a="${esc(l.p.id)}" aria-label="+">+</button></span></div>`).join('')}
    ${totalsBlock()}
    <button class="btn" id="toCheckout" style="margin-bottom:12px">${esc(t('checkout'))}</button>`);
  $('#sheetIn').querySelectorAll('[data-a]').forEach(b => b.onclick = () => { state.cart[b.dataset.a]++; saveCart(); updateBar(); openCart(); });
  $('#sheetIn').querySelectorAll('[data-m]').forEach(b => b.onclick = () => {
    const id = b.dataset.m; state.cart[id]--; if (state.cart[id] <= 0) delete state.cart[id];
    saveCart(); updateBar(); openCart(); });
  $('#toCheckout').onclick = openCheckout;
}

function totalsBlock(){
  const s = subtotal(), d = deliveryFee(), L = state.loc;
  const below = L?.minOrder && s < L.minOrder;
  return `<div class="totals">
    <div class="row"><span>${esc(t('subtotal'))}</span><span>${money(s)}</span></div>
    <div class="row"><span>${esc(t('delivery'))}</span><span>${d ? money(d) : esc(t('free'))}</span></div>
    <div class="row grand"><span>${esc(t('total'))}</span><span>${money(s + d)}</span></div>
    ${below ? `<div class="err">${esc(t('min'))}: ${money(L.minOrder)}</div>` : ''}
  </div>`;
}

function openCheckout(){
  const s = subtotal(), L = state.loc;
  if (L?.minOrder && s < L.minOrder) return toast(`${t('min')}: ${money(L.minOrder)}`);
  sheet(`<h2>${esc(t('checkout'))}</h2>
    <label for="f-name">${esc(t('name'))}</label>
    <input id="f-name" autocomplete="name" value="${esc(safeGet('dw_name') || '')}">
    <label for="f-phone">${esc(t('phone'))}</label>
    <input id="f-phone" type="tel" inputmode="tel" autocomplete="tel" placeholder="+355…" value="${esc(safeGet('dw_phone') || '')}">
    <label for="f-addr">${esc(t('address'))}</label>
    <textarea id="f-addr" autocomplete="street-address">${esc(safeGet('dw_addr') || '')}</textarea>
    <label for="f-when">${esc(t('when'))}</label>
    <div class="when">
      <button type="button" class="chip on" data-when="asap" aria-pressed="true">${esc(t('asap'))}</button>
      <button type="button" class="chip" data-when="later" aria-pressed="false">${esc(t('later'))}</button>
    </div>
    <input type="datetime-local" id="f-when" hidden>
    ${state.loc?.hasDeliveryZones ? `
      <button type="button" class="btn btn-ghost" id="f-geo" style="margin-bottom:8px">
        <i class="ti ti-map-pin-check" aria-hidden="true"></i><span>${esc(t('checkArea'))}</span></button>
      <p id="f-geo-out" class="geo" hidden></p>` : ''}
    <label for="f-note">${esc(t('note'))}</label>
    <input id="f-note">
    <label>${esc(t('pay'))}</label>
    <div class="pays" role="radiogroup">
      <button class="pay" role="radio" aria-checked="true" data-pay="cash">
        <i class="ti ti-cash i" aria-hidden="true"></i><span class="t"><b>${esc(t('cash'))}</b><small>${esc(t('cashNote'))}</small></span></button>
      ${state.loc?.stripePublishableKey ? `<button class="pay" role="radio" aria-checked="false" data-pay="card">
        <i class="ti ti-credit-card i" aria-hidden="true"></i><span class="t"><b>${esc(t('card'))}</b><small>${esc(t('cardNote'))}</small></span></button>` : ''}
    </div>
    <div id="f-err"></div>
    ${totalsBlock()}
    <button class="btn" id="place" style="margin-bottom:12px">${esc(t('place'))}</button>`);
  let pay = 'cash';
  $('#sheetIn').querySelectorAll('[data-pay]').forEach(b => b.onclick = () => {
    if (b.disabled) return;
    $('#sheetIn').querySelectorAll('[data-pay]').forEach(x => x.setAttribute('aria-checked', String(x === b)));
    pay = b.dataset.pay;
  });
  $('#place').onclick = () => place(pay);

  // WHERE ARE YOU? Only asked when the venue has actually drawn a service area,
  // and only when the customer presses the button. A location prompt that fires
  // on its own is the kind of thing people dismiss reflexively and then distrust
  // the site for.
  //
  // Coordinates NEVER replace the typed address -- there is no geocoder here and
  // a courier needs a street and a door number, not a decimal pair. They ride
  // alongside it so the hub can answer one question: is this inside the area.
  // ASAP or later. The picker only appears once "later" is chosen, because a
  // datetime field on every checkout is a decision most customers do not want
  // to make and will mistrust if it is prefilled.
  let when = 'asap';
  $('#sheetIn').querySelectorAll('[data-when]').forEach(b => b.onclick = () => {
    when = b.dataset.when;
    $('#sheetIn').querySelectorAll('[data-when]').forEach(x => {
      x.classList.toggle('on', x === b);
      x.setAttribute('aria-pressed', String(x === b));
    });
    const f = $('#f-when');
    f.hidden = when !== 'later';
    if (when === 'later' && !f.value) {
      // Default an hour out, rounded to the next half hour: a sensible offer
      // rather than an empty field, and past the hub's ten-minute floor.
      const d = new Date(Date.now() + 60 * 60 * 1000);
      d.setMinutes(d.getMinutes() > 30 ? 60 : 30, 0, 0);
      const pad = n => String(n).padStart(2, '0');
      f.value = `${d.getFullYear()}-${pad(d.getMonth()+1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
      f.min = f.value;
    }
    if (when === 'later') f.focus();
  });

  const geo = $('#f-geo');
  if (geo) geo.onclick = () => {
    const out = $('#f-geo-out');
    out.hidden = false; out.className = 'geo'; out.textContent = t('checking');
    if (!navigator.geolocation) { out.className = 'geo bad'; out.textContent = t('noGeo'); return; }
    navigator.geolocation.getCurrentPosition(async pos => {
      // Micro-degrees, rounded once here and never re-derived: the whole system
      // holds coordinates as integers, and a float crossing into an order is
      // the thing MANIFESTO C2 forbids.
      state.geo = { lat_udeg: Math.round(pos.coords.latitude * 1e6),
                    lon_udeg: Math.round(pos.coords.longitude * 1e6) };
      try {
        const r = await fetch(`${API}/public/reach?lat_udeg=${state.geo.lat_udeg}&lon_udeg=${state.geo.lon_udeg}`);
        const d = await r.json();
        if (d.deliverable) { out.className = 'geo ok'; out.textContent = t('inArea'); }
        else {
          out.className = 'geo bad';
          const km = (d.nearestMetres || 0) / 1000;
          out.textContent = t('outArea') + (km >= 0.1 ? ` · ~${km.toFixed(1)} km` : '');
        }
      } catch { out.className = 'geo bad'; out.textContent = t('noGeo'); }
    }, () => {
      // A refusal is not a failure: the order still goes through unverified.
      state.geo = null; out.className = 'geo'; out.textContent = t('noGeo');
    }, { enableHighAccuracy: true, timeout: 10000, maximumAge: 60000 });
  };
}

/// The chosen time, or null for "as soon as possible".
function scheduledAt(){
  const f = document.getElementById('f-when');
  if (!f || f.hidden || !f.value) return null;
  const ms = new Date(f.value).getTime();
  return Number.isFinite(ms) ? ms : null;
}

async function place(pay){
  if (state.placing) return;
  const name = $('#f-name').value.trim(), phone = $('#f-phone').value.trim(),
        addr = $('#f-addr').value.trim(), note = $('#f-note').value.trim();
  const errs = [];
  if (!phone || phone.replace(/\D/g,'').length < 8) errs.push(t('badPhone'));
  if (!addr) errs.push(t('address') + ': ' + t('required'));
  $('#f-err').innerHTML = errs.map(e => `<div class="err">${esc(e)}</div>`).join('');
  if (errs.length) return;

  safeSet('dw_name', name); safeSet('dw_phone', phone); safeSet('dw_addr', addr);
  state.placing = true; $('#place').disabled = true; $('#place').textContent = t('ordering');
  try {
    const items = cartLines().map(l => ({ product_id: l.p.id, modifier_ids: [], quantity: l.q, unit_price: l.p.price }));
    const r = await fetch(`${API}/public/locations/${encodeURIComponent(SLUG)}/orders`, {
      method:'POST', headers:{ 'content-type':'application/json' },
      body: JSON.stringify({ items, contact:{ name, phone },
        fulfilment:{ kind:'delivery',
          address:{ line:addr, note: note || null,
                    ...(state.geo || {}) } },
        payment: pay, locale: lang,
        // Epoch milliseconds. `datetime-local` has no timezone, so it is read
        // in the CUSTOMER'S timezone -- which is the venue's too, for a
        // delivery you can walk to.
        ...(scheduledAt() ? { scheduled_for_ms: scheduledAt() } : {}) })
    });
    const d = await r.json();
    if (!r.ok) throw new Error(d.error || d.message || ('HTTP ' + r.status));
    state.cart = {}; saveCart(); updateBar();
    safeSet('dw_last_order', d.id);
    remember(d);
    dispatchEvent(new Event('dw:ordered'));
    // The order IS placed -- it is in the log. Only the card rail failed, so say
    // that instead of letting a silent absence look like success.
    if (d.payment_error) toast(String(d.payment_error));
    if (d.client_secret) return collectCard(d);
    seaEvent('order_created', 160);
    openTracking(d);
  } catch (e) {
    $('#f-err').innerHTML = `<div class="err">${esc(String(e.message || e))}</div>`;
  } finally {
    state.placing = false;
    const b = $('#place'); if (b) { b.disabled = false; b.textContent = t('place'); }
  }
}

// ── card ──
// Stripe.js is fetched on demand, so a venue taking only cash never loads it.
// The card itself goes from the browser straight to Stripe: no card field, and
// no value that could hold one, ever reaches dowiz.
let stripeLib = null;
async function loadStripe(){
  if (stripeLib) return stripeLib;
  await new Promise((ok, no) => {
    const el = document.createElement('script');
    el.src = 'https://js.stripe.com/v3/'; el.onload = ok; el.onerror = no;
    document.head.appendChild(el);
  });
  stripeLib = window.Stripe(state.loc.stripePublishableKey);
  return stripeLib;
}

async function collectCard(order){
  sheet(`<h2>${esc(t('pay'))}</h2>
    <p class="sub" style="color:var(--brand-text-muted)">#${esc(String(order.id).slice(0,8))} · ${money(order.total)}</p>
    <div id="pe" style="margin:var(--space-4) 0;min-height:180px"></div>
    <div id="pe-err"></div>
    <button class="btn" id="pay">${esc(t('place'))}</button>`);
  let stripe, elements;
  try {
    stripe = await loadStripe();
    elements = stripe.elements({ clientSecret: order.client_secret });
    // Apple Pay and Google Pay are not separate integrations: the Payment
    // Element offers whichever wallet the device supports, which is why the
    // intent asked for automatic_payment_methods.
    elements.create('payment', { layout: 'tabs' }).mount('#pe');
  } catch {
    $('#pe').innerHTML = `<div class="err">${esc(t('offline'))}</div>`;
    return;
  }
  $('#pay').onclick = async () => {
    const b = $('#pay'); b.disabled = true; b.textContent = t('ordering');
    const { error } = await stripe.confirmPayment({
      elements,
      confirmParams: { return_url: location.origin + '/?order=' + encodeURIComponent(order.id) },
      redirect: 'if_required',
    });
    if (error) {
      // Stripe's message is written for the customer; passing it through beats
      // replacing it with a generic one.
      $('#pe-err').innerHTML = `<div class="err">${esc(error.message || '')}</div>`;
      b.disabled = false; b.textContent = t('place');
      return;
    }
    // The order is not marked paid here. The WEBHOOK does that, because a
    // browser saying "it worked" is not evidence that money moved.
    seaEvent('order_created', 160);
    openTracking(order);
  };
}

const FLOW = ['PENDING','CONFIRMED','PREPARING','READY','IN_DELIVERY','DELIVERED'];
function openTracking(order){
  const st = order.status, i = FLOW.indexOf(st);
  if (openTracking._last !== st) { openTracking._last = st; seaForOrder(st); }
  const dead = st === 'REJECTED' || st === 'CANCELLED';
  // The follow link is what makes notifications possible at all: the server has
  // no way to reach this customer until they start a chat with the bot, and the
  // `start` payload is how their first message carries the order id with it.
  // Hidden when the venue has no bot, rather than linking somewhere broken, and
  // hidden once the order is finished, when there is nothing left to follow.
  const bot = state.loc?.telegramBot;
  const follow = (bot && !dead && st !== 'DELIVERED') ? `
    <a class="btn btn-ghost" style="margin-bottom:8px"
       href="https://t.me/${encodeURIComponent(bot)}?start=${encodeURIComponent(order.id)}"
       target="_blank" rel="noopener noreferrer">
      <i class="ti ti-brand-telegram" aria-hidden="true"></i><span>${esc(t('notify'))}</span></a>
    <p style="color:var(--brand-text-muted);font-size:var(--text-sm);margin:0 0 12px">
      ${esc(t('notifyHint'))}</p>` : '';
  sheet(`<h2>${esc(dead ? t('st')[st] : t('sent'))}</h2>
    <p style="color:var(--brand-text-muted);margin:6px 0 2px">#${esc(String(order.id).slice(0,8))}</p>
    ${dead ? '' : `<div class="track">${FLOW.map((s, n) => `
      <div class="step ${n < i ? 'done' : n === i ? 'now' : ''}">
        <span class="dot">${n < i ? `<i class="ti ti-check" aria-hidden="true"></i>` : ""}</span>
        <span><b>${esc(t('st')[s])}</b></span>
      </div>`).join('')}</div>`}
    <div class="totals"><div class="row grand"><span>${esc(t('total'))}</span>
      <span>${money(order.total ?? order.subtotal ?? 0)}</span></div></div>
    ${follow}
    <button class="btn btn-ghost" id="closeTrack" style="margin-bottom:12px">OK</button>`);
  $('#closeTrack').onclick = closeSheet;
  clearTimeout(openTracking._t);
  if (!dead && st !== 'DELIVERED') {
    // Poll: the order's state belongs to the server, so ask it rather than
    // guessing locally. A socket comes later; this is honest in the meantime.
    openTracking._t = setTimeout(async () => {
      try {
        // The order's OWN token. Polling used to be unauthenticated, which
        // meant anyone holding the id could read the address and phone.
        const tok = history().find(x => x.id === order.id)?.t;
        const r = await fetch(`${API}/order/${encodeURIComponent(order.id)}`,
                              tok ? { headers:{ authorization:'Bearer ' + tok } } : undefined);
        if (!r.ok) throw new Error('HTTP ' + r.status);
        const d = await r.json();
        openTracking._fails = 0;
        if ($('#sheet').classList.contains('show')) openTracking(d);
      } catch {
        // A poll that fails forever in silence leaves the customer watching a
        // status that stopped being true. After three misses -- around
        // forty seconds -- say so, and keep trying, because the usual cause is
        // a tunnel and it ends.
        openTracking._fails = (openTracking._fails || 0) + 1;
        if (openTracking._fails === 3) toast(t('loadFail'));
        if ($('#sheet').classList.contains('show')) openTracking(order);
      }
    }, 12000);
  }
}

// ── chrome ──
document.querySelectorAll('.lang').forEach(b => {
  b.setAttribute('aria-pressed', String(b.dataset.l === lang));
  b.onclick = () => { lang = b.dataset.l; safeSet('dw_lang', lang);
    document.querySelectorAll('.lang').forEach(x => x.setAttribute('aria-pressed', String(x.dataset.l === lang)));
    load(); };
});
function netState(){
  const el = $('#offline');
  el.hidden = navigator.onLine;
  el.textContent = t('offline') + (state.loc?.phone ? ` · ${state.loc.phone}` : '');
}
// The history button appears only once there IS history.
(function historyChrome(){
  const b = document.getElementById('histBtn');
  if (!b) return;
  const sync = () => { b.hidden = history().length === 0; };
  b.onclick = openHistory;
  sync();
  // Re-checked after a placement, which is the only moment the answer changes.
  addEventListener('dw:ordered', sync);
})();

addEventListener('online', netState); addEventListener('offline', netState);
netState(); load();
