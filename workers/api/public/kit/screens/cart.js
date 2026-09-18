// My Cart — Figma node 1:2937 (Delivery) and 1:3452 (Pickup);
// light set 1:13352 and 1:13825.
//
// The two frames differ in one card: Delivery shows an address and a delivery
// type, Pickup shows a pickup time and no delivery fee. That is a MODE, not a
// second screen, so it is `#/cart?mode=pickup` -- and the totals follow from
// the mode rather than from two hand-kept copies of the same arithmetic.
//
// MONEY IS INTEGER CENTS HERE, as it is in dowiz's kernel (kernel/src/money.rs:
// zero floats, ever). The only float in this file is the one `toFixed` that
// prints the result.

import { icon, esc, go } from '/kit/app.js';
import { plate, topBar, ctaBar, paintPlates } from '/kit/parts.js';
import { eta, menu, venue, formatMoney } from '/kit/data.js';
import { lines as basketLines, setQty as basketSetQty, seedFrame } from '/kit/basket.js';
import * as Me from '/kit/me.js';

// THE CART SHOWS THE BASKET, not a copy of the design's two example rows.
//
// It used to hold those two rows as a constant, so "Add item" on the dish screen
// had nowhere to add TO and this screen could not have shown it if it did. The
// rows now come from `basket.js`, which is the customer's own list, persisted on
// their device.
//
// THE FRAME'S TWO ROWS ARE SEEDED ONLY WHEN THERE IS NO VENUE. They are
// `ItaliaCrisp Pizza` and `Mexican Tacos`: dishes that exist in the Figma frame
// and in no kitchen. Seeding them unconditionally, which is what happened until
// now, put products into a real customer's cart that the hub has never heard of
// — and the hub is right to refuse the order built from them, AFTER the customer
// has pressed Confirm. So this screen asks for them only when it is being looked
// at as a design (no `?s=` and no venue host), which is the only place they mean
// anything.
//
// Read through a function, not into a constant: the basket can change while this
// screen is open, from another tab or from the stepper below.
const LINES = () => basketLines();

const TIPS = [0, 1000, 1200, 1400, 1600, 1800];

const state = {
  // Filled by the kernel's estimate once it answers; the frame's own range
  // stands until then, and stays if there is no venue to ask.
  eta: null,
  mode: 'delivery',
  qty: new Map(LINES().map(l => [l.id, l.qty])),
  tip: 1000,
  keepTip: true,
  coupon: '',
  discountCents: 800,
};

// Delivery is charged by distance in the frame ("Delivery Fee | 2.5 Miles"), and
// pickup is not charged at all -- which is the whole reason the mode changes the
// total rather than only the wording.
const MILES = 2.5;
const FRAME_DELIVERY_CENTS = 1200;

// THE VENUE'S OWN TERMS, once it has answered. Null until then, and on a hub
// that names no venue it stays null and the frame's dollars stand.
let PLACE = null;

/// THE CART IS THE LAST SCREEN THAT MAY GUESS AT MONEY.
///
/// This printed `'$' + cents / 100` for every amount, unconditionally: a venue
/// trading in lek had a 1200-lek line drawn as `$12.00` -- the wrong currency
/// AND a hundredth of the figure, on the one screen a customer reads before
/// paying. `lib/money.js` knows how many minor units a currency has; nothing
/// else is allowed to.
const money = cents => {
  // The negative goes THROUGH the formatter rather than being pasted in front
  // of it: a discount rendered as `-ALL 800` by hand is the sign in the wrong
  // place for the currency, and Intl already knows where each one puts it.
  if (PLACE?.currency) return formatMoney(cents, PLACE.currency);
  return (cents < 0 ? '-$' : '$') + (Math.abs(cents) / 100).toFixed(2);
};

/// What the venue charges to deliver, and what the frame charges when there is
/// no venue. A hard-coded fee on a real hub is a number the customer is quoted
/// and then not charged.
const deliveryCents = () =>
  PLACE && Number.isInteger(PLACE.deliveryFee) ? PLACE.deliveryFee : FRAME_DELIVERY_CENTS;

const subTotal = () =>
  LINES().reduce((n, l) => n + l.cents * (state.qty.get(l.id) || 0), 0);

