// Checkout -- the STILLEST moment on the page.
//
// The Sea drops to near-nothing while this sheet is open (sea.js). All
// salience sits on the words and the numbers: who, how, where, when, what it
// costs, one button. Progressive disclosure: the address block appears for
// delivery only, the picker for "later" only, the area check only where the
// venue has drawn one, tips only on delivery, the promo field only where the
// venue runs promotions. A form that shows every field to every customer is a
// form people fill in wrongly.
//
// Everything the old checkout did is here: name, optional phone, delivery or
// collection, saved addresses, the map pin, ASAP or a chosen time, the delivery
// area check, a note, a tip, promo codes checked by the hub, cash or card, and
// placement with the hub pricing the basket itself.

import { state, API, SLUG, cartLines, subtotal, lineUnit, saveCart, addresses, rememberAddress, forgetAddress, moneyEl, money, on, remember } from '/store/state.js';
import { safeGet, safeSet } from '/store/storage.js';
import { t, lang } from '/store/i18n.js';
import { $, $$, esc, icon, sheet, closeSheet, toast, whenSheetCloses } from '/store/ui.js';
import { totalsBlock, refreshTotals, refreshBar } from '/store/cart.js';
import { quoteEta } from '/store/eta.js';
import { seaCalm, seaEvent } from '/store/sea.js';

const TIPS = [0, 100, 200, 500];

function savedAddressMarkup(){
  const list = addresses();
  if (!list.length) return '';
  const cur = (safeGet('dw_addr') || '').replace(/\s+/g, ' ').trim().toLowerCase();
  return `<div class="addrs" role="radiogroup" data-t-attr="aria-label:savedAddresses">
    ${list.map(a => { const onn = a.line.replace(/\s+/g, ' ').trim().toLowerCase() === cur; return `<span class="addr-chip">
      <button type="button" class="chip ${onn ? 'on' : ''}" role="radio" aria-checked="${onn}"
        data-addr="${esc(a.line)}" data-lat="${a.lat ?? ''}" data-lng="${a.lng ?? ''}">${icon('map-pin')}<span>${esc(a.line.length > 34 ? a.line.slice(0, 33) + '…' : a.line)}</span></button>
      <button type="button" class="addr-x" data-addr-del="${esc(a.line)}" aria-label="${esc(t('forget'))}">${icon('x')}</button></span>`; }).join('')}
  </div>`;
}

