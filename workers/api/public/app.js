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
        search:'Kërkoni në meny', sortBy:'Renditni', sortPop:'Si në meny', sortLow:'Çmimi: nga i ulëti',
        sortHigh:'Çmimi: nga i larti', sortAz:'Sipas emrit', noHits:'Asgjë nuk u gjet', clear:'Pastroni',
        onlyAvail:'Vetëm në dispozicion',
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
        search:'Search the menu', sortBy:'Sort', sortPop:'As on the menu', sortLow:'Price: low first',
        sortHigh:'Price: high first', sortAz:'By name', noHits:'Nothing matched', clear:'Clear',
        onlyAvail:'Available only',
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
        search:'Пошук у меню', sortBy:'Сортування', sortLow:'Ціна: від дешевших',
        sortPop:'Як у меню', sortHigh:'Ціна: від дорожчих', sortAz:'За назвою',
        noHits:'Нічого не знайдено', clear:'Очистити', onlyAvail:'Лише в наявності',
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

// `q`, `sort` and `availOnly` live HERE and not in the DOM: renderMenu replaces
// the whole node, and a re-render for any other reason -- a theme change, a
// language switch -- would otherwise silently reset what the customer was
// looking at.
let state = { loc:null, cats:[], cart:loadCart(), placing:false,
              q:'', sort:'pop', availOnly:false };
// A cart line is a dish AND the choices made about it: two rolls of the same
// dish with different extras are two lines, not one with a quantity of two.
// The key is the product id plus its sorted option ids, so the same choices
// always collapse onto the same line and a different set never does.
const lineKey = (pid, mods) => [pid, ...[...(mods || [])].sort()].join('|');

function loadCart(){
  try {
    const raw = JSON.parse(safeGet('dw_cart_'+SLUG) || '{}');
    const out = {};
    for (const [k, v] of Object.entries(raw)) {
      // A cart saved before options existed is `{id: quantity}`. Migrated
      // rather than discarded: a customer who closed the tab mid-order should
      // find their basket, not an apology.
      if (typeof v === 'number') out[lineKey(k, [])] = { p: k, m: [], q: v };
      else if (v && typeof v === 'object' && v.p) out[k] = { p: v.p, m: v.m || [], q: v.q || 1 };
    }
    return out;
  } catch { return {}; }
}

