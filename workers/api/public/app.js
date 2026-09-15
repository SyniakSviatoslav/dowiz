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
        cashNote:'Paguani korrierit në dorëzim', cardNote:'Së shpejti', place:'Porosit',
        subtotal:'Nëntotali', delivery:'Dërgesa', free:'Falas', closed:'Mbyllur tani',
        closedHint:'Telefononi për të porositur', soldOut:'S’ka', min:'Porosia minimale',
        sent:'Porosia u dërgua', track:'Ndiqni porosinë', offline:'Jeni offline — telefononi',
        required:'E detyrueshme', badPhone:'Numër i pavlefshëm', ordering:'Duke dërguar…',
        st:{PENDING:'Duke pritur konfirmimin',CONFIRMED:'U konfirmua',PREPARING:'Po gatuhet',
            READY:'Gati',IN_DELIVERY:'Në rrugë',DELIVERED:'U dorëzua',
            REJECTED:'U refuzua',CANCELLED:'U anulua'} },
  en: { cart:'Cart', add:'Add', total:'Total', checkout:'Checkout', empty:'Your cart is empty',
        emptyHint:'Pick a dish from the menu', name:'Name', phone:'Phone', address:'Address',
        note:'Note for the courier', pay:'Payment', cash:'Cash', card:'Card',
        cashNote:'Pay the courier on delivery', cardNote:'Coming soon', place:'Place order',
        subtotal:'Subtotal', delivery:'Delivery', free:'Free', closed:'Closed right now',
        closedHint:'Call to order', soldOut:'Sold out', min:'Minimum order',
        sent:'Order placed', track:'Track your order', offline:'You are offline — call instead',
        required:'Required', badPhone:'Invalid number', ordering:'Sending…',
        st:{PENDING:'Awaiting confirmation',CONFIRMED:'Confirmed',PREPARING:'Being prepared',
            READY:'Ready',IN_DELIVERY:'On the way',DELIVERED:'Delivered',
            REJECTED:'Rejected',CANCELLED:'Cancelled'} },
  uk: { cart:'Кошик', add:'Додати', total:'Разом', checkout:'Оформити', empty:'Кошик порожній',
        emptyHint:'Оберіть страву з меню', name:'Ім’я', phone:'Телефон', address:'Адреса',
        note:'Коментар кур’єру', pay:'Оплата', cash:'Готівка', card:'Картка',
        cashNote:'Оплата кур’єру при отриманні', cardNote:'Незабаром', place:'Замовити',
        subtotal:'Сума', delivery:'Доставка', free:'Безкоштовно', closed:'Зараз зачинено',
        closedHint:'Зателефонуйте, щоб замовити', soldOut:'Немає', min:'Мінімальне замовлення',
        sent:'Замовлення прийнято', track:'Стежити за замовленням', offline:'Немає зв’язку — телефонуйте',
        required:'Обов’язкове поле', badPhone:'Некоректний номер', ordering:'Надсилаємо…',
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

function toast(msg){ const el = $('#toast'); el.textContent = msg; el.classList.add('show');
  clearTimeout(toast._t); toast._t = setTimeout(() => el.classList.remove('show'), 2600); }

// ── data ──
async function load(){
  render(skeleton());
  try {
    const r = await fetch(`${API}/public/locations/${encodeURIComponent(SLUG)}/menu?locale=${lang}`);
    if (!r.ok) throw new Error('HTTP ' + r.status);
    const d = await r.json();
    state.loc = d.location; state.cats = d.categories || [];
    document.title = d.location.name;
    $('#brandName').textContent = d.location.name;
    document.documentElement.lang = lang;
    renderMenu();
  } catch (e) {
    // An honest error naming the real fallback -- the venue's own phone.
    render(`<div class="empty"><b>${esc(t('offline'))}</b>
      ${state.loc?.phone ? `<a class="btn" style="margin-top:14px" href="tel:${esc(state.loc.phone)}">${esc(state.loc.phone)}</a>` : ''}</div>`);
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

function openDish(p){
  sheet(`<h2>${esc(p.name)}</h2>
    ${p.description ? `<p style="color:var(--brand-text-muted);margin:8px 0 4px">${esc(p.description)}</p>` : ''}
    <div class="row"><span class="dish-price">${money(p.price)}</span>
      <span class="qty"><button id="dm" aria-label="−">−</button><span id="dq">1</span><button id="dp" aria-label="+">+</button></span></div>
    <button class="btn" id="dadd" style="margin:14px 0">${esc(t('add'))}</button>`);
  let q = 1;
  $('#dm').onclick = () => { q = Math.max(1, q - 1); $('#dq').textContent = q; };
  $('#dp').onclick = () => { q = Math.min(99, q + 1); $('#dq').textContent = q; };
  $('#dadd').onclick = () => { state.cart[p.id] = (state.cart[p.id] || 0) + q; saveCart(); updateBar(); closeSheet(); toast(`${p.name} · ${q}`); };
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
    <label for="f-note">${esc(t('note'))}</label>
    <input id="f-note">
    <label>${esc(t('pay'))}</label>
    <div class="pays" role="radiogroup">
      <button class="pay" role="radio" aria-checked="true" data-pay="cash">
        <i class="ti ti-cash i" aria-hidden="true"></i><span class="t"><b>${esc(t('cash'))}</b><small>${esc(t('cashNote'))}</small></span></button>
      <button class="pay" role="radio" aria-checked="false" data-pay="card" disabled style="opacity:.5">
        <i class="ti ti-credit-card i" aria-hidden="true"></i><span class="t"><b>${esc(t('card'))}</b><small>${esc(t('cardNote'))}</small></span></button>
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
        fulfilment:{ kind:'delivery', address:{ line:addr, note: note || null } },
        payment: pay, locale: lang })
    });
    const d = await r.json();
    if (!r.ok) throw new Error(d.error || d.message || ('HTTP ' + r.status));
    state.cart = {}; saveCart(); updateBar();
    safeSet('dw_last_order', d.id);
    openTracking(d);
  } catch (e) {
    $('#f-err').innerHTML = `<div class="err">${esc(String(e.message || e))}</div>`;
  } finally {
    state.placing = false;
    const b = $('#place'); if (b) { b.disabled = false; b.textContent = t('place'); }
  }
}