export function openCheckout(){
  const s = subtotal(), L = state.loc;
  if (L?.minOrder && s < L.minOrder) return toast(`${t('min')}: ${money(L.minOrder)}`);
  seaCalm(true);
  whenSheetCloses(() => seaCalm(false));
  const pickup = state.how === 'pickup' && L?.pickup;
  sheet(`
    <p class="eyebrow" data-t="checkout"></p>
    <h2 data-t="summary"></h2>
    <p class="geo" id="ckEta" hidden></p>

    <h3 class="fsec" data-t="contact"></h3>
    <label for="f-name" data-t="name"></label>
    <input id="f-name" autocomplete="name" value="${esc(safeGet('dw_name') || '')}">
    <label for="f-phone"><span data-t="phone"></span> <span class="opt" data-t="optional"></span></label>
    <input id="f-phone" type="tel" inputmode="tel" autocomplete="tel" placeholder="+355…" value="${esc(safeGet('dw_phone') || '')}">

    ${L?.pickup ? `<h3 class="fsec" data-t="how"></h3>
    <div class="seg" role="radiogroup" data-t-attr="aria-label:how">
      <button type="button" class="seg-b ${!pickup ? 'on' : ''}" data-how="delivery" aria-pressed="${!pickup}">${icon('bike')}<span data-t="toDoor"></span></button>
      <button type="button" class="seg-b ${pickup ? 'on' : ''}" data-how="pickup" aria-pressed="${pickup}">${icon('shopping-bag')}<span data-t="toPickup"></span></button>
    </div>` : ''}

    <div id="addrBox" ${pickup ? 'hidden' : ''}>
      <h3 class="fsec" data-t="address"></h3>
      ${savedAddressMarkup()}
      <button type="button" class="btn btn-ghost mb-2" id="pinGo">${icon('gps')}<span data-t="pickOnMap"></span></button>
      <textarea id="f-addr" autocomplete="street-address" data-t-attr="placeholder:address">${esc(safeGet('dw_addr') || '')}</textarea>
      <p class="geo ok" id="pinLine" ${state.pin ? '' : 'hidden'}>${icon('map-pin-check')}<span data-t="confirmPin"></span></p>
      ${L?.hasDeliveryZones ? `<button type="button" class="btn btn-ghost mb-1" id="f-geo">${icon('map-pin-check')}<span data-t="checkArea"></span></button>
        <p id="f-geo-out" class="geo" hidden></p>` : ''}
    </div>
    ${pickup && L?.address ? `<p class="geo ok"><span data-t="pickupAt"></span>: ${esc(L.address)}</p>` : ''}

    <h3 class="fsec" data-t="when"></h3>
    <div class="seg" role="radiogroup">
      <button type="button" class="seg-b on" data-when="asap" aria-pressed="true">${icon('clock')}<span data-t="asap"></span></button>
      <button type="button" class="seg-b" data-when="later" aria-pressed="false">${icon('history')}<span data-t="later"></span></button>
    </div>
    <input type="datetime-local" id="f-when" hidden>

    <h3 class="fsec" data-t="extras"></h3>
    <label for="f-note" data-t="note"></label>
    <input id="f-note">
    ${!pickup && on('tips') ? `<label data-t="tip"></label>
    <div class="seg" role="radiogroup" data-t-attr="aria-label:tip">
      ${TIPS.map(v => `<button type="button" class="seg-b ${state.tip === v ? 'on' : ''}" data-tip="${v}" aria-pressed="${state.tip === v}">${v ? moneyEl(v) : `<span data-t="tipNo"></span>`}</button>`).join('')}
    </div>` : ''}
    ${on('promo') ? `<label for="f-promo" data-t="promo"></label>
    <div class="promo-row">
      <input id="f-promo" autocomplete="off" autocapitalize="characters" spellcheck="false" value="${esc(state.promo ? state.promo.code : '')}">
      <button type="button" class="btn btn-ghost" id="f-promo-go" data-t="${state.promo ? 'promoOff' : 'promoApply'}"></button>
    </div>
    <p id="f-promo-out" class="geo" hidden></p>` : ''}

    <h3 class="fsec" data-t="pay"></h3>
    <div class="pays" role="radiogroup">
      <button type="button" class="pay" role="radio" aria-checked="true" data-pay="cash">${icon('cash')}<span class="t"><b data-t="cash"></b><small data-t="cashNote"></small></span></button>
      ${L?.stripePublishableKey ? `<button type="button" class="pay" role="radio" aria-checked="false" data-pay="card">${icon('credit-card')}<span class="t"><b data-t="card"></b><small data-t="cardNote"></small></span></button>` : ''}
    </div>

    <div id="f-err"></div>
    ${totalsBlock()}
    <button class="btn mb-2" id="place"><span data-t="place"></span>${icon('chevron-right')}</button>`,
    { name: 'checkout' });

  let pay = 'cash';
  for (const b of $$('[data-pay]', $('#sheetIn'))) b.onclick = () => { for (const x of $$('[data-pay]', $('#sheetIn'))) x.setAttribute('aria-checked', String(x === b)); pay = b.dataset.pay; };
  $('#place').onclick = () => place(pay);

  const etaLine = async () => {
    const e = await quoteEta({ pickup: state.how === 'pickup' && L?.pickup });
    const el = $('#ckEta'); if (!el || $('#sheet').dataset.name !== 'checkout') return;
    el.hidden = !e;
    if (e) el.innerHTML = `${icon('clock')} ${esc(t('etaRange'))}: <b>${esc(e.text)} ${esc(t('etaMin'))}</b>`;
  };
  etaLine();

  // delivery / collection
  for (const b of $$('[data-how]', $('#sheetIn'))) b.onclick = () => {
    state.how = b.dataset.how;
    for (const x of $$('[data-how]', $('#sheetIn'))) { x.classList.toggle('on', x === b); x.setAttribute('aria-pressed', String(x === b)); }
    const box = $('#addrBox'); if (box) box.hidden = state.how === 'pickup';
    refreshTotals(); etaLine();
  };

  // saved addresses: pick fills the field (and the pin, when it had one)
  const addrBox = $('#addrBox');
  if (addrBox) {
    for (const b of $$('[data-addr]', addrBox)) b.onclick = () => {
      const f = $('#f-addr'); f.value = b.dataset.addr;
      for (const x of $$('[data-addr]', addrBox)) { x.classList.toggle('on', x === b); x.setAttribute('aria-checked', String(x === b)); }
      const la = Number(b.dataset.lat), ln = Number(b.dataset.lng);
      state.pin = Number.isFinite(la) && Number.isFinite(ln) && b.dataset.lat !== '' ? { lat: la, lng: ln } : null;
      $('#pinLine').hidden = !state.pin;
      f.focus(); try { f.setSelectionRange(f.value.length, f.value.length); } catch {}
      etaLine();
    };
    for (const b of $$('[data-addr-del]', addrBox)) b.onclick = () => {
      forgetAddress(b.dataset.addrDel);
      b.closest('.addr-chip')?.remove();
      if (!addrBox.querySelectorAll('.addr-chip').length) addrBox.querySelector('.addrs')?.remove();
    };
    // the map
    $('#pinGo').onclick = async () => {
      const cur = $('#f-addr').value.trim();
      const { pickOnMap } = await import('/store/address.js');
      const r = await pickOnMap({ initial: state.pin ? { ...state.pin, line: cur } : null });
      openCheckout();   // the checkout re-opens with what was chosen
      if (r) {
        state.pin = { lat: r.lat, lng: r.lng };
        const f = $('#f-addr');
        if (r.line && (!f.value.trim() || confirmReplace(f.value.trim(), r.line))) f.value = r.line;
        $('#pinLine').hidden = false;
        etaLine();
      }
    };
  }
  // A typed line the customer already has is not overwritten by a geocode
  // silently: if it starts with the same street it is kept, else replaced.
  const confirmReplace = (typed, found) => !typed || !found.split(',')[0] || !typed.toLowerCase().startsWith(found.split(',')[0].toLowerCase().slice(0, 6));

  // when
  for (const b of $$('[data-when]', $('#sheetIn'))) b.onclick = () => {
    const when = b.dataset.when;
    for (const x of $$('[data-when]', $('#sheetIn'))) { x.classList.toggle('on', x === b); x.setAttribute('aria-pressed', String(x === b)); }
    const f = $('#f-when'); f.hidden = when !== 'later';
    if (when === 'later' && !f.value) {
      const d = new Date(Date.now() + 60 * 60 * 1000); d.setMinutes(d.getMinutes() > 30 ? 60 : 30, 0, 0);
      const pad = n => String(n).padStart(2, '0');
      f.value = `${d.getFullYear()}-${pad(d.getMonth()+1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`; f.min = f.value;
    }
    if (when === 'later') f.focus();
  };

  // tip
  for (const b of $$('[data-tip]', $('#sheetIn'))) b.onclick = () => {
    state.tip = parseInt(b.dataset.tip, 10) || 0;
    for (const x of $$('[data-tip]', $('#sheetIn'))) { x.classList.toggle('on', x === b); x.setAttribute('aria-pressed', String(x === b)); }
    refreshTotals();
  };

  // promo -- checked by the hub against a basket the hub prices itself
  const promoBtn = $('#f-promo-go');
  if (promoBtn) promoBtn.onclick = async () => {
    const out = $('#f-promo-out');
    if (state.promo) { state.promo = null; $('#f-promo').value = ''; out.hidden = true; promoBtn.textContent = t('promoApply'); return refreshTotals(); }
    const code = $('#f-promo').value.trim(); if (!code) return;
    out.hidden = false; out.className = 'geo'; out.textContent = t('checking'); promoBtn.disabled = true;
    try {
      const items = cartLines().map(l => ({ product_id: l.p.id, modifier_ids: l.m, quantity: l.q }));
      const r = await fetch(`${API}/promo/check`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ code, items }) });
      const d = await r.json(); if (!r.ok) throw new Error(d.error || ('HTTP ' + r.status));
      state.promo = { code: d.code, discount: d.discount };
      out.className = 'geo ok'; out.textContent = `−${money(d.discount)}`; promoBtn.textContent = t('promoOff'); refreshTotals();
    } catch (e) { state.promo = null; out.className = 'geo bad'; out.textContent = String(e.message || e); refreshTotals(); }
    finally { promoBtn.disabled = false; }
  };

  // the delivery-area check, only where the venue drew one
  const geo = $('#f-geo');
  if (geo) geo.onclick = () => {
    const out = $('#f-geo-out'); out.hidden = false; out.className = 'geo'; out.textContent = t('checking');
    const check = async (lat, lng) => {
      state.geo = { lat_udeg: Math.round(lat * 1e6), lon_udeg: Math.round(lng * 1e6) };
      try {
        const r = await fetch(`${API}/public/reach?lat_udeg=${state.geo.lat_udeg}&lon_udeg=${state.geo.lon_udeg}`);
        const d = await r.json();
        if (d.deliverable) { out.className = 'geo ok'; out.textContent = t('inArea'); }
        else { out.className = 'geo bad'; const km = (d.nearestMetres || 0) / 1000; out.textContent = t('outArea') + (km >= 0.1 ? ` · ~${km.toFixed(1)} km` : ''); }
      } catch { out.className = 'geo bad'; out.textContent = t('noGeo'); }
    };
    if (state.pin) return check(state.pin.lat, state.pin.lng);
    if (!navigator.geolocation) { out.className = 'geo bad'; out.textContent = t('noGeo'); return; }
    navigator.geolocation.getCurrentPosition(pos => check(pos.coords.latitude, pos.coords.longitude),
      () => { state.geo = null; out.className = 'geo'; out.textContent = t('noGeo'); },
      { enableHighAccuracy: true, timeout: 10000, maximumAge: 60000 });
  };
}

