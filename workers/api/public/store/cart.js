// The basket: the floating pill, the cart sheet, the totals block, and THE
// WAY THEY GET IT -- delivery, pickup or the hall -- chosen where the fee is
// first drawn (W-SERVICE-MODE, 2026-11-03). The cart used to show "Delivery
// 300" on its first paint with nothing to tap: the choice lived only on the
// checkout, and the hall was not on it at all.
//
// The pill is a `<Money>` that snaps. When a dish is added the pill bounces
// once (a motion), and the number inside it is REPLACED, never counted up --
// a number that counts up is a number that is briefly wrong, and this one is
// what somebody pays.

import { state, API, SLUG, cartLines, cartCount, subtotal, deliveryFee, lineUnit, lineNames, saveCart, moneyEl, money, exactCharge } from '/store/state.js';
import { t, retranslate } from '/store/i18n.js';
import { $, $$, esc, icon, sheet, toast, fallbackArt, paintFallbacks } from '/store/ui.js';
import { quoteEta } from '/store/eta.js';
import { TABLE, tableBanner } from '/store/table.js';
import { bagLine } from '/store/bag.js'; // W-QR: imported here so the bag landing is read on the first paint
import { k, cta, stepper, emptySheet, seg } from '/store/parts.js';
import { offered, hallTables, hallTable, planSlotMin, keep, leavesTheBuilding, DINE_IN } from '/store/how.js';

export function refreshBar(){
  const n = cartCount();
  const pill = $('#cartPill');
  pill.classList.toggle('show', n > 0);
  $('#pillCount').textContent = n;
  const total = $('#pillTotal'); total.dataset.money = String(subtotal()); total.textContent = money(subtotal());
  const badge = $('#navCartN');
  if (badge) {
    const grew = Number(badge.textContent) < n;
    badge.textContent = n; badge.hidden = n === 0;
    if (grew) { badge.classList.remove('pop'); void badge.offsetWidth; badge.classList.add('pop'); }
  }
  document.body.classList.toggle('has-cart', n > 0);
}

/// One bounce, then still. Called after an add.
export function bounceBar(){
  const pill = $('#cartPill');
  pill.classList.remove('bounce'); void pill.offsetWidth; pill.classList.add('bounce');
}

// ── how they get it ─────────────────────────────────────────────────────────
// THE HALL IS OFFERED WHEN THE VENUE HAS A PLAN WITH A TABLE ON IT, asked once
// per visit of the booking's own read (`GET .../tables`), the moment a first
// dish goes in -- so the choice is there when the cart first opens. Until the
// answer comes, the choice shows what is known; when it comes, the block is
// redrawn in place.
const session = () => { try { return globalThis.sessionStorage; } catch { return null; } };
const keepHow = () => { const s = session(); if (s) keep(s, state.how, state.hallTable); };
/// The segment and the table chips, as icon, word, learning anchor.
const HOW_WORDS = { delivery: ['bike', 'toDoor', 'checkout.delivery'], pickup: ['walk', 'toPickup', 'checkout.pickup'], dine_in: ['tools-kitchen-2', 'inHall', null] };
let hallAsked = null;
let hallOpts = { tables: false };
let hallZone = null;
let onHow = null;

export function loadHall(){
  if (hallAsked || !state.loc || TABLE) return hallAsked;
  hallAsked = (async () => {
    try {
      const r = await fetch(`${API}/public/locations/${encodeURIComponent(SLUG)}/tables?slotMin=${planSlotMin(Date.now())}`);
      state.hallPlan = r.ok ? await r.json() : { zones: [] };
    } catch { state.hallPlan = { zones: [] }; }
    // A remembered hall the venue no longer offers is a delivery again.
    const modes = offered(state.loc, state.hallPlan);
    if (!modes.includes(state.how)) { state.how = modes[0]; keepHow(); }
    refreshHow();
  })();
  return hallAsked;
}
addEventListener('dw:cart', () => loadHall());

/// The block: nothing to choose at a signed table, or when only delivery is on.
export function howBlock({ tables = false } = {}){
  hallOpts = { tables };
  const modes = TABLE ? [] : offered(state.loc, state.hallPlan);
  if (modes.length < 2) return '<div id="howBox"></div>';
  return `<div id="howBox"><h3 class="fsec" data-t="how"></h3>
    <div class="seg" role="radiogroup" data-t-attr="aria-label:how">
      ${modes.map(m => seg({ on: m === state.how, icon: HOW_WORDS[m][0], label: k(HOW_WORDS[m][1]), attrs: { data: { how: m } }, tour: HOW_WORDS[m][2] })).join('')}
    </div>${tables ? hallPicker() : ''}</div>`;
}
/// The venue's tables, by zone, the chosen one lit. The zone's name is the
/// venue's own word and is never translated.
function hallPicker(){
  const all = hallTables(state.hallPlan);
  if (!all.length) return '';
  const zones = [...new Map(all.map(x => [x.zone, x.name])).entries()];
  const cur = hallTable(state.hallPlan, state.hallTable);
  const zone = zones.some(([id]) => id === hallZone) ? hallZone : cur ? cur.zone : zones[0][0];
  return `<div id="hallBox" ${state.how === DINE_IN ? '' : 'hidden'}><h3 class="fsec" data-t="tblTable"></h3>
    ${zones.length > 1 ? `<div class="seg" role="radiogroup">${zones.map(([id, name]) => seg({ on: id === zone, label: name, attrs: { data: { hallZone: id } } })).join('')}</div>` : ''}
    <div class="seg" role="radiogroup" data-t-attr="aria-label:tblTable">
      ${all.filter(x => x.zone === zone).map(x => seg({ on: !!cur && cur.zone === x.zone && cur.n === x.n, label: String(x.n), attrs: { data: { hallZone: x.zone, hallN: String(x.n) } } })).join('')}
    </div></div>`;
}
/// Bind the block inside the open sheet. `onChange(how)` is the sheet's own
/// follow-up (the checkout hides its address block); it is kept for a redraw.
export function wireHow(onChange){
  if (onChange) onHow = onChange;
  const root = $('#sheetIn'); if (!root) return;
  for (const b of $$('[data-how]', root)) b.onclick = () => {
    state.how = b.dataset.how; keepHow();
    for (const x of $$('[data-how]', root)) { x.classList.toggle('on', x === b); x.setAttribute('aria-pressed', String(x === b)); }
    const hb = $('#hallBox'); if (hb) hb.hidden = state.how !== DINE_IN;
    refreshTotals();
    if (onHow) onHow(state.how);
  };
  for (const b of $$('[data-hall-n]', root)) b.onclick = () => {
    state.hallTable = { zone: b.dataset.hallZone, n: Number(b.dataset.hallN) }; keepHow();
    for (const x of $$('[data-hall-n]', root)) { x.classList.toggle('on', x === b); x.setAttribute('aria-pressed', String(x === b)); }
  };
  for (const b of $$('[data-hall-zone]:not([data-hall-n])', root)) b.onclick = () => { hallZone = b.dataset.hallZone; refreshHow(); };
}
function refreshHow(){
  const box = document.getElementById('howBox'); if (!box) return;
  box.outerHTML = howBlock(hallOpts);
  const nb = document.getElementById('howBox'); if (nb) retranslate(nb);
  wireHow();
  refreshTotals();
}

