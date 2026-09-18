// Review Summary — 1:3990 (an order) and 1:5530 (a table booking);
// light set 1:14383 and 1:15940.
//
// The last screen before money moves: everything chosen so far, each block with
// an "Edit Details" that goes to the screen that OWNS that value. A summary
// whose edit link goes somewhere generic is a summary people stop trusting.

import { icon, esc, toast, go } from '/kit/app.js';
import { topBar, ctaBar, plate } from '/kit/parts.js';
import { menu, venue, formatMoney, orders, orderPayload } from '/kit/data.js';
import { lines as basketLines, subtotalCents, clear as clearBasket } from '/kit/basket.js';
import * as Me from '/kit/me.js';
import * as Orders from '/kit/orders.js';

// ── THE ONE TRANSACTION THIS PRODUCT EXISTS FOR ──
//
// This screen used to end at another design frame: its button carried
// `data-go: 'payment-methods'`, so a customer could fill a basket, read a total
// and press Confirm Order without anything being ordered. Nothing failed, which
// is the worst shape a defect can take on a checkout.
//
// It now builds the order from the customer's own basket and the venue's own
// terms and POSTs it to the same endpoint the storefront has always used. The
// PRICE IS NOT DECIDED HERE and never was: the hub re-derives every line from
// its catalogue, so what this sends is a choice, not an amount.


/// What the venue and the basket actually say, or null when there is no venue
/// and the frame's own numbers stand.
let LIVE = null;

async function liveOrder(){
  const place = venue(await menu('uk'));
  const lines = basketLines();
  if (!place || !lines.length) return null;
  const sub = subtotalCents();
  const fee = Number.isInteger(place.deliveryFee) ? place.deliveryFee : 0;
  const free = Number.isInteger(place.freeOver) && sub >= place.freeOver;
  const delivery = free ? 0 : fee;
  const money = c => formatMoney(c, place.currency);
  const where = Me.chosen();
  return {
    place, lines, sub, delivery, total: sub + delivery, where,
    // Whatever the customer has told this device about themselves -- including
    // what they told the storefront on this same origin, since it is the same
    // shop. An order with no phone is accepted by the hub and is a delivery
    // nobody can ring about, so this is read from one place, not retyped here.
    contact: Me.contact(),
    view: {
      title: 'Review Summary',
      cta: 'Confirm Order',
      blocks: [
        { head: 'Order Type', edit: 'order-type',
          choice: { icon: 'scooter', title: where ? 'Delivery' : 'Pickup',
                    sub: place.eta ? `${place.eta} mins` : '' } },
        { head: 'Delivery Address', edit: 'delivery-address',
          choice: { icon: 'pin-24', title: where ? (where.label || 'Address') : 'Not chosen',
                    sub: where ? where.line : 'Choose an address, or collect it yourself' } },
        { head: 'Your Order', rows: lines.map(l => [`${l.qty} × ${l.name}`, money(l.cents * l.qty)]) },
      ],
      summary: [
        ['Sub-Total', money(sub)],
        [delivery ? 'Delivery Fee' : 'Delivery', delivery ? money(delivery) : 'Free'],
      ],
      total: money(sub + delivery),
    },
  };
}

const ORDER = {
  title: 'Review Summary',
  cta: 'Confirm Order',
  then: 'payment-methods',
  blocks: [
    { head: 'Order Date', rows: [['Date and Time', 'April 12, 2026 | 07:30 PM']] },
    { head: 'Order Type', edit: 'order-type',
      choice: { icon: 'scooter', title: 'Delivery', sub: '30–45 mins · To your address' } },
    { head: 'Delivery Address', edit: 'delivery-address',
      choice: { icon: 'pin-24', title: 'Home',
                sub: '245 Madison Ave, New York, NY 10016, USA' },
      note: '10 minute estimate arrived' },
    { head: 'Delivery Type', edit: 'delivery-type',
      choice: { icon: 'scooter', title: 'Standard Delivery', sub: 'Delivery in 30–45 mins' } },
    { head: 'Payment Method', edit: 'payment-methods',
      choice: { icon: 'wallet', title: 'Wallet', sub: '' } },
  ],
  summary: [
    ['Sub-Total', '$34.00'], ['Delivery Fee | 2.5 Miles', '$12.00'],
    ['Delivery Tip', '$10.00'], ['Tax', '$00.00'], ['Discount', '-$08.00'],
  ],
  total: '$48.00',
};

const BOOKING = {
  title: 'Review Summary',
  cta: 'Confirm Booking',
  then: 'reservation-confirmed',
  venue: { name: 'The Savory Spot', rating: 4.8,
           address: '88 Bedford Street, New York, N...', mins: '10 Min', miles: '2.5 Miles' },
  blocks: [
    { head: 'Customer Details', edit: 'your-profile',
      rows: [['Jennifer Aaker', ''], ['', '+1 (208) 555-0112'], ['', 'example@gmail.com']] },
    { head: 'Booking Date', edit: 'book-a-table',
      rows: [['Date and Time', 'April 15, 2026 | 07:30 PM']] },
    { head: 'Number of Guests', edit: 'book-a-table', rows: [['Guests', '06']] },
    { head: 'Special Occasion', edit: 'book-a-table', rows: [['Occasion', 'Birthday']] },
  ],
};