function addLine(pid, mods, q){
  const k = lineKey(pid, mods);
  const cur = state.cart[k];
  state.cart[k] = { p: pid, m: mods || [], q: (cur?.q || 0) + q };
  saveCart();
}
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
      <span class="money">${money(o.total ?? 0)}</span></button>`;
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

// ── search, sort, filter ────────────────────────────────────────────────────
// A fifty-dish menu is a scroll; a search box turns it into a menu. All three
// run ON WHAT IS ALREADY LOADED -- the whole catalogue arrives in one response,
// so filtering is instant and works with no signal, which matters more here
// than anywhere: a customer standing outside a restaurant has one bar.
//
// The state lives in `state.q/sort/availOnly` rather than in the DOM, so a
// re-render (a theme change, a language switch) does not silently reset what
// the customer was looking at.
function normalise(x){
  // Accent- and case-insensitive. "Byrek" must find "Byrek me spinaq", and
  // "cmimi" must find "çmimi" — a search that demands the right diacritic on a
  // phone keyboard is a search nobody uses twice.
  return String(x ?? '').toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '');
}

function visibleCats(){
  const q = normalise(state.q).trim();
  const terms = q ? q.split(/\s+/) : [];
  const out = [];
  for (const c of state.cats) {
    let items = (c.products || []).filter(p => {
      if (state.availOnly && !p.available) return false;
      if (!terms.length) return true;
      // EVERY term must match somewhere — name, description or category. That
      // makes "sake roll" narrow rather than widen, which is what a person
      // typing a second word means by it.
      const hay = normalise(`${p.name} ${p.description || ''} ${c.name}`);
      return terms.every(t => hay.includes(t));
    });
    if (state.sort === 'low')  items = [...items].sort((a,b) => a.price - b.price);
    if (state.sort === 'high') items = [...items].sort((a,b) => b.price - a.price);
    if (state.sort === 'az')   items = [...items].sort((a,b) => a.name.localeCompare(b.name, lang));
    // 'pop' is the venue's own order, which is the default because a
    // restaurant arranges its menu deliberately and we should not overrule it.
    if (items.length) out.push({ ...c, products: items });
  }
  return out;
}

function renderMenu(){
  const L = state.loc, open = L.status === 'open';
  const cats = visibleCats();
  const filtering = Boolean(state.q?.trim()) || state.availOnly || state.sort !== 'pop';
  render(`
    <section class="hero">
      <h1>${esc(L.name)}</h1>
      <div class="meta">
        <span class="status-dot ${open ? '' : L.status === 'busy' ? 'busy' : 'closed'}">
          ${open ? `${esc(L.deliveryEta)} min` : esc(t('closed'))}</span>
        ${L.address ? `<span>${esc(L.address)}</span>` : ''}
        ${L.deliveryFee ? `<span><b class="money">${money(L.deliveryFee)}</b> ${esc(t('delivery').toLowerCase())}</span>`
                        : `<span><b>${esc(t('free'))}</b> ${esc(t('delivery').toLowerCase())}</span>`}
      </div>
      ${open ? '' : `<div class="notice"><i class="ti ti-clock-hour-9 i" aria-hidden="true"></i><div>${esc(t('closedHint'))}
        <a href="tel:${esc(L.phone)}">${esc(L.phone)}</a></div></div>`}
    </section>
    <div class="find">
      <label class="srch">
        <i class="ti ti-search" aria-hidden="true"></i>
        <input id="q" type="search" inputmode="search" autocomplete="off"
               placeholder="${esc(t('search'))}" aria-label="${esc(t('search'))}"
               value="${esc(state.q || '')}">
        ${state.q ? `<button id="qx" type="button" aria-label="${esc(t('clear'))}">
          <i class="ti ti-x" aria-hidden="true"></i></button>` : ''}
      </label>
      <div class="finds">
        <label class="sr" for="sort">${esc(t('sortBy'))}</label>
        <select id="sort" aria-label="${esc(t('sortBy'))}">
          <option value="pop"  ${state.sort==='pop' ?'selected':''}>${esc(t('sortPop'))}</option>
          <option value="low"  ${state.sort==='low' ?'selected':''}>${esc(t('sortLow'))}</option>
          <option value="high" ${state.sort==='high'?'selected':''}>${esc(t('sortHigh'))}</option>
          <option value="az"   ${state.sort==='az'  ?'selected':''}>${esc(t('sortAz'))}</option>
        </select>
        <label class="chk"><input type="checkbox" id="availOnly" ${state.availOnly?'checked':''}>
          <span>${esc(t('onlyAvail'))}</span></label>
      </div>
    </div>
    ${cats.length ? `
    <nav class="cats"><div class="cats-in">${cats.map((c,i) =>
      `<button class="cat" data-c="${esc(c.id)}" ${i===0?'aria-current="true"':''}>${esc(c.name)}</button>`).join('')}</div></nav>
    ${cats.map(c => `<h2 class="sec-h" id="c-${esc(c.id)}">${esc(c.name)}</h2>
      <div class="dishes">${c.products.map(dish).join('')}</div>`).join('')}`
    : `<div class="empty">
        <i class="ti ti-search-off" aria-hidden="true" style="font-size:2rem;display:block;margin-bottom:8px"></i>
        <b>${esc(t('noHits'))}</b>
        ${filtering ? `<button class="btn btn-ghost" id="qreset" style="margin-top:14px;max-width:16rem">${esc(t('clear'))}</button>` : ''}
      </div>`}
    <div style="height:28px"></div>`);
  bindMenu(); updateBar();
}

function dish(p){
  const out = !p.available;
  return `<button class="dish ${out ? 'sold-out' : ''}" data-p="${esc(p.id)}" ${out ? 'aria-disabled="true"' : ''}>
    <span>
      <span class="dish-name">${esc(p.name)}</span>
      ${p.description ? `<span class="dish-desc">${esc(p.description)}</span>` : ''}
      <span class="dish-price money">${money(p.price)}</span>
      ${out ? `<span class="badge">${esc(p.unavailableNote || t('soldOut'))}</span>` : ''}
    </span>
    <span class="dish-media">${p.imageUrl
      ? `<img src="${esc(p.imageUrl)}" alt="" loading="lazy" decoding="async"
           onerror="this.parentNode.innerHTML=this.dataset.fb" data-fb='${fallbackArt(p.name).replace(/'/g,"&#39;")}'>`
      : fallbackArt(p.name)}</span>
  </button>`;
}

function bindMenu(){
  // Search runs on every keystroke because it is local: there is no request to
  // debounce, and a fifty-item filter is microseconds. Focus and caret are
  // restored because renderMenu replaces the whole node.
  const q = $('#q');
  if (q) {
    q.oninput = () => {
      const pos = q.selectionStart;
      state.q = q.value;
      renderMenu();
      const again = $('#q');
      if (again) { again.focus(); try { again.setSelectionRange(pos, pos); } catch {} }
    };
  }
  const qx = $('#qx');
  if (qx) qx.onclick = () => { state.q = ''; renderMenu(); $('#q')?.focus(); };
  const qreset = $('#qreset');
  if (qreset) qreset.onclick = () => {
    state.q = ''; state.availOnly = false; state.sort = 'pop'; renderMenu();
  };
  const sort = $('#sort');
  if (sort) sort.onchange = () => { state.sort = sort.value; renderMenu(); };
  const av = $('#availOnly');
  if (av) av.onchange = () => { state.availOnly = av.checked; renderMenu(); };

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

// The choices a venue offers on a dish.
//
// Rendered from the RULES the hub sent: a group with max 1 is radios, anything
// else is checkboxes, and a required group says so. The hub validates and
// prices again on arrival -- this is the courtesy of catching it before the
// customer presses the button, not the authority.
function groupMarkup(g){
  const single = g.max === 1;
  const need = g.min >= 1
    ? `<span class="req">${esc(t('required'))}</span>`
    : g.max > 0 ? `<span class="hint">${g.max}</span>` : '';
  return `<fieldset class="mgroup" data-g="${esc(g.id)}"
            data-min="${g.min|0}" data-max="${g.max|0}">
    <legend>${esc(g.name)} ${need}</legend>
    ${(g.options || []).map(o => `
      <label class="mopt ${o.available === false ? 'off' : ''}">
        <input type="${single ? 'radio' : 'checkbox'}"
               name="mg-${esc(g.id)}" value="${esc(o.id)}"
               data-delta="${o.priceDelta | 0}"
               ${o.available === false ? 'disabled' : ''}>
        <span>${esc(o.name)}</span>
        ${o.priceDelta ? `<span class="mdelta money">${o.priceDelta > 0 ? '+' : '−'}${money(Math.abs(o.priceDelta))}</span>` : ''}
      </label>`).join('')}
  </fieldset>`;
}

function openDish(p){
  const groups = Array.isArray(p.modifierGroups) ? p.modifierGroups : [];
  sheet(`<h2>${esc(p.name)}</h2>
    ${p.description ? `<p style="color:var(--brand-text-muted);margin:8px 0 4px">${esc(p.description)}</p>` : ''}
    ${p.allergens?.length ? `<p class="allerg"><i class="ti ti-alert-circle i" aria-hidden="true"></i>${p.allergens.map(esc).join(', ')}</p>` : ''}
    ${groups.map(groupMarkup).join('')}
    <div class="row"><span class="dish-price money" id="dprice">${money(p.price)}</span>
      <span class="qty"><button id="dm" aria-label="−">−</button><span id="dq">1</span><button id="dp" aria-label="+">+</button></span></div>
    <p id="derr" class="err" hidden></p>
    <button class="btn" id="dadd" style="margin:14px 0">${esc(t('add'))}</button>
    <button class="btn btn-ghost" id="dar" style="margin-bottom:14px" hidden>
      <i class="ti ti-cube-3d-sphere" aria-hidden="true"></i><span>${esc(t('onTable'))}</span></button>
    <p id="darNote" class="geo" hidden></p>`);
  bindAr(p);
  let q = 1;

  // The chosen ids and what they add. Recomputed from the DOM on every change
  // rather than tracked separately: one source, and a radio group that swaps a
  // selection cannot leave a stale delta behind.
  const chosen = () => [...$('#sheetIn').querySelectorAll('.mgroup input:checked')];
  const repriceAndCheck = () => {
    const picked = chosen();
    const delta = picked.reduce((s, el) => s + (parseInt(el.dataset.delta, 10) || 0), 0);
    $('#dprice').textContent = money(Math.max(0, p.price + delta));

    // The same rules the hub enforces, checked here so the customer is told
    // before they press the button rather than after.
    let problem = null;
    for (const fs of $('#sheetIn').querySelectorAll('.mgroup')) {
      const min = parseInt(fs.dataset.min, 10) || 0;
      const max = parseInt(fs.dataset.max, 10) || 0;
      const n = fs.querySelectorAll('input:checked').length;
      const name = fs.querySelector('legend')?.firstChild?.textContent?.trim() || '';
      if (n === 0 && min >= 1) { problem = `${t('required')}: ${name}`; break; }
      if (n > 0 && n < min)   { problem = `${name}: ${min}`; break; }
      if (max > 0 && n > max) { problem = `${name}: ${max}`; break; }
    }
    const err = $('#derr'), add = $('#dadd');
    err.hidden = !problem;
    if (problem) err.textContent = problem;
    add.disabled = Boolean(problem);
    return picked.map(el => el.value);
  };
  $('#sheetIn').querySelectorAll('.mgroup input').forEach(el => el.onchange = repriceAndCheck);
  repriceAndCheck();

  $('#dm').onclick = () => { q = Math.max(1, q - 1); $('#dq').textContent = q; };
  $('#dp').onclick = () => { q = Math.min(99, q + 1); $('#dq').textContent = q; };
  $('#dadd').onclick = () => {
    const mods = repriceAndCheck();
    if ($('#dadd').disabled) return;
    // A line is keyed by the dish AND its choices: two rolls of the same dish
    // with different extras are two lines, not one with a quantity of two.
    addLine(p.id, mods, q);
    updateBar(); closeSheet();
    seaEvent('order_created', 24); toast(`${p.name} · ${q}`);
  };
}

// A line's unit price is the dish plus its chosen deltas, computed from the
// CATALOGUE rather than stored in the cart: a price that changed while the
// basket sat in a tab must re-price, not sell at yesterday's number.
function lineUnit(p, mods){
  const groups = Array.isArray(p.modifierGroups) ? p.modifierGroups : [];
  let d = 0;
  for (const g of groups) {
    for (const o of (g.options || [])) {
      if ((mods || []).includes(o.id)) d += (o.priceDelta | 0);
    }
  }
  return Math.max(0, p.price + d);
}
function lineNames(p, mods){
  const groups = Array.isArray(p.modifierGroups) ? p.modifierGroups : [];
  const out = [];
  for (const g of groups) {
    for (const o of (g.options || [])) if ((mods || []).includes(o.id)) out.push(o.name);
  }
  return out;
}
const cartLines = () => Object.entries(state.cart)
  .map(([k, l]) => ({ k, p: findProduct(l.p), m: l.m || [], q: l.q }))
  .filter(x => x.p && x.p.available);
const subtotal = () => cartLines().reduce((s, l) => s + lineUnit(l.p, l.m) * l.q, 0);
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
    ${lines.map(l => `<div class="row"><span><b>${esc(l.p.name)}</b>
      ${lineNames(l.p, l.m).length ? `<br><small style="color:var(--brand-text-muted)">${lineNames(l.p, l.m).map(esc).join(' · ')}</small>` : ''}
      <br><small class="money" style="color:var(--brand-text-muted)">${money(lineUnit(l.p, l.m))}</small></span>
      <span class="qty"><button data-m="${esc(l.k)}" aria-label="−">−</button>
      <span>${l.q}</span><button data-a="${esc(l.k)}" aria-label="+">+</button></span></div>`).join('')}
    ${totalsBlock()}
    <button class="btn" id="toCheckout" style="margin-bottom:12px">${esc(t('checkout'))}</button>`);
  $('#sheetIn').querySelectorAll('[data-a]').forEach(b => b.onclick = () => {
    const l = state.cart[b.dataset.a]; if (!l) return;
    l.q++; saveCart(); updateBar(); openCart();
  });
  $('#sheetIn').querySelectorAll('[data-m]').forEach(b => b.onclick = () => {
    const k = b.dataset.m, l = state.cart[k]; if (!l) return;
    l.q--; if (l.q <= 0) delete state.cart[k];
    saveCart(); updateBar(); openCart(); });
  $('#toCheckout').onclick = openCheckout;
}

