// Storefront. Renders the menu, holds a cart, places an order through the kernel
// and tracks it. It decides NOTHING about money or order state: prices come from
// the server and every status transition is the kernel's answer. The only sums
// computed here are for display, and they are recomputed server-side before the
// order is accepted -- a client total is never trusted.
const API = '/api';
// WHICH VENUE THIS IS, and the host is the answer before the query string.
//
// A client hub is `sushi-durres.dowiz.org`, so the venue is the label in front
// of the platform domain -- a name a restaurant can print on a receipt, rather
// than `?s=sushi-durres`, which is a debug handle a customer can edit.
//
// `?s=` STILL WINS WHEN IT IS GIVEN, because the workers.dev deployment has no
// per-venue hostname and is how this is tested. And a workers.dev host is
// explicitly NOT read as naming a venue: `dowiz-api.sviatoslavsyniak.workers.dev`
// would otherwise be read as a venue called `dowiz-api` and every request would
// go to a hub that does not exist.
const SLUG = (() => {
  const explicit = new URLSearchParams(location.search).get('s');
  if (explicit) return explicit;
  const host = location.hostname.toLowerCase();
  if (host.endsWith('.workers.dev') || host === 'localhost') return 'demo';
  const labels = host.split('.');
  if (labels.length > 2 && labels[0] !== 'www') return labels[0];
  return 'demo';
})();
const $ = (s, r = document) => r.querySelector(s);