const block = b => `
  <div class="k-sum-head">
    <h2>${esc(b.head)}</h2>
    ${b.edit ? `<button class="k-edit" type="button" data-go="${esc(b.edit)}">Edit Details</button>`
             : ''}
  </div>
  <div class="k-box">
    ${b.choice ? `
      <div class="k-choice">
        <span class="k-choice-ring">${icon(b.choice.icon)}</span>
        <span class="k-choice-body">
          <span class="k-choice-t">${esc(b.choice.title)}</span>
          ${b.choice.sub ? `<span class="k-choice-s">${esc(b.choice.sub)}</span>` : ''}
        </span>
      </div>
      ${b.note ? `<p class="k-eta">${icon('timer')}${esc(b.note)}</p>` : ''}` : ''}
    ${(b.rows || []).map(([k, v]) => `
      <div class="k-kv">
        ${k ? `<span class="k-kv-k">${esc(k)}</span>` : ''}
        ${v ? `<span class="k-kv-v">${esc(v)}</span>` : ''}
      </div>`).join('')}
  </div>`;

export async function render(params, routeName = 'review-summary'){
  LIVE = routeName === 'booking-summary' ? null : await liveOrder();
  const s = routeName === 'booking-summary' ? BOOKING : (LIVE ? LIVE.view : ORDER);
  return `
  ${topBar(s.title)}
  <div class="wrap k-page">
    ${s.venue ? `
      <div class="k-rrow">
        <div class="k-rrow-img">${plate(s.venue.name)}</div>
        <div class="k-rrow-body">
          <div class="k-rrow-head">
            <span class="k-rrow-name">${esc(s.venue.name)}</span>
            <span class="k-rate">${icon('star2')}${s.venue.rating}</span>
          </div>
          <span class="k-rrow-line">${icon('pin-17')}${esc(s.venue.address)}</span>
          <span class="k-rrow-line">${icon('clock')}${esc(s.venue.mins)}
            ${icon('ellipse3293', 'k-dot')}${esc(s.venue.miles)}</span>
        </div>
      </div>` : ''}

    ${s.blocks.map(block).join('')}

    ${s.summary ? `
      <div class="k-sum-head"><h2>Payment Summary</h2></div>
      <div class="k-box">
        <div class="k-sum">
          ${s.summary.map(([k, v]) => `
            <div class="k-sum-row${v.startsWith('-') ? ' is-off' : ''}">
              <span>${esc(k)}</span><span>${esc(v)}</span></div>`).join('')}
        </div>
        <div class="k-sum-rule"></div>
        <div class="k-sum-total"><span>Total Payment</span><span>${esc(s.total)}</span></div>
      </div>` : ''}
  </div>
  ${LIVE ? ctaBar(s.cta) : ctaBar(s.cta, { to: s.then })}`;
}

export function bind(root){
  // Only the live order takes this path; the frame's button still navigates,
  // because a design review of this screen must still be able to walk through
  // it with no venue behind it.
  if (!LIVE) return;
  const btn = root.querySelector('[data-cta]');
  if (!btn) return;

  btn.addEventListener('click', async () => {
    if (btn.disabled) return;
    const { lines, where, contact, place } = LIVE;

    // Delivery needs somewhere to deliver TO. Rather than refuse, send the
    // customer to the screen that owns that value and say why.
    if (!where){
      toast('Спершу оберіть адресу доставки');
      return go('delivery-address');
    }

    btn.disabled = true;
    const was = btn.textContent;
    btn.textContent = 'Надсилаємо…';

    const answer = await orders.place(orderPayload({
      lines, contact, mode: 'delivery', address: where.line, note: where.note,
      payment: 'cash', locale: place.defaultLocale || 'sq',
    }));

    if (!answer || answer.error){
      // THE HUB'S OWN SENTENCE, not a generic apology. It is the only thing that
      // tells the customer whether to change something or simply try again.
      btn.disabled = false;
      btn.textContent = was;
      toast(answer && answer.error ? String(answer.error) : 'Замовлення не пройшло');
      return;
    }

    // THE KEY COMES BACK EXACTLY ONCE. `answer.access_token` is minted for this
    // order and the hub keeps no copy, so it is written down here, next to the
    // id, before anything else can go wrong. Without it the customer cannot read
    // their own order back — `/api/order/:id` refuses an id on its own.
    //
    // Kept BEFORE the basket is cleared: the lines are what a receipt and a
    // tracking screen name the dishes from, and they are about to be erased.
    Orders.remember(answer, { lines, currency: place.currency,
                              address: where ? { line: where.line, note: where.note } : null });
    clearBasket();
    go('payment-successful');
  });
}
