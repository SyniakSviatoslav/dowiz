// E-Receipt — Figma node 1:4687 (dark) / 1:15089 (light).
//
// Layout read from the offline document dump (kit/design/extract.py 1:4687):
// the order's lines at 25,126; "Order Details" at 24,568 over a 303-wide grid
// of key/value pairs with a copy control at 296,764; then the same payment
// summary the cart shows, ending in "Total Amount".

import { icon, esc, toast } from '/kit/app.js';
import { topBar, orderLine } from '/kit/parts.js';
import { formatMoney } from '/kit/data.js';
import * as Orders from '/kit/orders.js';
import * as Me from '/kit/me.js';

const RECEIPT = {
  lines: [
    { name: 'ItaliaCrisp Pizza', kind: 'Pizza', variant: '8’ - Small', price: '$12.00',
      addons: ['Olives', 'Capsicum', 'Onion'] },
    { name: 'Mexican Tacos', kind: 'Tacos', variant: '2 Tacos', price: '$22.00',
      addons: ['Extra Cheese', 'Extra Sauce'] },
  ],
  facts: [
    ['Order ID', '#FD785462'],
    ['Customer Name', 'Jennifer Aaker'],
    ['Phone', '+1 (208) 555-0112'],
    ['Promo Code', 'FD45YU2026'],
    ['Payment Methods', 'Wallet'],
    ['Transaction ID', 'TR8574KHTG'],
    ['Order Type', 'Delivery'],
    ['Delivery Type', 'Standard Delivery'],
    ['Order Date', 'April 12, 2026'],
    ['Order Time', '07:30 PM'],
  ],
  summary: [
    ['Sub-Total', '$225.00'],
    ['Delivery Fee | 2.5 Miles', '$20.00'],
    ['Delivery Tip', '$10.00'],
    ['Tax', '$00.00'],
    ['Discount', '-$08.00'],
  ],
  total: '$48.00',
};

/// THE RECEIPT FOR THE ORDER THAT WAS ACTUALLY PLACED.
///
/// The frame's receipt names `Jennifer Aaker`, a promo code, a transaction id
/// and a tip, and it was what every customer got. A receipt is the one document
/// a person keeps to argue with later, so a fabricated one is worse here than
/// anywhere else in the app: it is confident, it is detailed, and every line of
/// it is false.
///
/// What is NOT KNOWN is left out rather than filled in. This order carries no
/// promo, no tip and no card transaction, so those rows do not appear — an empty
/// row is a question, a borrowed one is a lie.
function receiptOf(o){
  if (!o) return null;
  const money = c => o.currency ? formatMoney(c, o.currency) : '';
  const sub = (o.lines || []).reduce((n, l) => n + l.cents * l.qty, 0);
  const total = Number.isInteger(o.total) ? o.total : sub;
  const me = Me.contact();
  const at = new Date(o.placedAtMs);
  const facts = [
    ['Order ID', '#' + String(o.id).slice(0, 8).toUpperCase()],
    me.name  ? ['Customer Name', me.name] : null,
    me.phone ? ['Phone', me.phone] : null,
    ['Payment Methods', 'Cash'],
    ['Order Type', o.address ? 'Delivery' : 'Pickup'],
    o.address ? ['Address', o.address.line] : null,
    ['Order Date', at.toLocaleDateString('uk-UA', { day: 'numeric', month: 'long', year: 'numeric' })],
    ['Order Time', at.toLocaleTimeString('uk-UA', { hour: '2-digit', minute: '2-digit' })],
  ].filter(Boolean);
  return {
    lines: (o.lines || []).map(l => ({ name: l.name, kind: l.kind, variant: l.variant,
                                       price: money(l.cents * l.qty), addons: l.addons })),
    facts,
    // The delivery fee is what the hub charged beyond the dishes. It is derived,
    // not guessed: if the hub's total is the subtotal, delivery was free and the
    // row says so rather than printing the frame's `$20.00`.
    summary: [['Sub-Total', money(sub)],
              [total > sub ? 'Delivery Fee' : 'Delivery',
               total > sub ? money(total - sub) : 'Free']],
    total: money(total),
  };
}

export async function render(params){
  const wanted = params?.get('id');
  const kept = wanted ? Orders.find(wanted) : Orders.last();
  const R = receiptOf(kept) || RECEIPT;
  return `
  ${topBar('E-Receipt', `<button class="k-top-btn" type="button" id="share" data-share
      aria-label="Поділитися чеком">${icon('share-btn')}</button>`)}

  <div class="wrap">
    ${R.lines.map(l => orderLine(l)).join('')}

    <div class="k-block">
      <h2 class="k-block-h">Order Details</h2>
      <div class="k-box">
        <div class="k-facts">
          ${R.facts.map(([k, v]) => `
            <span><span class="k-fact-k">${esc(k)}</span>
                  <span class="k-fact-v">${esc(v)}</span></span>`).join('')}
          <button class="k-copy" type="button" id="copyId"
                  data-copy="${esc(R.facts[0][1])}"
                  aria-label="Скопіювати номер замовлення">${icon('clipboard-text')}</button>
        </div>
      </div>
    </div>

    <div class="k-block">
      <div class="k-box">
        <div class="k-sum-rule"></div>
        <div class="k-sum">
          ${R.summary.map(([k, v]) => `
            <div class="k-sum-row${v.startsWith('-') ? ' is-off' : ''}">
              <span>${esc(k)}</span><span>${esc(v)}</span></div>`).join('')}
        </div>
        <div class="k-sum-rule"></div>
        <div class="k-sum-total"><span>Total Amount</span><span>${esc(R.total)}</span></div>
      </div>
    </div>
  </div>`;
}

export function bind(root){
  root.querySelector('#copyId')?.addEventListener('click', async e => {
    const btn = e.currentTarget;
    // A copy button that silently does nothing is the worst kind: say which
    // way it went, EVERY TIME it is pressed. `navigator.clipboard` is
    // unavailable over plain http and in some embedded webviews, and the
    // earlier version answered the first failed tap by rewriting its own label
    // and every tap after that by doing nothing at all — which is what the
    // interaction gate reported as `DEAD button#copyId`. The line of feedback
    // is the answer, so the order number is on screen to be selected by hand
    // when the clipboard is not available.
    const id = btn.dataset.copy;
    // THE NUMBER IS ON SCREEN BEFORE THE CLIPBOARD IS ASKED. `writeText` can sit
    // unresolved behind a permission decision for longer than a person will
    // wait, and a button whose only answer arrives after that wait is a button
    // they tap again. So the line of feedback carries the number itself first —
    // useful even if the clipboard never answers — and is corrected once it
    // does.
    const said = toast(`Номер замовлення: ${id}`);
    try {
      await navigator.clipboard.writeText(id);
      btn.classList.add('is-done');
      btn.setAttribute('aria-label', 'Скопійовано');
      said.textContent = `Скопійовано: ${id}`;
    } catch {
      btn.setAttribute('aria-label', 'Не вдалося скопіювати — виділіть вручну');
      said.textContent = `Скопіюйте вручну: ${id}`;
    }
  });
}