const deliveryFee = () => {
  if (state.mode === 'pickup') return 0;
  // Free over the venue's own threshold, when it publishes one: a customer who
  // crossed it and is still shown a fee has been told the wrong total.
  const over = PLACE && Number.isInteger(PLACE.freeOver) && subTotal() >= PLACE.freeOver;
  return over ? 0 : deliveryCents();
};
const tipCents = () => (state.mode === 'pickup' ? 0 : state.tip);

const total = () =>
  subTotal() + deliveryFee() + tipCents() - state.discountCents;

// ── Pieces ─────────────────────────────────────────────────────────────────

const lineCard = l => {
  const n = state.qty.get(l.id) || 0;
  return `
  <div class="k-box k-line-card" data-line="${esc(l.id)}">
    <div class="k-line">
      <div class="k-line-img">${plate(l.name)}</div>
      <div class="k-line-body">
        <span class="k-line-name">${esc(l.name)}</span>
        <span class="k-line-sub">${esc(l.kind)}${icon('ellipse3293')}${esc(l.variant)}</span>
        <span class="k-line-price">${money(l.cents)}</span>
      </div>
    </div>
    ${l.addons.length ? `
      <div class="k-addons">
        <p>Add-ons</p>
        <div class="k-tags">${l.addons.map(a => `<span class="k-tag">${esc(a)}</span>`).join('')}</div>
      </div>` : ''}
    <div class="k-line-foot">
      <span class="k-qty k-qty-sm">
        <button type="button" data-line-step="-1" aria-label="Менше">${icon('minus-glyph')}</button>
        <output>${n}</output>
        <button type="button" data-line-step="1" aria-label="Більше">${icon('plus-glyph')}</button>
      </span>
      <button class="k-change" type="button" data-remove>REMOVE</button>
    </div>
  </div>`;
};

const orderTypeCard = () => `
  <div class="k-block">
    <h2 class="k-block-h">Order Type</h2>
    <div class="k-box">
      <div class="k-choice">
        <span class="k-choice-ring">${icon(state.mode === 'pickup' ? 'box' : 'scooter')}</span>
        <span class="k-choice-body">
          <span class="k-choice-t">${state.mode === 'pickup' ? 'Pickup' : 'Delivery'}</span>
          <span class="k-choice-s" id="etaLine">${esc(state.eta || '30–45')} mins${
            icon('ellipse3293')}${
            state.mode === 'pickup' ? 'At the restaurant' : 'To your address'}</span>
        </span>
        <button class="k-change" type="button" data-mode>CHANGE</button>
      </div>
    </div>
  </div>`;

/// WHERE IT IS ACTUALLY GOING.
///
/// This card printed the frame's `245 Madison Ave` whatever the customer had
/// chosen, and the summary two screens later printed the real one -- so the two
/// screens before paying disagreed about the address, and the one that was wrong
/// is the one people read first. An unchosen address says so rather than
/// borrowing New York's.
const addressCard = () => {
  const where = Me.chosen();
  return `
  <div class="k-block">
    <h2 class="k-block-h">Delivery Address</h2>
    <div class="k-box">
      <div class="k-choice">
        <span class="k-choice-ring">${icon('pin-27')}</span>
        <span class="k-choice-body">
          <span class="k-choice-t">${esc(where ? (where.label || 'Address') : 'Not chosen')}</span>
          <span class="k-choice-s">${esc(where ? where.line : 'Choose where this order goes')}</span>
        </span>
        <button class="k-change" type="button" data-go="delivery-address">CHANGE</button>
      </div>
      <p class="k-eta">${icon('timer')}10 minute estimate arrived</p>
    </div>
  </div>`;
};

const deliveryTypeCard = () => `
  <div class="k-block">
    <h2 class="k-block-h">Delivery Type</h2>
    <div class="k-box">
      <div class="k-choice">
        <span class="k-choice-ring">${icon('scooter')}</span>
        <span class="k-choice-body">
          <span class="k-choice-t">Standard Delivery</span>
          <span class="k-choice-s">Delivery in 30–45 mins</span>
        </span>
        <button class="k-change" type="button" data-go="delivery-type">CHANGE</button>
      </div>
    </div>
  </div>`;