function scheduledAt(){
  const f = document.getElementById('f-when');
  if (!f || f.hidden || !f.value) return null;
  const ms = new Date(f.value).getTime();
  return Number.isFinite(ms) ? ms : null;
}

async function place(pay){
  if (state.placing) return;
  const name = $('#f-name').value.trim(), phone = $('#f-phone').value.trim(),
        addr = $('#f-addr')?.value.trim() || '', note = $('#f-note').value.trim();
  const errs = [];
  if (phone && phone.replace(/\D/g, '').length < 8) errs.push(t('badPhone'));
  const collecting = state.how === 'pickup' && state.loc?.pickup;
  if (!collecting && !addr) errs.push(t('address') + ': ' + t('required'));
  $('#f-err').innerHTML = errs.map(e => `<div class="err">${esc(e)}</div>`).join('');
  if (errs.length) { $('#f-err').scrollIntoView({ block: 'center', behavior: 'smooth' }); return; }

  safeSet('dw_name', name); safeSet('dw_phone', phone); safeSet('dw_addr', addr);
  if (!collecting) rememberAddress({ line: addr, lat: state.pin?.lat ?? null, lng: state.pin?.lng ?? null });
  state.placing = true; $('#place').disabled = true; $('#place').textContent = t('ordering');
  const geo = state.pin ? { lat_udeg: Math.round(state.pin.lat * 1e6), lon_udeg: Math.round(state.pin.lng * 1e6) } : (state.geo || {});
  const eta = state.lastEta;
  try {
    const items = cartLines().map(l => ({ product_id: l.p.id, modifier_ids: l.m, quantity: l.q, unit_price: lineUnit(l.p, l.m) }));
    const r = await fetch(`${API}/public/locations/${encodeURIComponent(SLUG)}/orders`, {
      method: 'POST', headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ items, contact: { name, phone },
        fulfilment: collecting ? { kind: 'pickup', note: note || null }
                               : { kind: 'delivery', address: { line: addr, note: note || null, ...geo } },
        payment: pay, locale: lang,
        ...(state.promo ? { promo: state.promo.code } : {}),
        ...(state.tip && !collecting ? { tip: state.tip } : {}),
        ...(scheduledAt() ? { scheduled_for_ms: scheduledAt() } : {}) })
    });
    const d = await r.json();
    if (!r.ok) throw new Error(d.error || d.message || ('HTTP ' + r.status));
    state.cart = {}; saveCart(); refreshBar();
    safeSet('dw_last_order', d.id);
    remember(d);
    dispatchEvent(new Event('dw:ordered'));
    if (d.payment_error) toast(String(d.payment_error));
    const { openTracking } = await import('/store/track.js');
    if (d.client_secret) return collectCard(d, openTracking);
    seaCalm(false);
    seaEvent('order_created', 160);
    openTracking({ ...d, eta });
  } catch (e) {
    $('#f-err').innerHTML = `<div class="err">${esc(String(e.message || e))}</div>`;
  } finally {
    state.placing = false;
    const b = $('#place'); if (b) { b.disabled = false; b.innerHTML = `<span data-t="place">${esc(t('place'))}</span>${icon('chevron-right')}`; }
  }
}