// ── i18n. sq is the default: the diners are in Durrës. ──
const T = {
  sq: { cart:'Shporta', add:'Shto', total:'Totali', chargedIn:'Do të paguhet', checkout:'Vazhdo', empty:'Shporta është bosh',
        emptyHint:'Zgjidhni një pjatë nga menuja', name:'Emri', phone:'Telefoni', address:'Adresa',
        note:'Shënim për korrierin', pay:'Mënyra e pagesës', cash:'Para në dorë', card:'Kartë',
        cashNote:'Paguani korrierit në dorëzim', cardNote:'Kartë ose Apple/Google Pay', place:'Porosit',
        subtotal:'Nëntotali', delivery:'Dërgesa', free:'Falas', closed:'Mbyllur tani',
        closedHint:'Telefononi për të porositur', soldOut:'S’ka', min:'Porosia minimale',
        sent:'Porosia u dërgua', track:'Ndiqni porosinë', offline:'Jeni offline — telefononi',
        required:'E detyrueshme', optional:'(opsionale)', badPhone:'Numër i pavlefshëm', ordering:'Duke dërguar…',
        checkArea:'Kontrolloni adresën', checking:'Po kontrollojmë…',
        retry:'Provo përsëri', loadFail:'Menuja nuk u ngarkua', loading:'Po ngarkohet…',
        hoursTitle:'Orari', openUntil:'Hapur deri në', opensAt2:'Hap në', closedNow:'Mbyllur tani', onMap:'Shfaq në hartë', directions:'Udhëzime', reviewsTitle:'Vlerësime', fromGoogle:'nga Google Maps', allReviews:'Të gjitha vlerësimet', callUs:'Telefono',
        myOrders:'Porositë e mia', noOrders:'Ende asnjë porosi', when:'Kur', asap:'Sa më shpejt',
        onTable:'Shikoni në tryezë', arScan:'Drejtojeni nga tryeza…', arTap:'Prekni për ta vendosur', arFail:'Nuk u hap',
        opensAt:'Hapet', pausedNow:'Dërgesat janë ndalur',
        search:'Kërkoni në meny', sortBy:'Renditni', sortPop:'Si në meny', sortLow:'Çmimi: nga i ulëti',
        sortHigh:'Çmimi: nga i larti', sortAz:'Sipas emrit', noHits:'Asgjë nuk u gjet', clear:'Pastroni',
        onlyAvail:'Vetëm në dispozicion',
        later:'Në një orë tjetër', schedFail:'Koha nuk vlen',
        inArea:'Ne dërgojmë këtu', outArea:'Jashtë zonës sonë të dërgesës', noGeo:'Nuk morëm dot vendndodhjen',
        notDeclared:'Alergjenët nuk janë deklaruar', noneOf14:'Asnjë nga 14 alergjenët',
        tip:'Bakshish për korrierin', tipNo:'Pa bakshish', tipOther:'Tjetër',
        sayHow:'Si ishte?', sayHint:'Vetëm restoranti e lexon. Pa yje, pa vlerësime.',
        sayGo:'Dërgo', saidIt:'Faleminderit',
        how:'Si e merrni', toDoor:'Dërgesë', toPickup:'E marr vetë', pickupAt:'Merreni te',
        avoid:'Alergjenët', avoidHint:'Fshihni pjatat që i përmbajnë', avoidOn:'Fshehur',
        avoidUnknown:'pjata pa deklaratë', clearAvoid:'Shfaqni të gjitha',
        promo:'Kodi i zbritjes', promoApply:'Apliko', promoOff:'Hiq', discount:'Zbritja',
        notify:'Merrni njoftime në Telegram', notifyHint:'Ju njoftojmë sa herë ndryshon porosia',
        st:{PENDING:'Duke pritur konfirmimin',CONFIRMED:'U konfirmua',PREPARING:'Po gatuhet',
            READY:'Gati',IN_DELIVERY:'Në rrugë',DELIVERED:'U dorëzua',
            REJECTED:'U refuzua',CANCELLED:'U anulua'} },
  en: { cart:'Cart', add:'Add', total:'Total', chargedIn:'You will be charged', checkout:'Checkout', empty:'Your cart is empty',
        emptyHint:'Pick a dish from the menu', name:'Name', phone:'Phone', address:'Address',
        note:'Note for the courier', pay:'Payment', cash:'Cash', card:'Card',
        cashNote:'Pay the courier on delivery', cardNote:'Card or Apple/Google Pay', place:'Place order',
        subtotal:'Subtotal', delivery:'Delivery', free:'Free', closed:'Closed right now',
        closedHint:'Call to order', soldOut:'Sold out', min:'Minimum order',
        sent:'Order placed', track:'Track your order', offline:'You are offline — call instead',
        required:'Required', optional:'(optional)', badPhone:'Invalid number', ordering:'Sending…',
        checkArea:'Check this address', checking:'Checking…',
        retry:'Try again', loadFail:'The menu did not load', loading:'Loading…',
        hoursTitle:'Opening hours', openUntil:'Open until', opensAt2:'Opens at', closedNow:'Closed now', onMap:'Show on map', directions:'Directions', reviewsTitle:'Reviews', fromGoogle:'from Google Maps', allReviews:'All reviews', callUs:'Call',
        myOrders:'My orders', noOrders:'No orders yet', when:'When', asap:'As soon as possible',
        onTable:'See it on your table', arScan:'Point at your table…', arTap:'Tap to place it', arFail:'Could not open',
        opensAt:'Opens', pausedNow:'Delivery is paused',
        search:'Search the menu', sortBy:'Sort', sortPop:'As on the menu', sortLow:'Price: low first',
        sortHigh:'Price: high first', sortAz:'By name', noHits:'Nothing matched', clear:'Clear',
        onlyAvail:'Available only',
        later:'At a later time', schedFail:'That time will not work',
        inArea:'We deliver here', outArea:'Outside our delivery area', noGeo:'Could not get your location',
        notDeclared:'Allergens not declared', noneOf14:'None of the 14 allergens',
        tip:'Tip for the courier', tipNo:'No tip', tipOther:'Other',
        sayHow:'How was it?', sayHint:'Only the venue reads this. No stars, no ratings.',
        sayGo:'Send', saidIt:'Thank you',
        how:'How you get it', toDoor:'Delivery', toPickup:'I will collect', pickupAt:'Collect at',
        avoid:'Allergens', avoidHint:'Hide dishes that contain them', avoidOn:'hidden',
        avoidUnknown:'undeclared dishes', clearAvoid:'Show everything',
        promo:'Promo code', promoApply:'Apply', promoOff:'Remove', discount:'Discount',
        notify:'Get updates on Telegram', notifyHint:'We\u2019ll message you each time this order moves',
        st:{PENDING:'Awaiting confirmation',CONFIRMED:'Confirmed',PREPARING:'Being prepared',
            READY:'Ready',IN_DELIVERY:'On the way',DELIVERED:'Delivered',
            REJECTED:'Rejected',CANCELLED:'Cancelled'} },
  uk: { cart:'Кошик', add:'Додати', total:'Разом', chargedIn:'Буде списано', checkout:'Оформити', empty:'Кошик порожній',
        emptyHint:'Оберіть страву з меню', name:'Ім’я', phone:'Телефон', address:'Адреса',
        note:'Коментар кур’єру', pay:'Оплата', cash:'Готівка', card:'Картка',
        cashNote:'Оплата кур’єру при отриманні', cardNote:'Картка або Apple/Google Pay', place:'Замовити',
        subtotal:'Сума', delivery:'Доставка', free:'Безкоштовно', closed:'Зараз зачинено',
        closedHint:'Зателефонуйте, щоб замовити', soldOut:'Немає', min:'Мінімальне замовлення',
        sent:'Замовлення прийнято', track:'Стежити за замовленням', offline:'Немає зв’язку — телефонуйте',
        required:'Обов’язкове поле', optional:'(необов’язково)', badPhone:'Некоректний номер', ordering:'Надсилаємо…',
        checkArea:'Перевірити адресу', checking:'Перевіряємо…',
        retry:'Спробувати ще раз', loadFail:'Меню не завантажилось', loading:'Завантажуємо…',
        hoursTitle:'Години роботи', openUntil:'Відчинено до', opensAt2:'Відчиняється о', closedNow:'Зараз зачинено', onMap:'Показати на карті', directions:'Маршрут', reviewsTitle:'Відгуки', fromGoogle:'з Google Maps', allReviews:'Усі відгуки', callUs:'Зателефонувати',
        myOrders:'Мої замовлення', noOrders:'Замовлень ще немає', when:'Коли', asap:'Якнайшвидше',
        onTable:'Подивитись на столі', arScan:'Наведіть на стіл…', arTap:'Торкніться, щоб поставити', arFail:'Не вдалося відкрити',
        opensAt:'Відчиняється', pausedNow:'Доставку призупинено',
        search:'Пошук у меню', sortBy:'Сортування', sortLow:'Ціна: від дешевших',
        sortPop:'Як у меню', sortHigh:'Ціна: від дорожчих', sortAz:'За назвою',
        noHits:'Нічого не знайдено', clear:'Очистити', onlyAvail:'Лише в наявності',
        later:'На інший час', schedFail:'Такий час не підходить',
        inArea:'Сюди доставляємо', outArea:'Поза зоною доставки', noGeo:'Не вдалося визначити місце',
        notDeclared:'Алергени не заявлено', noneOf14:'Жодного з 14 алергенів',
        tip:'Чайові кур\'єру', tipNo:'Без чайових', tipOther:'Інша сума',
        sayHow:'Як вам?', sayHint:'Читає лише заклад. Без зірок і оцінок.',
        sayGo:'Надіслати', saidIt:'Дякуємо',
        how:'Як заберете', toDoor:'Доставка', toPickup:'Заберу сам', pickupAt:'Забрати за адресою',
        avoid:'Алергени', avoidHint:'Сховати страви, що їх містять', avoidOn:'сховано',
        avoidUnknown:'страв без заяви', clearAvoid:'Показати все',
        promo:'Промокод', promoApply:'Застосувати', promoOff:'Прибрати', discount:'Знижка',
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
function loadAvoid(){
  // A stored value that is not an array of known codes is discarded rather than
  // trusted: a corrupted entry must not silently hide half a menu, nor silently
  // show a dish somebody is avoiding.
  try {
    const v = JSON.parse(safeGet('dw_avoid') || '[]');
    return Array.isArray(v) ? v.filter(c => ALLERGENS.some(a => a[0] === c)) : [];
  } catch { return []; }
}
let state = { loc:null, cats:[], cart:loadCart(), placing:false,
              q:'', sort:'pop', availOnly:false, avoid:[], avoidOpen:false,
              how:'delivery', tip:0 };
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
// A DISCOUNT BELONGS TO THE BASKET IT WAS QUOTED FOR. Change the basket and the
// number the hub gave back stops describing it -- a 50% code checked against
// 2700 would sit on screen claiming 1350 off a basket now worth 900. The order
// would still be priced correctly by the hub; the customer would have been
// shown a lie on the way there. So the quote is dropped with the change and
// re-asked.
function saveCart(){
  safeSet('dw_cart_'+SLUG, JSON.stringify(state.cart));
  state.promo = null;
}

/// Re-render the totals in place after the discount moved.
function refreshTotals(){
  const box = document.getElementById('totalsBox');
  if (box) box.outerHTML = totalsBlock();
}

const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
// Money arrives as integer minor units and is formatted only here. It is never
// parsed back out of the DOM.
//
// ONE MODULE FOR EVERY SURFACE. This was one of three copies; see /lib/money.js
// for what the other two got wrong. The customer may read prices in EUR or USD,
// and that is a READING -- the charge is in the venue's currency and the cart
// says so, because a converted total that looked like the amount taken would be
// a number nobody could reconcile against their bank.
import * as Money from '/lib/money.js';
let MoneyRates = null;
const baseCurrency = () => state.loc?.currencyCode || 'ALL';
const displayCurrency = () => state.currency || baseCurrency();
const money = n => Money.formatter({
  base: baseCurrency(),
  display: displayCurrency(),
  rates: MoneyRates,
  locale: lang === 'uk' ? 'uk' : lang === 'en' ? 'en' : 'sq',
})(n);
/// Switch the reading currency. Rates are fetched once and reused.
///
/// IT REDRAWS THE MENU, not `render()`. `render` is the low-level writer --
/// `render(html)` puts its argument into `#app` -- so calling it with no
/// argument wrote the string `undefined` over the whole storefront: every dish,
/// every price and every control disappeared the moment anyone chose EUR or
/// USD, with nothing in the console to say why. Measured against production on
/// 2026-09-17: 52 dish cards before the tap, 0 after.
async function setDisplayCurrency(code){
  state.currency = code;
  Money.remember(code);
  if (code !== baseCurrency() && (!MoneyRates || MoneyRates.base !== baseCurrency())) {
    MoneyRates = await Money.loadRates(baseCurrency());
  }
  renderMenu();
}

// A dish without a photo gets a deliberate mark, not a grey rectangle. Hue is a
// stable hash of the name so the same dish always looks the same.
function fallbackArt(name){
  let h = 0; for (const ch of name) h = (h * 31 + ch.codePointAt(0)) >>> 0;
  const hue = h % 360;
  return `<div class="fallback" data-hue="${hue}" aria-hidden="true">${esc(name.trim()[0] || '·')}</div>`;
}

// The gradient is per-dish and computed, so it cannot be a stylesheet rule --
// and `style-src 'self'` (public/_headers) forbids the `style=` attribute it
// used to ride in. CSSOM is neither an inline stylesheet nor an attribute, so
// the value is written once the markup is in the tree. Every path that inserts
// dish markup calls this; a missed call shows as a flat tile, not a blank one.
function paintFallbacks(root){
  for (const el of (root || document).querySelectorAll('.fallback[data-hue]')){
    const hue = Number(el.dataset.hue);
    el.style.background =
      `linear-gradient(140deg,hsl(${hue} 46% 58%),hsl(${(hue + 38) % 360} 52% 42%))`;
    el.removeAttribute('data-hue');
  }
}

// A dish photo that 404s used to swap itself out through an inline `onerror=`,
// which is script under `script-src 'self'` and never ran. `error` does not
// bubble, but it does capture, so one delegated listener covers every image on
// every surface of this page.
document.addEventListener('error', e => {
  const img = e.target;
  if (!(img instanceof HTMLImageElement) || !img.dataset.fb) return;
  const holder = img.parentNode;
  if (!holder) return;
  holder.innerHTML = fallbackArt(img.dataset.fb);
  paintFallbacks(holder);
}, true);

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
  // THE ONE THING ON THE PAGE THAT NEVER STOPS DRAWING. A venue whose customers
  // are on older phones should be able to turn it off, and turning it off must
  // mean the module is never fetched -- a flag that still downloads 12 KB and
  // starts a WebGL context has saved nothing.
  if (!on('sea')) return;
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
// THE TYPE PAIRS LIVE HERE, in the client, and the hub only ever names one.
//
// A font-family arriving from the network and going into a stylesheet is
// arbitrary text reaching CSS: `font-family: x; } body { display:none` is a
// defacement that escaping in the wrong place will not reliably stop. An id is
// looked up in this table, so the worst a compromised or mistaken hub can do is
// pick one of four pairings that already shipped.
//
// Every stack is system-resident. A storefront that waits on a web font is a
// storefront showing nothing on a Durrës 3G connection, and the wait lands on
// exactly the customers least able to absorb it.
const TYPE_PAIRS = {
  classic: { heading: "Georgia,'Iowan Old Style','Times New Roman',serif",
             body: "system-ui,-apple-system,'Segoe UI',Roboto,sans-serif" },
  modern:  { heading: "system-ui,-apple-system,'Segoe UI',sans-serif",
             body: "system-ui,-apple-system,'Segoe UI',sans-serif" },
  warm:    { heading: "Georgia,'Iowan Old Style','Palatino Linotype',serif",
             body: "Georgia,'Iowan Old Style','Palatino Linotype',serif" },
  plain:   { heading: "ui-sans-serif,system-ui,sans-serif",
             body: "ui-sans-serif,system-ui,sans-serif" },
};

// A FLAG IS OFF UNLESS THE HUB SAYS ON, and an absent `features` block means
// every flag is on. Those two rules are not the same and both are deliberate:
// an old hub that does not send the block gets the product it already had,
// while a hub that sends the block and omits a key has not heard of it, and
// something the hub has not heard of must not render.
function on(name){
  const f = state.loc?.features;
  if (!f) return true;
  return f[name] !== false;
}

// The venue's palette used to be written into a <style> element this function
// created. `style-src 'self'` (public/_headers) blocks a created <style> exactly
// as it blocks a hand-written one, so the venue's branding never applied. The
// tokens are now set as custom properties on :root through CSSOM, which the
// policy does not govern -- and an inline property on the root element outranks
// any :root rule in a sheet, which is the precedence the <style> block had.
//
// One cost of the move: CSSOM holds no @media, so the light/dark choice that the
// stylesheet made declaratively is made here instead, and re-made whenever the
// answer changes.
let VENUE_THEME = null;

function themeMode(){
  const explicit = document.documentElement.getAttribute('data-theme');
  if (explicit === 'dark' || explicit === 'light') return explicit;
  return matchMedia('(prefers-color-scheme:dark)').matches ? 'dark' : 'light';
}

// Names written last time, so a venue (or a theme switch) that drops a token
// does not leave the previous venue's value standing on the root element.
let themeApplied = [];

function paintTheme(){
  const root = document.documentElement;
  for (const name of themeApplied) root.style.removeProperty(name);
  themeApplied = [];
  if (!VENUE_THEME) return;
  const decls = (themeMode() === 'dark' && VENUE_THEME.dark.length)
    ? VENUE_THEME.dark : VENUE_THEME.light;
  for (const [name, value] of decls){ root.style.setProperty(name, value); themeApplied.push(name); }
  for (const [name, value] of VENUE_THEME.shape){ root.style.setProperty(name, value); themeApplied.push(name); }
}

function applyTheme(theme){
  if (!theme || !theme.light) { VENUE_THEME = null; paintTheme(); return; }
  // Only tokens, and only ones that look like tokens. The string arrives from
  // this venue's own hub, but the root element is the wrong place to relax
  // about what goes into it.
  const safe = s => String(s || '').split(';')
    .map(d => d.trim())
    .filter(d => /^--brand-[a-z-]+:\s*#[0-9a-fA-F]{3,8}$/.test(d))
    .map(d => { const i = d.indexOf(':'); return [d.slice(0, i).trim(), d.slice(i + 1).trim()]; });
  const light = safe(theme.light), dark = safe(theme.dark);
  if (!light.length) { VENUE_THEME = null; paintTheme(); return; }

  // The two non-colour tokens. Neither value from the hub reaches CSS: the
  // pair is an index into the table above, and the radius is put through
  // parseInt and clamped, so the only thing that can land on the element is
  // a number this function produced.
  const pair = TYPE_PAIRS[theme.typePair] || TYPE_PAIRS.classic;
  const r = Math.max(0, Math.min(20, parseInt(theme.radius, 10) || 0));
  const shape = [
    ['--brand-font-heading', pair.heading],
    ['--brand-font-body', pair.body],
    // One number, four steps. A venue picking "round" should get round
    // consistently rather than having to set four values that drift apart.
    ['--radius-sm', `${Math.round(r / 2)}px`],
    ['--radius-md', `${r}px`],
    ['--radius-lg', `${Math.round(r * 1.5)}px`],
    ['--radius-xl', `${r * 2}px`],
  ];
  VENUE_THEME = { light, dark, shape };
  paintTheme();
}

// The system preference is the one input to themeMode() that changes without
// this page doing anything.
matchMedia('(prefers-color-scheme:dark)').addEventListener('change', paintTheme);

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
        class="ico-lg"></i>
        <b>${esc(t('noOrders'))}</b><span>${esc(t('emptyHint'))}</span></div>
      <button class="btn btn-ghost" id="closeHist">OK</button>`);
    $('#closeHist').onclick = closeSheet;
    return;
  }
  sheet(`<h2>${esc(t('myOrders'))}</h2>
    <div id="histList">${list.map(() =>
      `<div class="skel skel-row"></div>`).join('')}</div>
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
    // Read here rather than at module scope: ALLERGENS is defined further down,
    // and a filter that silently came back empty would be the failure this
    // whole control exists to prevent.
    state.avoid = loadAvoid();
    applyTheme(d.location.theme);
    document.title = d.location.name;
    // THE VENUE'S OWN MARK, WHEN IT HAS ONE. The name is always written, because
    // a logo that fails to load must still leave something readable, and the
    // <img> is added beside it rather than instead of it -- the alt text is the
    // venue's name for exactly the same reason. `logoUrl` was dead in the
    // schema until the menu route started serving it; a venue without one is
    // the common case and renders as it always did.
    $('#brandName').textContent = d.location.name;
    const mark = $('#brandMark');
    if (mark) {
      if (d.location.logoUrl) {
        mark.src = d.location.logoUrl;
        mark.alt = d.location.name;
        mark.hidden = false;
      } else {
        mark.hidden = true;
      }
    }
    document.documentElement.lang = lang;
    // THE VENUE'S CURRENCY IS KNOWN ONLY NOW, so the reading currency is
    // resolved here: whatever this browser last chose, if the product still
    // offers it, otherwise the venue's own. Rates are fetched only when the two
    // differ, so a customer reading lek costs no third-party round trip.
    state.currency = Money.preferred(baseCurrency());
    if (state.currency !== baseCurrency()) {
      MoneyRates = await Money.loadRates(baseCurrency());
    }
    paintCurrencies();
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
         class="ico-lg"></i>
      <b>${esc(off ? t('offline') : t('loadFail'))}</b>
      <span class="reason">${esc(String(e.message || e))}</span>
      <button class="btn mt-3" id="retry">${esc(t('retry'))}</button>
      ${state.loc?.phone ? `<a class="btn btn-ghost mt-1" href="tel:${esc(state.loc.phone)}">${esc(state.loc.phone)}</a>` : ''}
    </div>`);
    $('#retry').onclick = load;
  }
}
const render = html => { $('#app').innerHTML = html; paintFallbacks($('#app')); };
const skeleton = () => `<div class="hero"><div class="skel skel-title"></div>
  <div class="skel skel-sub"></div></div>
  ${'<div class="skel skel-card"></div>'.repeat(4)}`;

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

// ── allergens ───────────────────────────────────────────────────────────────
//
// THE FILTER HIDES THE UNDECLARED TOO, and that is the whole design. A customer
// avoiding fish is not asking "which dishes are tagged fish"; they are asking
// "which dishes can I safely eat". A dish nobody has declared cannot answer
// that, so showing it would present an unanswered question as a clear one --
// which is precisely how somebody gets hurt. The count of what was hidden for
// that reason is shown, so the absence is visible rather than silent.
const ALLERGENS = [
  ['gluten',      { sq:'Gluten',      en:'Gluten',      uk:'Глютен' }],
  ['crustaceans', { sq:'Guaskorë',    en:'Crustaceans', uk:'Ракоподібні' }],
  ['eggs',        { sq:'Vezë',        en:'Eggs',        uk:'Яйця' }],
  ['fish',        { sq:'Peshk',       en:'Fish',        uk:'Риба' }],
  ['peanuts',     { sq:'Kikirikë',    en:'Peanuts',     uk:'Арахіс' }],
  ['soy',         { sq:'Soja',        en:'Soy',         uk:'Соя' }],
  ['milk',        { sq:'Qumësht',     en:'Milk',        uk:'Молоко' }],
  ['nuts',        { sq:'Arra',        en:'Nuts',        uk:'Горіхи' }],
  ['celery',      { sq:'Selino',      en:'Celery',      uk:'Селера' }],
  ['mustard',     { sq:'Mustardë',    en:'Mustard',     uk:'Гірчиця' }],
  ['sesame',      { sq:'Susam',       en:'Sesame',      uk:'Кунжут' }],
  ['sulphites',   { sq:'Sulfite',     en:'Sulphites',   uk:'Сульфіти' }],
  ['lupin',       { sq:'Lupin',       en:'Lupin',       uk:'Люпин' }],
  ['molluscs',    { sq:'Molusqe',     en:'Molluscs',    uk:'Молюски' }],
];
const allergenName = c => {
  const row = ALLERGENS.find(a => a[0] === c);
  return row ? (row[1][lang] || row[1].en) : c;
};

/// What the dish card says about allergens. THREE outcomes, not two: a dish
/// with nothing declared says so, in words, rather than showing the blank space
/// that a declared-clear dish also shows. Silence is the one thing this line
/// must never be, because silence reads as "nothing to worry about".
function allergenLine(p){
  if (!Array.isArray(p.allergens)) {
    return `<p class="allerg unknown"><i class="ti ti-help-circle i" aria-hidden="true"></i>${esc(t('notDeclared'))}</p>`;
  }
  if (!p.allergens.length) {
    return `<p class="allerg none"><i class="ti ti-check i" aria-hidden="true"></i>${esc(t('noneOf14'))}</p>`;
  }
  return `<p class="allerg"><i class="ti ti-alert-circle i" aria-hidden="true"></i>${
    p.allergens.map(c => esc(allergenName(c))).join(', ')}</p>`;
}

/// Why this dish is hidden, or null. Separated from the filter so the counts
/// below can say WHICH reason without running the test twice.
function hiddenBecause(p){
  const avoid = state.avoid || [];
  if (!avoid.length) return null;
  if (!Array.isArray(p.allergens)) return 'undeclared';
  return p.allergens.some(c => avoid.includes(c)) ? 'contains' : null;
}

function visibleCats(){
  const q = normalise(state.q).trim();
  const terms = q ? q.split(/\s+/) : [];
  const out = [];
  for (const c of state.cats) {
    let items = (c.products || []).filter(p => {
      if (state.availOnly && !p.available) return false;
      if (hiddenBecause(p)) return false;
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

// "Opens Monday at 11:00" rather than "closed".
//
// A customer told only that a place is shut goes somewhere else; one told when
// it opens comes back. Weekday 0 is Monday, matching the hub.
function whenOpens(n){
  const days = { uk:['пн','вт','ср','чт','пт','сб','нд'],
                 en:['Mon','Tue','Wed','Thu','Fri','Sat','Sun'],
                 sq:['Hën','Mar','Mër','Enj','Pre','Sht','Die'] };
  const d = (days[lang] || days.en)[n.weekday] ?? '';
  const hh = String(Math.floor((n.minute || 0) / 60)).padStart(2, '0');
  const mm = String((n.minute || 0) % 60).padStart(2, '0');
  // Today needs no day name — "opens at 18:00" reads better than "opens Tue".
  const todayIdx = (new Date().getDay() + 6) % 7;
  return n.weekday === todayIdx ? `${hh}:${mm}` : `${d} ${hh}:${mm}`;
}


// ── The venue, as a place and not only as a menu ───────────────────────────
//
// A storefront that lists food and says nothing about WHERE it is, WHEN it
// opens or WHAT ANYBODY THINKS OF IT is half a page: the customer leaves to
// find those three things on a map and orders from whatever they find there.
// This is the other half, and every part of it is the venue's own material --
// the schedule the kernel already enforces, the coordinates the owner set, and
// the Google listing, attributed to Google and linked back to it.
//
// EACH ROW APPEARS ONLY IF ITS FACT EXISTS. A venue with no coordinates draws
// no map link, one with no schedule draws no hours; a row that says "—" is a
// row that teaches the customer the page is broken.
const DAY_NAMES = {
  sq: ['E hënë','E martë','E mërkurë','E enjte','E premte','E shtunë','E diel'],
  en: ['Monday','Tuesday','Wednesday','Thursday','Friday','Saturday','Sunday'],
  uk: ['Понеділок','Вівторок','Середа','Четвер','Пʼятниця','Субота','Неділя'],
};
const hhmm = m => String(Math.floor(m / 60)).padStart(2, '0') + ':' + String(m % 60).padStart(2, '0');

/// Which weekday it is where the VENUE is, not where the phone is. A customer
/// reading this in Kyiv must see the venue's Friday, and `nextOpen.weekday`
/// from the server is already in the venue's own frame.
const todayAt = () => {
  const off = Number.isFinite(state.loc?.tzOffsetMinutes) ? state.loc.tzOffsetMinutes : 120;
  const local = new Date(Date.now() + off * 60_000);
  return { day: (local.getUTCDay() + 6) % 7, minute: local.getUTCHours() * 60 + local.getUTCMinutes() };
};

function hoursRows(){
  const week = state.loc?.hours;
  if (!Array.isArray(week) || week.length !== 7) return '';
  const names = DAY_NAMES[lang] || DAY_NAMES.en;
  const now = todayAt();
  return `<div class="hours" id="hoursBox" ${state.hoursOpen ? '' : 'hidden'}>${
    week.map((wins, i) => `<div class="hours-row${i === now.day ? ' on' : ''}">
      <span>${esc(names[i])}</span>
      <span>${wins.length ? wins.map(w => `${hhmm(w.open)}–${hhmm(w.close)}`).join(', ')
                          : esc(t('closedNow'))}</span></div>`).join('')}</div>`;
}

/// The one line at the top of the hours row: what the customer needs before the
/// table, which is whether they can order in the next minute.
function openLine(){
  const L = state.loc;
  if (L.status === 'open') {
    const week = Array.isArray(L.hours) ? L.hours : null;
    const now = todayAt();
    const win = week && (week[now.day] || []).find(w => now.minute >= w.open && now.minute < w.close);
    return win ? `${t('openUntil')} ${hhmm(win.close)}` : t('open') || t('openUntil');
  }
  if (L.nextOpen) return `${t('closedNow')} · ${t('opensAt2')} ${whenOpens(L.nextOpen)}`;
  return t('closedNow');
}

function venueCard(){
  const L = state.loc;
  const g = L.google || null;
  const hasGeo = Number.isFinite(L.lat) && Number.isFinite(L.lng);
  // Google's own deep link, built from coordinates rather than pasted: the
  // harvested short link can expire, a coordinate cannot.
  const mapsHref = hasGeo
    ? `https://www.google.com/maps/search/?api=1&query=${L.lat},${L.lng}`
    : (g && g.url) || null;
  const rows = [];

  if (Array.isArray(L.hours) && L.hours.length === 7) {
    rows.push(`<button class="vrow" type="button" id="hoursGo"
        aria-expanded="${state.hoursOpen ? 'true' : 'false'}" aria-controls="hoursBox">
      <i class="ti ti-clock-hour-9" aria-hidden="true"></i>
      <span>${esc(openLine())}</span>
      <i class="ti ti-chevron-right vrow-chev" aria-hidden="true"></i></button>${hoursRows()}`);
  }
  if (L.address) {
    rows.push(mapsHref
      ? `<a class="vrow" href="${esc(mapsHref)}" target="_blank" rel="noopener noreferrer">
          <i class="ti ti-map-pin" aria-hidden="true"></i><span>${esc(L.address)}</span>
          <span class="vrow-act">${esc(t('directions'))}</span></a>`
      : `<div class="vrow"><i class="ti ti-map-pin" aria-hidden="true"></i>
          <span>${esc(L.address)}</span></div>`);
  }
  if (L.phone) {
    rows.push(`<a class="vrow" href="tel:${esc(L.phone)}">
      <i class="ti ti-phone" aria-hidden="true"></i><span>${esc(L.phone)}</span>
      <span class="vrow-act">${esc(t('callUs'))}</span></a>`);
  }
  if (hasGeo) {
    // THE MAP IS NOT LOADED UNTIL IT IS ASKED FOR. MapLibre and a tile session
    // are a third of a megabyte and a round trip to another origin; a customer
    // who came to read a menu should not pay for either.
    rows.push(`<button class="vrow" type="button" id="mapGo"
        aria-expanded="false" aria-controls="mapBox">
      <i class="ti ti-gps" aria-hidden="true"></i><span>${esc(t('onMap'))}</span>
      <i class="ti ti-chevron-right vrow-chev" aria-hidden="true"></i></button>
      <div class="vmap" id="mapBox" hidden></div>`);
  }

  const reviews = (g && Array.isArray(g.reviews) ? g.reviews : []).slice(0, 6);
  const stars = n => '★'.repeat(Math.round(n || 0)) + '☆'.repeat(5 - Math.round(n || 0));
  const revs = reviews.length ? `
    <div class="revs">
      <h2 class="vsec-h">${esc(t('reviewsTitle'))}
        <span class="vsrc">${esc(t('fromGoogle'))}</span></h2>
      <div class="revs-in">${reviews.map(r => `
        <figure class="rev">
          <figcaption>
            <span class="rev-who" aria-hidden="true">${esc((r.author || '?').trim().charAt(0))}</span>
            <span><b>${esc(r.author || '')}</b>
            <span class="rev-stars" aria-label="${r.rating || 0}/5">${stars(r.rating)}</span></span>
          </figcaption>
          <blockquote>${esc(r.text || '')}</blockquote>
        </figure>`).join('')}</div>
    </div>` : '';

  if (!rows.length && !g) return '';
  return `
    <section class="venue">
      ${g && g.rating ? `<div class="vrate">
        <b>${esc(String(g.rating).replace('.', lang === 'en' ? '.' : ','))}</b>
        <span class="rev-stars" aria-hidden="true">${stars(g.rating)}</span>
        ${g.reviewCount ? `<span class="vcount">${g.reviewCount}</span>` : ''}
        ${g.url ? `<a class="vsrc" href="${esc(g.url)}" target="_blank" rel="noopener noreferrer"
           >${esc(t('fromGoogle'))}</a>` : `<span class="vsrc">${esc(t('fromGoogle'))}</span>`}
      </div>` : ''}
      ${rows.join('')}
      ${revs}
    </section>`;
}

/// MapLibre, imported the first time somebody asks to see the map.
async function showMap(){
  const box = $('#mapBox');
  const btn = $('#mapGo');
  if (!box) return;
  const opening = box.hidden;
  box.hidden = !opening;
  btn?.setAttribute('aria-expanded', String(opening));
  if (!opening || box.dataset.drawn) return;
  box.dataset.drawn = '1';
  try {
    const { default: maplibregl } = await import('/lib/map/maplibre-gl.js');
    const map = new maplibregl.Map({
      container: box,
      style: 'https://tiles.openfreemap.org/styles/liberty',
      center: [state.loc.lng, state.loc.lat],
      zoom: 16,
      attributionControl: true,
    });
    new maplibregl.Marker().setLngLat([state.loc.lng, state.loc.lat]).addTo(map);
  } catch (e) {
    // A map that will not load is a missing map, not a broken page: say so in
    // the box that was opened for it and leave everything else alone.
    box.textContent = t('loadFail');
    box.dataset.drawn = '';
  }
}

function renderMenu(){
  const L = state.loc, open = L.status === 'open';
  const cats = visibleCats();
  const filtering = Boolean(state.q?.trim()) || state.availOnly || state.sort !== 'pop'
    || Boolean(state.avoid?.length);
  // Counted BEFORE the render, and split by reason. "12 hidden" without saying
  // that four of them are hidden because nobody declared them would hide the
  // gap in the venue's own data behind the customer's own filter.
  const hiddenCount = state.avoid?.length
    ? state.cats.reduce((acc, c) => {
        for (const p of (c.products || [])) {
          const why = hiddenBecause(p);
          if (why) acc[why] += 1;
        }
        return acc;
      }, { contains: 0, undeclared: 0 })
    : null;
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
      ${open ? '' : `<div class="notice"><i class="ti ti-clock-hour-9 i" aria-hidden="true"></i><div>
        ${L.closedReason === 'paused' ? esc(t('pausedNow'))
          : L.nextOpen ? `${esc(t('opensAt'))} ${esc(whenOpens(L.nextOpen))}`
          : esc(t('closedHint'))}
        ${L.phone ? `<br><a href="tel:${esc(L.phone)}">${esc(L.phone)}</a>` : ''}</div></div>`}
    </section>
    ${venueCard()}
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
        ${on('allergen_filter') ? `<button type="button" class="chip ${state.avoid?.length ? 'on' : ''}" id="avoidGo"
                aria-expanded="${state.avoidOpen ? 'true' : 'false'}" aria-controls="avoidBox">
          <i class="ti ti-alert-circle" aria-hidden="true"></i>
          ${esc(t('avoid'))}${state.avoid?.length ? ` · ${state.avoid.length}` : ''}</button>` : ''}
      </div>
      <div class="avoid" id="avoidBox" ${state.avoidOpen ? '' : 'hidden'}>
        <p class="avoid-h">${esc(t('avoidHint'))}</p>
        <div class="avoid-in">
          ${ALLERGENS.map(([code]) => `<button type="button" class="chip ${state.avoid?.includes(code) ? 'on' : ''}"
             data-avoid="${code}" aria-pressed="${state.avoid?.includes(code) ? 'true' : 'false'}"
             >${esc(allergenName(code))}</button>`).join('')}
        </div>
        ${hiddenCount ? `<p class="avoid-h">${hiddenCount.contains} ${esc(t('avoidOn'))}${
          hiddenCount.undeclared ? `, ${hiddenCount.undeclared} ${esc(t('avoidUnknown'))}` : ''}
          ${state.avoid?.length ? `· <button type="button" class="linky" id="avoidClear">${esc(t('clearAvoid'))}</button>` : ''}</p>` : ''}
      </div>
    </div>
    ${cats.length ? `
    <nav class="cats"><div class="cats-in">${cats.map((c,i) =>
      `<button class="cat" data-c="${esc(c.id)}" ${i===0?'aria-current="true"':''}>${esc(c.name)}</button>`).join('')}</div></nav>
    ${cats.map(c => `<h2 class="sec-h" id="c-${esc(c.id)}">${esc(c.name)}</h2>
      <div class="dishes">${c.products.map(dish).join('')}</div>`).join('')}`
    : `<div class="empty">
        <i class="ti ti-search-off" aria-hidden="true" class="ico-lg"></i>
        <b>${esc(t('noHits'))}</b>
        ${filtering ? `<button class="btn btn-ghost mt-3 w-cap" id="qreset">${esc(t('clear'))}</button>` : ''}
      </div>`}
    <div class="tail-gap"></div>`);
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
           data-fb="${esc(p.name)}">`
      : fallbackArt(p.name)}</span>
  </button>`;
}

function bindMenu(){
  // The hours table and the map toggle the node they already have rather than
  // re-rendering the menu: a re-render loses the scroll position, and the
  // customer opened the hours to read them, not to be sent back to the top.
  const hg = $('#hoursGo');
  if (hg) {
    hg.onclick = () => {
      const box = $('#hoursBox');
      if (!box) return;
      state.hoursOpen = box.hidden;
      box.hidden = !box.hidden;
      hg.setAttribute('aria-expanded', String(!box.hidden));
    };
  }
  const mg = $('#mapGo');
  if (mg) mg.onclick = showMap;

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
    // The allergen choice is NOT cleared here. "Show everything" on an empty
    // search means the search; a control that quietly dropped somebody's
    // allergens while they were looking elsewhere would be the worst bug on
    // this screen.
    state.q = ''; state.availOnly = false; state.sort = 'pop'; renderMenu();
  };
  const sort = $('#sort');
  if (sort) sort.onchange = () => { state.sort = sort.value; renderMenu(); };
  const av = $('#availOnly');
  if (av) av.onchange = () => { state.availOnly = av.checked; renderMenu(); };

  // The chosen allergens SURVIVE a reload. Somebody who is allergic to fish is
  // allergic to fish tomorrow too, and asking them to re-tick it every visit is
  // how a safety control becomes one people stop using.
  const ago = $('#avoidGo');
  if (ago) ago.onclick = () => { state.avoidOpen = !state.avoidOpen; renderMenu(); };
  document.querySelectorAll('[data-avoid]').forEach(b => b.onclick = () => {
    const c = b.dataset.avoid, cur = state.avoid || [];
    state.avoid = cur.includes(c) ? cur.filter(x => x !== c) : [...cur, c];
    safeSet('dw_avoid', JSON.stringify(state.avoid));
    renderMenu();
  });
  const acl = $('#avoidClear');
  if (acl) acl.onclick = () => {
    state.avoid = []; safeSet('dw_avoid', '[]'); renderMenu();
  };

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
function sheet(html){ $('#sheetIn').innerHTML = html; paintFallbacks($('#sheetIn')); $('#sheet').classList.add('show'); $('#scrim').classList.add('show'); }
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
  if (!b || !on('ar') || !p.imageUrl || !(p.sizeCm > 0)) return;
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
    : g.max > 0 ? `<span class="req">${g.max}</span>` : '';
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
    ${p.description ? `<p class="muted d-desc">${esc(p.description)}</p>` : ''}
    ${allergenLine(p)}
    ${groups.map(groupMarkup).join('')}
    <div class="row"><span class="dish-price money" id="dprice">${money(p.price)}</span>
      <span class="qty"><button id="dm" aria-label="−">−</button><span id="dq">1</span><button id="dp" aria-label="+">+</button></span></div>
    <p id="derr" class="err" hidden></p>
    <button class="btn my-3" id="dadd">${esc(t('add'))}</button>
    <button class="btn btn-ghost mb-3" id="dar" hidden>
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
  // Collection costs nothing to deliver. The hub decides this again for the
  // real order; showing anything else here would be a number the customer
  // watches change at the last step.
  if (state.how === 'pickup') return 0;
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
      ${lineNames(l.p, l.m).length ? `<br><small class="muted">${lineNames(l.p, l.m).map(esc).join(' · ')}</small>` : ''}
      <br><small class="money muted">${money(lineUnit(l.p, l.m))}</small></span>
      <span class="qty"><button data-m="${esc(l.k)}" aria-label="−">−</button>
      <span>${l.q}</span><button data-a="${esc(l.k)}" aria-label="+">+</button></span></div>`).join('')}
    ${totalsBlock()}
    <button class="btn mb-2" id="toCheckout">${esc(t('checkout'))}</button>`);
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
  // The hub's number, never one worked out here. The storefront knows what a
  // code took off only because the hub said so; re-deriving it locally would
  // give two answers to the same question and put the wrong one on screen.
  const cut = state.promo ? state.promo.discount : 0;
  // Never on a collection: the control is hidden there, and a stale amount left
  // over from a delivery the customer changed their mind about must not ride
  // along into the total.
  const tip = state.how === 'pickup' ? 0 : (state.tip || 0);
  return `<div class="totals" id="totalsBox">
    <div class="row"><span>${esc(t('subtotal'))}</span><span class="money">${money(s)}</span></div>
    ${cut ? `<div class="row cut"><span>${esc(t('discount'))} · ${esc(state.promo.code)}</span>
      <span class="money">−${money(cut)}</span></div>` : ''}
    <div class="row"><span>${esc(t('delivery'))}</span><span class="money">${d ? money(d) : esc(t('free'))}</span></div>
    ${tip ? `<div class="row"><span>${esc(t('tip'))}</span>
      <span class="money">${money(tip)}</span></div>` : ''}
    <div class="row grand"><span>${esc(t('total'))}</span><span class="money">${money(s - cut + d + tip)}</span></div>
    ${chargeNote(s - cut + d + tip)}
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
    <label for="f-phone">${esc(t('phone'))} <span class="opt">${esc(t('optional'))}</span></label>
    <input id="f-phone" type="tel" inputmode="tel" autocomplete="tel" placeholder="+355…" value="${esc(safeGet('dw_phone') || '')}">
    ${state.loc?.pickup ? `
    <label>${esc(t('how'))}</label>
    <div class="when" role="radiogroup" aria-label="${esc(t('how'))}">
      <button type="button" class="chip ${state.how !== 'pickup' ? 'on' : ''}" data-how="delivery"
              aria-pressed="${state.how !== 'pickup'}">${esc(t('toDoor'))}</button>
      <button type="button" class="chip ${state.how === 'pickup' ? 'on' : ''}" data-how="pickup"
              aria-pressed="${state.how === 'pickup'}">${esc(t('toPickup'))}</button>
    </div>` : ''}
    <div id="addrBox" ${state.how === 'pickup' && state.loc?.pickup ? 'hidden' : ''}>
      <label for="f-addr">${esc(t('address'))}</label>
      <textarea id="f-addr" autocomplete="street-address">${esc(safeGet('dw_addr') || '')}</textarea>
    </div>
    ${state.how === 'pickup' && state.loc?.address ? `
      <p class="geo ok">${esc(t('pickupAt'))}: ${esc(state.loc.address)}</p>` : ''}
    <label for="f-when">${esc(t('when'))}</label>
    <div class="when">
      <button type="button" class="chip on" data-when="asap" aria-pressed="true">${esc(t('asap'))}</button>
      <button type="button" class="chip" data-when="later" aria-pressed="false">${esc(t('later'))}</button>
    </div>
    <input type="datetime-local" id="f-when" hidden>
    ${state.loc?.hasDeliveryZones ? `
      <button type="button" class="btn btn-ghost mb-1" id="f-geo">
        <i class="ti ti-map-pin-check" aria-hidden="true"></i><span>${esc(t('checkArea'))}</span></button>
      <p id="f-geo-out" class="geo" hidden></p>` : ''}
    <label for="f-note">${esc(t('note'))}</label>
    <input id="f-note">
    ${state.how !== 'pickup' && on('tips') ? `
    <label>${esc(t('tip'))}</label>
    <div class="when" role="radiogroup" aria-label="${esc(t('tip'))}">
      ${TIPS.map(v => `<button type="button" class="chip ${state.tip === v ? 'on' : ''}"
         data-tip="${v}" aria-pressed="${state.tip === v}">${
           v ? `<span class="money">${money(v)}</span>` : esc(t('tipNo'))}</button>`).join('')}
    </div>` : ''}
    <label>${esc(t('pay'))}</label>
    <div class="pays" role="radiogroup">
      <button class="pay" role="radio" aria-checked="true" data-pay="cash">
        <i class="ti ti-cash i" aria-hidden="true"></i><span class="t"><b>${esc(t('cash'))}</b><small>${esc(t('cashNote'))}</small></span></button>
      ${state.loc?.stripePublishableKey ? `<button class="pay" role="radio" aria-checked="false" data-pay="card">
        <i class="ti ti-credit-card i" aria-hidden="true"></i><span class="t"><b>${esc(t('card'))}</b><small>${esc(t('cardNote'))}</small></span></button>` : ''}
    </div>
    ${on('promo') ? `<label for="f-promo">${esc(t('promo'))}</label>
    <div class="promo-row">
      <input id="f-promo" autocomplete="off" autocapitalize="characters" spellcheck="false"
             value="${esc(state.promo ? state.promo.code : '')}">
      <button type="button" class="btn btn-ghost" id="f-promo-go">
        ${esc(state.promo ? t('promoOff') : t('promoApply'))}</button>
    </div>
    <p id="f-promo-out" class="geo" hidden></p>` : ''}
    <div id="f-err"></div>
    ${totalsBlock()}
    <button class="btn mb-2" id="place">${esc(t('place'))}</button>`);
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

  $('#sheetIn').querySelectorAll('[data-tip]').forEach(b => b.onclick = () => {
    state.tip = parseInt(b.dataset.tip, 10) || 0;
    $('#sheetIn').querySelectorAll('[data-tip]').forEach(x => {
      x.classList.toggle('on', x === b);
      x.setAttribute('aria-pressed', String(x === b));
    });
    refreshTotals();
  });

  // Delivery or collection. The address field goes away rather than becoming
  // optional: a field that is sometimes required and sometimes ignored is one
  // people fill in wrongly and one the validator has to guess about.
  $('#sheetIn').querySelectorAll('[data-how]').forEach(b => b.onclick = () => {
    state.how = b.dataset.how;
    $('#sheetIn').querySelectorAll('[data-how]').forEach(x => {
      x.classList.toggle('on', x === b);
      x.setAttribute('aria-pressed', String(x === b));
    });
    const box = $('#addrBox');
    if (box) box.hidden = state.how === 'pickup';
    refreshTotals();
  });

  // THE CODE IS CHECKED BY THE HUB, against a basket the hub prices itself. The
  // storefront sends product ids and quantities and gets back one number. It is
  // a PREVIEW: the order re-checks it under the write lock, so a code on its
  // last use can pass here and still be refused at the end. That is the right
  // way round -- the alternative is giving the same last use away twice.
  const promoBtn = $('#f-promo-go');
  promoBtn.onclick = async () => {
    const out = $('#f-promo-out');
    if (state.promo) {
      state.promo = null; $('#f-promo').value = ''; out.hidden = true;
      promoBtn.textContent = t('promoApply');
      return refreshTotals();
    }
    const code = $('#f-promo').value.trim();
    if (!code) return;
    out.hidden = false; out.className = 'geo'; out.textContent = t('checking');
    promoBtn.disabled = true;
    try {
      const items = cartLines().map(l => ({
        product_id: l.p.id, modifier_ids: l.m, quantity: l.q }));
      const r = await fetch(`${API}/promo/check`, {
        method:'POST', headers:{ 'content-type':'application/json' },
        body: JSON.stringify({ code, items }) });
      const d = await r.json();
      if (!r.ok) throw new Error(d.error || ('HTTP ' + r.status));
      state.promo = { code: d.code, discount: d.discount };
      out.className = 'geo ok';
      // A toast is plain text and so is this line: one number, read once.
      out.textContent = `−${money(d.discount)}`; // money:toast
      promoBtn.textContent = t('promoOff');
      refreshTotals();
    } catch (e) {
      state.promo = null;
      out.className = 'geo bad'; out.textContent = String(e.message || e);
      refreshTotals();
    } finally { promoBtn.disabled = false; }
  };

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
  // THE TELEPHONE NUMBER IS OPTIONAL (operator decision, 2026-09-17). It is a
  // courtesy to the courier, not something the order depends on -- the order
  // has its own id and its own tracking link. A number that IS typed must still
  // be a number, because a half-typed one is worse than none: the courier will
  // try it, and the customer will never know it did not connect.
  if (phone && phone.replace(/\D/g,'').length < 8) errs.push(t('badPhone'));
  const collecting = state.how === 'pickup' && state.loc?.pickup;
  if (!collecting && !addr) errs.push(t('address') + ': ' + t('required'));
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
        // A PICKUP ORDER CARRIES NO ADDRESS. Sending one anyway would put a
        // street on a ticket the courier never sees and the kitchen would read
        // as a delivery.
        fulfilment: collecting
          ? { kind:'pickup', note: note || null }
          : { kind:'delivery',
              address:{ line:addr, note: note || null, ...(state.geo || {}) } },
        payment: pay, locale: lang,
        ...(state.promo ? { promo: state.promo.code } : {}),
        ...(state.tip && !collecting ? { tip: state.tip } : {}),
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
    <p class="muted">#${esc(String(order.id).slice(0,8))} · <span class="money">${money(order.total)}</span></p>
    <div id="pe" class="pe-box"></div>
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

// ── what the customer thought ───────────────────────────────────────────────
//
// A SENTENCE, NOT A SCORE, and not about a person. dowiz does not rank the
// people who work through it; a number on an order becomes a number on whoever
// carried it the moment anybody joins the two. A kitchen can act on "the rice
// was cold" and can do nothing at all with a three.
//
// Only once the order is over, and only once: the box goes away after it is
// sent, because a note the customer can rewrite is one the venue cannot trust
// it read.
function sayBlock(order){
  const over = ['DELIVERED', 'REJECTED', 'CANCELLED'].includes(order.status);
  if (!over || !on('feedback')) return '';
  if (order.feedback) {
    return `<p class="geo ok mb-2">${esc(t('saidIt'))}</p>`;
  }
  return `<label for="f-say">${esc(t('sayHow'))}</label>
    <textarea id="f-say" maxlength="600" rows="2"></textarea>
    <p class="avoid-h">${esc(t('sayHint'))}</p>
    <button class="btn btn-ghost mb-2" id="sayGo">${esc(t('sayGo'))}</button>`;
}

function bindSay(order){
  const go = $('#sayGo'); if (!go) return;
  go.onclick = async () => {
    const text = $('#f-say').value.trim();
    if (!text) return;
    go.disabled = true;
    try {
      const tok = history().find(x => x.id === order.id)?.t;
      const r = await fetch(`${API}/order/${encodeURIComponent(order.id)}/feedback`, {
        method:'POST',
        headers:{ 'content-type':'application/json',
                  ...(tok ? { authorization:'Bearer ' + tok } : {}) },
        body: JSON.stringify({ text }) });
      const d = await r.json();
      if (!r.ok) throw new Error(d.error || ('HTTP ' + r.status));
      openTracking({ ...order, feedback: { text } });
    } catch (e) { go.disabled = false; toast(String(e.message || e)); }
  };
}

// FIXED AMOUNTS, NOT PERCENTAGES. A percentage of a basket is a number the
// customer has to work out to know what they are agreeing to, and it grows with
// the food rather than with the journey -- which is what the courier actually
// did. Three plausible amounts and "none", in the venue's own currency.
//
// It is a DELIVERY control: nobody tips a courier for an order they collect
// themselves, and offering it anyway is asking for money on somebody else's
// behalf who did no work.
const TIPS = [0, 100, 200, 500];

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
    <a class="btn btn-ghost mb-1"
       href="https://t.me/${encodeURIComponent(bot)}?start=${encodeURIComponent(order.id)}"
       target="_blank" rel="noopener noreferrer">
      <i class="ti ti-brand-telegram" aria-hidden="true"></i><span>${esc(t('notify'))}</span></a>
    <p class="muted trk-note">
      ${esc(t('notifyHint'))}</p>` : '';
  sheet(`<h2>${esc(dead ? t('st')[st] : t('sent'))}</h2>
    <p class="muted trk-id">#${esc(String(order.id).slice(0,8))}</p>
    ${dead ? '' : `<div class="track">${FLOW.map((s, n) => `
      <div class="step ${n < i ? 'done' : n === i ? 'now' : ''}">
        <span class="dot">${n < i ? `<i class="ti ti-check" aria-hidden="true"></i>` : ""}</span>
        <span><b>${esc(t('st')[s])}</b></span>
      </div>`).join('')}</div>`}
    <div class="totals"><div class="row grand"><span>${esc(t('total'))}</span>
      <span class="money">${money(order.total ?? order.subtotal ?? 0)}</span></div></div>
    ${follow}
    ${sayBlock(order)}
    <button class="btn btn-ghost mb-2" id="closeTrack">OK</button>`);
  bindSay(order);
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
/// What will actually leave the customer's account.
///
/// A CONVERTED TOTAL MUST NEVER LOOK LIKE THE AMOUNT TAKEN. The venue prices,
/// charges and refunds in its own currency; a euro figure is this page's
/// arithmetic on a reference rate that moved this morning. Showing it without
/// saying so would hand someone a number they cannot reconcile against their
/// bank statement, and the difference would look like the restaurant taking
/// more than it quoted. Empty when the two currencies are the same, so the
/// common case gains no clutter.
function chargeNote(amount){
  const base = baseCurrency();
  if (displayCurrency() === base) return '';
  const exact = Money.format(amount, base, lang === 'uk' ? 'uk' : lang === 'en' ? 'en' : 'sq');
  const stale = MoneyRates && MoneyRates.stale ? ' ⚠' : '';
  return `<div class="row note"><span>${esc(t('chargedIn'))}</span><span class="money">${esc(exact)}${stale}</span></div>`;
}

// ── the currency switcher ───────────────────────────────────────────────────
//
// BUILT FROM THE VENUE'S OWN CURRENCY, not from a fixed list, so a venue
// trading in euros shows EUR first and does not offer to "convert" to itself.
// Rendered only once the venue is known, because before that there is nothing
// to convert from.
function paintCurrencies(){
  const host = document.getElementById('curs');
  if (!host) return;
  const base = baseCurrency();
  const codes = [base, ...Money.CURRENCIES.filter(c => c !== base)];
  host.innerHTML = codes.map(c =>
    `<button class="lang" data-c="${c}" aria-pressed="${c === displayCurrency()}">${c}</button>`).join('');
  host.querySelectorAll('[data-c]').forEach(b => {
    b.onclick = async () => {
      await setDisplayCurrency(b.dataset.c);
      paintCurrencies();
    };
  });
}

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
