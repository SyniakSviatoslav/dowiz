// The basket: the floating pill, the cart sheet, the totals block.
//
// The pill is a `<Money>` that snaps. When a dish is added the pill bounces
// once (a motion), and the number inside it is REPLACED, never counted up --
// a number that counts up is a number that is briefly wrong, and this one is
// what somebody pays.

import { state, cartLines, cartCount, subtotal, deliveryFee, lineUnit, lineNames, saveCart, moneyEl, money, exactCharge } from '/store/state.js';
import { t, retranslate } from '/store/i18n.js';
import { $, $$, esc, icon, sheet, toast, fallbackArt, paintFallbacks } from '/store/ui.js';
import { quoteEta } from '/store/eta.js';

export function refreshBar(){
  const n = cartCount();
  const pill = $('#cartPill');
  pill.classList.toggle('show', n > 0);
  $('#pillCount').textContent = n;
  const total = $('#pillTotal'); total.dataset.money = String(subtotal()); total.textContent = money(subtotal());
  const badge = $('#navCartN'); if (badge) { badge.textContent = n; badge.hidden = n === 0; }
  document.body.classList.toggle('has-cart', n > 0);
}

/// One bounce, then still. Called after an add.
export function bounceBar(){
  const pill = $('#cartPill');
  pill.classList.remove('bounce'); void pill.offsetWidth; pill.classList.add('bounce');
}

/// The totals. A collection order has no delivery line at all -- it says
/// "pickup" where the fee would be, so the absence of a charge is visible
/// rather than a zero the customer has to interpret.
export function totalsBlock(){
  const s = subtotal(), d = deliveryFee(), L = state.loc;
  const collecting = state.how === 'pickup' && L?.pickup;
  const below = L?.minOrder && s < L.minOrder;
  const cut = state.promo ? state.promo.discount : 0;
  const tip = collecting ? 0 : (state.tip || 0);
  const grand = s - cut + d + tip;
  const exact = exactCharge(grand);
  return `<div class="totals" id="totalsBox">
    <div class="row"><span data-t="subtotal"></span>${moneyEl(s)}</div>
    ${cut ? `<div class="row cut"><span><span data-t="discount"></span> · ${esc(state.promo.code)}</span><span>−${moneyEl(cut)}</span></div>` : ''}
    ${collecting ? `<div class="row"><span data-t="pickup"></span><span data-t="free"></span></div>`
                 : `<div class="row"><span data-t="delivery"></span>${d ? moneyEl(d) : `<span data-t="free"></span>`}</div>`}
    ${tip ? `<div class="row"><span data-t="tip"></span>${moneyEl(tip)}</div>` : ''}
    <div class="row grand"><span data-t="total"></span>${moneyEl(grand)}</div>
    ${exact ? `<div class="row note"><span data-t="chargedIn"></span><span class="money">${esc(exact)}</span></div>` : ''}
    ${below ? `<div class="err"><span data-t="min"></span>: ${moneyEl(L.minOrder)}</div>` : ''}
  </div>`;
}
export function refreshTotals(){
  const box = document.getElementById('totalsBox');
  if (!box) return;
  box.outerHTML = totalsBlock();
  const nb = document.getElementById('totalsBox'); if (nb) retranslate(nb);
}

export function openCart(){
  const lines = cartLines();
  if (!lines.length) return sheet(`<div class="empty">${icon('shopping-bag', 'ico-lg')}<b data-t="empty"></b><span data-t="emptyHint"></span></div>`, { name: 'cart' });
  sheet(`
    <p class="eyebrow" data-t="yourOrder"></p>
    <h2 data-t="cart"></h2>
    <div class="clines">${lines.map(l => `<div class="cline">
      <span class="cthumb">${l.p.imageUrl ? `<img src="${esc(l.p.imageUrl)}" alt="" loading="lazy" data-fb="${esc(l.p.name)}">` : fallbackArt(l.p.name)}</span>
      <span class="cmain"><b>${esc(l.p.name)}</b>
        ${lineNames(l.p, l.m).length ? `<small class="muted">${lineNames(l.p, l.m).map(esc).join(' · ')}</small>` : ''}
        <small>${moneyEl(lineUnit(l.p, l.m))}</small></span>
      <span class="qty"><button type="button" data-m="${esc(l.k)}" aria-label="−">${icon('minus')}</button>
        <span>${l.q}</span><button type="button" data-a="${esc(l.k)}" aria-label="+">${icon('plus')}</button></span></div>`).join('')}</div>
    <p class="geo" id="cartEta" hidden></p>
    ${totalsBlock()}
    <button class="btn mb-2" id="toCheckout"><span data-t="checkout"></span>${icon('chevron-right')}</button>`,
    { name: 'cart', keepScroll: true });
  paintFallbacks($('#sheetIn'));
  for (const b of $$('[data-a]', $('#sheetIn'))) b.onclick = () => { const l = state.cart[b.dataset.a]; if (!l) return; l.q++; saveCart(); refreshBar(); openCart(); };
  for (const b of $$('[data-m]', $('#sheetIn'))) b.onclick = () => { const k = b.dataset.m, l = state.cart[k]; if (!l) return; l.q--; if (l.q <= 0) delete state.cart[k]; saveCart(); refreshBar(); openCart(); };
  $('#toCheckout').onclick = async () => (await import('/store/checkout.js')).openCheckout();
  // The estimate for THIS basket, from the kernel. Shown when it answers;
  // shown as nothing when it does not.
  quoteEta().then(e => { const el = $('#cartEta'); if (el && e && $('#sheet').dataset.name === 'cart') { el.hidden = false; el.innerHTML = `${icon('clock')} ${esc(t('etaRange'))}: <b>${esc(e.text)} ${esc(t('etaMin'))}</b>`; } });
}
export { toast };