// ── card ── Stripe.js on demand; the card goes from the browser to Stripe.
let stripeLib = null;
async function loadStripe(){
  if (stripeLib) return stripeLib;
  await new Promise((ok, no) => { const el = document.createElement('script'); el.src = 'https://js.stripe.com/v3/'; el.onload = ok; el.onerror = no; document.head.appendChild(el); });
  stripeLib = window.Stripe(state.loc.stripePublishableKey);
  return stripeLib;
}
async function collectCard(order, openTracking){
  sheet(`<p class="eyebrow" data-t="pay"></p><h2>#${esc(String(order.id).slice(0, 8))}</h2>
    <p class="muted">${moneyEl(order.total)}</p>
    <div id="pe" class="pe-box"></div><div id="pe-err"></div>
    <button class="btn" id="pay" data-t="place"></button>`, { name: 'pay' });
  let stripe, elements;
  try {
    stripe = await loadStripe();
    elements = stripe.elements({ clientSecret: order.client_secret });
    elements.create('payment', { layout: 'tabs' }).mount('#pe');
  } catch { $('#pe').innerHTML = `<div class="err">${esc(t('offline'))}</div>`; return; }
  $('#pay').onclick = async () => {
    const b = $('#pay'); b.disabled = true; b.textContent = t('ordering');
    const { error } = await stripe.confirmPayment({ elements,
      confirmParams: { return_url: location.origin + '/?order=' + encodeURIComponent(order.id) }, redirect: 'if_required' });
    if (error) { $('#pe-err').innerHTML = `<div class="err">${esc(error.message || '')}</div>`; b.disabled = false; b.textContent = t('place'); return; }
    seaCalm(false);
    seaEvent('order_created', 160);
    openTracking(order);
  };
}