/// The totals. An order with no trip has no delivery line at all -- it says
/// "pickup" or "room" where the fee would be, so the absence of a charge is
/// visible rather than a zero the customer has to interpret.
export function totalsBlock(){
  const s = subtotal(), d = deliveryFee(), L = state.loc;
  const trip = !TABLE && leavesTheBuilding(state.how);
  const below = !TABLE && L?.minOrder && s < L.minOrder;   // a round at a table has no minimum
  const cut = state.promo ? state.promo.discount : 0;
  const tip = trip ? (state.tip || 0) : 0;
  const grand = s - cut + d + tip;
  const exact = exactCharge(grand);
  return `<div class="totals" id="totalsBox">
    <div class="row"><span data-t="subtotal"></span>${moneyEl(s)}</div>
    ${cut ? `<div class="row cut"><span><span data-t="discount"></span> · ${esc(state.promo.code)}</span><span>−${moneyEl(cut)}</span></div>` : ''}
    ${TABLE ? '' : trip ? `<div class="row"><span data-t="delivery"></span>${d ? moneyEl(d) : `<span data-t="free"></span>`}</div>`
                 : `<div class="row"><span data-t="${state.how === DINE_IN ? 'inHall' : 'pickup'}"></span><span data-t="free"></span></div>`}
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
  if (!lines.length) return sheet(emptySheet({ icon: 'shopping-bag', title: 'empty', body: 'emptyHint' }), { name: 'cart' });
  loadHall();
  sheet(`
    <p class="eyebrow" data-t="yourOrder"></p>
    <h2 data-t="cart"></h2>
    ${tableBanner()}${bagLine()}
    <div class="clines">${lines.map(l => `<div class="cline">
      <span class="cthumb">${l.p.imageUrl ? `<img src="${esc(l.p.imageUrl)}" alt="" loading="lazy" data-fb="${esc(l.p.name)}">` : fallbackArt(l.p.name)}</span>
      <span class="cmain"><b>${esc(l.p.name)}</b>
        ${lineNames(l.p, l.m).length ? `<small class="muted">${lineNames(l.p, l.m).map(esc).join(' · ')}</small>` : ''}
        <small>${moneyEl(lineUnit(l.p, l.m))}</small></span>
      ${stepper({ value: l.q, minus: { attrs: { data: { m: l.k } } }, plus: { attrs: { data: { a: l.k } } }, tour: 'cart.qty' })}</div>`).join('')}</div>
    <p class="geo" id="cartEta" hidden></p>
    ${howBlock()}
    ${totalsBlock()}
    ${cta({ id: 'toCheckout', cls: 'mb-2', label: k('checkout'), iconEnd: 'chevron-right', tour: 'checkout.open' })}`,
    { name: 'cart', keepScroll: true });
  paintFallbacks($('#sheetIn'));
  const etaLine = () => quoteEta().then(e => { const el = $('#cartEta'); if (el && $('#sheet').dataset.name === 'cart') { el.hidden = !e; if (e) el.innerHTML = `${icon('clock')} ${esc(t(leavesTheBuilding(state.how) ? 'etaRange' : 'ready'))}: <b>${esc(e.text)} ${esc(t('etaMin'))}</b>`; } });
  wireHow(etaLine);
  for (const b of $$('[data-a]', $('#sheetIn'))) b.onclick = () => { const l = state.cart[b.dataset.a]; if (!l) return; l.q++; saveCart(); refreshBar(); openCart(); };
  for (const b of $$('[data-m]', $('#sheetIn'))) b.onclick = () => { const k = b.dataset.m, l = state.cart[k]; if (!l) return; l.q--; if (l.q <= 0) delete state.cart[k]; saveCart(); refreshBar(); openCart(); };
  $('#toCheckout').onclick = async () => (await import('/store/checkout.js')).openCheckout();
  // The estimate for THIS basket, from the kernel. Shown when it answers;
  // shown as nothing when it does not.
  etaLine();
}
export { toast };
