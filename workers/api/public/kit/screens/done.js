// Outcome screens — Payment Successful (1:4231), Reservation Confirmed!
// (1:4276) and Top Up Successful! (1:9927); light set 1:14629, 1:14675, 1:20391.
//
// Three frames, one layout: a 126px badge, a 22px headline, one sentence and a
// stack of actions whose second is a ghost. They differ only in those strings,
// so they are one module keyed by the route.

import { icon, esc } from '/kit/app.js';

const OUTCOMES = {
  'payment-successful': {
    head: 'Payment Successful!',
    body: 'Order placed successfully!',
    actions: [
      { label: 'View Order', to: 'track-order' },
      { label: 'View E-Receipt', to: 'e-receipt', ghost: true },
    ],
  },
  'reservation-confirmed': {
    head: 'Reservation Confirmed!',
    body: 'Your table is booked. We’ve sent the details to your email.',
    actions: [
      { label: 'View Booking', to: 'my-booking' },
      { label: 'Back to Home', to: 'home', ghost: true },
    ],
  },
  'top-up-successful': {
    head: 'Top Up Successful!',
    body: 'Your wallet has been topped up.',
    actions: [
      { label: 'View Wallet', to: 'my-wallet' },
      { label: 'Back to Home', to: 'home', ghost: true },
    ],
  },
};

export function render(params, routeName = 'payment-successful'){
  const o = OUTCOMES[routeName] || OUTCOMES['payment-successful'];
  return `
  <button class="k-top-btn k-done-back" type="button" data-back aria-label="Назад">
    ${icon('arrow-left')}
  </button>

  <div class="k-done">
    <span class="k-done-badge" aria-hidden="true">${icon('verify-badge')}</span>
    <h1>${esc(o.head)}</h1>
    <p>${esc(o.body)}</p>
  </div>

  <div class="k-bar k-bar-stack">
    ${o.actions.map(a => `
      <button class="k-cta${a.ghost ? ' k-cta-ghost' : ''}" type="button"
              data-go="${esc(a.to)}">${esc(a.label)}</button>`).join('')}
  </div>`;
}