const FLOW = ['PENDING','CONFIRMED','PREPARING','READY','IN_DELIVERY','DELIVERED'];
function openTracking(order){
  const st = order.status, i = FLOW.indexOf(st);
  const dead = st === 'REJECTED' || st === 'CANCELLED';
  sheet(`<h2>${esc(dead ? t('st')[st] : t('sent'))}</h2>
    <p style="color:var(--brand-text-muted);margin:6px 0 2px">#${esc(String(order.id).slice(0,8))}</p>
    ${dead ? '' : `<div class="track">${FLOW.map((s, n) => `
      <div class="step ${n < i ? 'done' : n === i ? 'now' : ''}">
        <span class="dot">${n < i ? `<i class="ti ti-check" aria-hidden="true"></i>` : ""}</span>
        <span><b>${esc(t('st')[s])}</b></span>
      </div>`).join('')}</div>`}
    <div class="totals"><div class="row grand"><span>${esc(t('total'))}</span>
      <span>${money(order.total ?? order.subtotal ?? 0)}</span></div></div>
    <button class="btn btn-ghost" id="closeTrack" style="margin-bottom:12px">OK</button>`);
  $('#closeTrack').onclick = closeSheet;
  clearTimeout(openTracking._t);
  if (!dead && st !== 'DELIVERED') {
    // Poll: the order's state belongs to the server, so ask it rather than
    // guessing locally. A socket comes later; this is honest in the meantime.
    openTracking._t = setTimeout(async () => {
      try { const r = await fetch(`${API}/order/${encodeURIComponent(order.id)}`);
            if (r.ok) { const d = await r.json(); if ($('#sheet').classList.contains('show')) openTracking(d); } }
      catch {}
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
addEventListener('online', netState); addEventListener('offline', netState);
netState(); load();