const pickupTimeCard = () => `
  <div class="k-block">
    <h2 class="k-block-h">Pickup Time</h2>
    <div class="k-box">
      <div class="k-choice">
        <span class="k-choice-ring">${icon('clock-bold')}</span>
        <span class="k-choice-body">
          <span class="k-choice-t">As soon as possible</span>
          <span class="k-choice-s">Ready in 20–30 mins</span>
        </span>
        <button class="k-change" type="button" data-go="pickup-time">CHANGE</button>
      </div>
    </div>
  </div>`;

const tipCard = () => `
  <div class="k-block">
    <h2 class="k-block-h">Delivery Tip</h2>
    <div class="k-box">
      <div class="k-tips" role="group" aria-label="Чайові">
        ${TIPS.map(c => `
          <button class="k-tip" type="button" data-tip="${c}"
                  aria-pressed="${c === state.tip}">${money(c).replace('.00', '')}</button>`).join('')}
      </div>
      <label class="k-check">
        <input type="checkbox" id="keepTip" ${state.keepTip ? 'checked' : ''}>
        <span class="k-check-box">${icon('check')}</span>
        Keep this tip applied to future orders
      </label>
    </div>
  </div>`;

const summary = () => {
  const rows = [
    ['Sub-Total', subTotal(), false],
    ...(state.mode === 'pickup' ? [] : [[`Delivery Fee | ${MILES} Miles`, deliveryFee(), false]]),
    ...(state.mode === 'pickup' ? [] : [['Delivery Tip', tipCents(), false]]),
    ['Tax', 0, false],
    ['Discount', -state.discountCents, true],
  ];
  return `
  <div class="k-block">
    <h2 class="k-block-h">Payment Summary</h2>
    <div class="k-box">
      <div class="k-sum">
        ${rows.map(([k, v, off]) => `
          <div class="k-sum-row${off ? ' is-off' : ''}">
            <span>${esc(k)}</span><span>${money(v)}</span>
          </div>`).join('')}
        <div class="k-sum-rule"></div>
        <div class="k-sum-total"><span>Total Payment</span>
          <span id="grandTotal">${money(total())}</span></div>
      </div>
    </div>
  </div>`;
};

const body = () => LINES().every(l => !state.qty.get(l.id))
  ? `<div class="k-empty">${icon('linear-shopping-ecommerce-bag4')}
       <p class="t-title">Кошик порожній</p>
       <p class="t-body muted2">Оберіть щось у меню — і воно з’явиться тут.</p>
       <button class="k-submit" type="button" data-go="home">До меню</button>
     </div>`
  : `
    ${LINES().filter(l => state.qty.get(l.id)).map(lineCard).join('')}
    ${orderTypeCard()}
    ${state.mode === 'pickup' ? pickupTimeCard() : addressCard()}
    ${state.mode === 'pickup' ? '' : deliveryTypeCard()}
    <div class="k-block">
      <h2 class="k-block-h">Coupon Code
        <button class="k-seeall" type="button" data-go="coupon">View Coupons</button></h2>
      <div class="k-box">
        <div class="k-coupon">
          <input id="coupon" type="text" placeholder="Add Coupon Code" aria-label="Код купона"
                 value="${esc(state.coupon)}">
          <button class="k-apply" type="button" id="applyCoupon">Apply</button>
        </div>
      </div>
    </div>
    ${state.mode === 'pickup' ? '' : tipCard()}
    <div class="k-block">
      <h2 class="k-block-h">Payment Method</h2>
      <button class="k-pay" type="button" data-go="payment-methods">
        ${icon('wallet')}Wallet<span class="k-pay-chev">${icon('chevron-right')}</span>
      </button>
    </div>
    ${summary()}`;