function totalsBlock(){
  const s = subtotal(), d = deliveryFee(), L = state.loc;
  const below = L?.minOrder && s < L.minOrder;
  return `<div class="totals">
    <div class="row"><span>${esc(t('subtotal'))}</span><span class="money">${money(s)}</span></div>
    <div class="row"><span>${esc(t('delivery'))}</span><span class="money">${d ? money(d) : esc(t('free'))}</span></div>
    <div class="row grand"><span>${esc(t('total'))}</span><span class="money">${money(s + d)}</span></div>
    ${below ? `<div class="err">${esc(t('min'))}: <span class="money">${money(L.minOrder)}</span></div>` : ''}
  </div>`;
}

function openCheckout(){
  const s = subtotal(), L = state.loc;
  // A toast is plain text: .money cannot ride on a string. The value is still
  // the kernel's integer, and a toast shows one number for two seconds rather
  // than a column to line up, so tabular figures buy nothing here.
  if (L?.minOrder && s < L.minOrder) return toast(`${t('min')}: ${money(L.minOrder)}`); // money:toast
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
    // The ids only. The hub prices it -- `unit_price` is sent for the
    // storefront's own arithmetic to be checkable against, never trusted.
    const items = cartLines().map(l => ({
      product_id: l.p.id, modifier_ids: l.m, quantity: l.q, unit_price: lineUnit(l.p, l.m) }));
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
    <p class="sub" style="color:var(--brand-text-muted)">#${esc(String(order.id).slice(0,8))} · <span class="money">${money(order.total)}</span></p>
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
      <span class="money">${money(order.total ?? order.subtotal ?? 0)}</span></div></div>
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
