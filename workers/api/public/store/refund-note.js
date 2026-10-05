// THE GUEST'S SIDE OF A CARD REFUND (W-REFUND). Once Stripe confirms a
// refund (its webhook writes `succeeded` on the order's `refund.card`
// record), the order page says how much went back and that a bank takes
// days to show it. Nothing is said before Stripe confirms: "on its way" from
// us would be a promise the card network has not made yet.
//
// ASCII QUOTES ONLY in this file. Money through the ONE formatter (state.js).

import { T } from '/store/i18n.js';
import { LANGS } from '../lib/langs.js';
import { moneyEl } from '/store/state.js';

const WORDS = {
  sq: { cardRefunded: 'U kthyen ne karten tuaj', cardRefundDays: 'Mund te duhen 5-10 dite qe te shfaqen ne karte.' },
  en: { cardRefunded: 'Refunded to your card', cardRefundDays: 'It can take 5-10 days to reach your card.' },
  uk: { cardRefunded: 'Повернуто на вашу картку', cardRefundDays: 'Кошти можуть надходити на картку 5-10 днів.' },
  ru: { cardRefunded: 'Возвращено на вашу карту', cardRefundDays: 'Деньги могут поступать на карту 5-10 дней.' },
};
for (const l of LANGS) Object.assign(T[l], WORDS[l]);

/// What Stripe has confirmed back to the card, in the order's units.
export function refundedToCard(order){
  return (order?.refund?.card?.attempts || []).filter(a => a.status === 'succeeded').reduce((s, a) => s + (a.amount | 0), 0);
}

export function cardRefundMarkup(order){
  const n = refundedToCard(order);
  if (!n) return '';
  return `<section class="geo ok mb-2"><p><span data-t="cardRefunded"></span> ${moneyEl(n)}</p>
    <p class="avoid-h" data-t="cardRefundDays"></p></section>`;
}