export async function render(params){
  // The venue's currency and delivery terms, before a single figure is drawn.
  // Asked through `menu()`, which caches per language, so this costs nothing
  // after the first screen.
  PLACE = venue(await menu('uk'));

  // No venue behind this screen: it is the design being read, so it may as well
  // look like the design it was drawn from. With a venue, an empty cart stays
  // empty — see the note at the top of this file.
  if (!PLACE) seedFrame();

  // The screen is entered fresh each time, and the basket may have changed since
  // the last visit — a dish added from the item screen, or a quantity edited in
  // another tab. Rebuilding the map here is what makes this screen a VIEW of the
  // basket rather than a second copy of it that drifts.
  for (const l of LINES()) if (!state.qty.has(l.id)) state.qty.set(l.id, l.qty);
  for (const id of [...state.qty.keys()])
    if (!LINES().some(l => l.id === id)) state.qty.delete(id);
  state.mode = params?.get('mode') === 'pickup' ? 'pickup' : 'delivery';
  const empty = LINES().every(l => !state.qty.get(l.id));
  return `
  ${topBar('My Cart')}
  <div class="wrap" id="cartBody">${body()}</div>
  ${empty ? '' : ctaBar('Proceed to Checkout', { to: 'review-summary' })}`;
}

export function bind(root){
  quoteEta(root);

  root.addEventListener('click', e => {
    const step = e.target.closest('[data-line-step]');
    if (step){
      const id = step.closest('[data-line]').dataset.line;
      bump(id, Number(step.dataset.lineStep), root);
      return;
    }
    const rm = e.target.closest('[data-remove]');
    if (rm){
      const id = rm.closest('[data-line]').dataset.line;
      state.qty.set(id, 0);
      repaint(root);
      return;
    }
    const mode = e.target.closest('[data-mode]');
    if (mode){
      state.mode = state.mode === 'pickup' ? 'delivery' : 'pickup';
      history.replaceState(null, '', `#/cart?mode=${state.mode}`);
      repaint(root);
      return;
    }
    const tip = e.target.closest('[data-tip]');
    if (tip){
      state.tip = Number(tip.dataset.tip);
      for (const b of root.querySelectorAll('[data-tip]'))
        b.setAttribute('aria-pressed', String(Number(b.dataset.tip) === state.tip));
      repaintTotal(root);
      return;
    }
    if (e.target.closest('#applyCoupon')){
      // The coupon's worth is the server's answer, not this screen's. Until it
      // is wired, the field records what was typed and says so plainly.
      state.coupon = root.querySelector('#coupon').value.trim();
      const box = root.querySelector('#applyCoupon');
      box.textContent = state.coupon ? 'Applied' : 'Apply';
    }
  });

  root.addEventListener('change', e => {
    if (e.target.id === 'keepTip') state.keepTip = e.target.checked;
  });
}

function bump(id, by, root){
  const next = Math.max(0, Math.min(99, (state.qty.get(id) || 0) + by));
  state.qty.set(id, next);
  // The basket is the record; this screen is a view of it. Writing back here is
  // what makes the tab-bar count and the next visit agree with what is on screen.
  basketSetQty(id, next);
  repaint(root);
}

// The summary depends on every line, so a changed line repaints the body rather
// than patching six places that each know the arithmetic.
function repaint(root){
  const host = root.querySelector('#cartBody');
  host.innerHTML = body();
  paintPlates(host);
  const bar = root.querySelector('.k-bar');
  if (bar) bar.hidden = LINES().every(l => !state.qty.get(l.id));
}

function repaintTotal(root){
  const el = root.querySelector('#grandTotal');
  if (el) el.textContent = money(total());
}

// Ask the kernel what this basket will actually take. The dishes carry their
// own cooking times, the hub counts the orders already on the pass, and the
// distance is whatever the customer's address gives — so a coffee two streets
// away and a banquet across town stop sharing one published range.
async function quoteEta(root){
  const lines = LINES().filter(l => state.qty.get(l.id))
    .map(l => ({ id: l.id, quantity: state.qty.get(l.id) }));
  if (!lines.length) return;

  let where = {};
  try {
    const saved = sessionStorage.getItem('dw_kit_geo');
    if (saved){
      const [lat, lon] = saved.split(',').map(Number);
      where = { latUdeg: Math.round(lat * 1e6), lonUdeg: Math.round(lon * 1e6) };
    }
  } catch { /* private window */ }

  const answer = await eta.quote({
    items: lines,
    pickup: state.mode === 'pickup',
    ...where,
  });
  if (!answer || answer.error) return;   // the frame's range stands

  state.eta = answer.range;
  const line = root.querySelector('#etaLine');
  if (line) line.textContent = `${answer.range} mins`;
}
