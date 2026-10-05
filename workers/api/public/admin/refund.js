// THE WAY AN ORDER PAST PENDING ENDS (2026-09-24). The order machine allows
// CANCELLED only from PENDING; from CONFIRMED, PREPARING, READY and
// IN_DELIVERY the one exit is REFUNDING (crates/dowiz-core order_machine.rs
// `allowed_next`). The console offered "cancel" on all of them and every tap
// answered 409 "Illegal transition" -- a Wolt order the kitchen had started
// could not be ended from any screen. This sheet is that exit:
// `POST /api/staff/orders/:id/refund` `{reason}` starts it (an order that
// took no money is complete in the same turn), and `{complete: true}` records
// the money handed back. Each refund is a signed row in the exception report.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.

import { $, esc, t, S, api, withLoc, toast, sheet, busy, money } from '/admin/core.js';
import { T, LANGS } from '/admin/i18n.js';
import { btn, field, select } from '/admin/parts.js';

const WORDS = {
  sq: { refund: 'Rimburso', refundHint: 'Porosia mbyllet si e rimbursuar dhe del te Perjashtimet me emrin tuaj.', refundReason: 'Arsyeja', rr_venue_cancelled: 'Lokali e anuloi', rr_customer_request: 'Me kerkese te klientit', rr_payment_error: 'Gabim pagese', rr_other: 'Tjeter', refundNote: 'Shenim (per tjeter)', moneyBack: 'Parate u kthyen', refundStarted: 'Rimbursimi filloi', refundDone: 'Rimbursimi u mbyll', cardPart: 'Me karte', cashPart: 'Me para ne dore', refundedAlready: 'Te kthyera tashme', cardAmount: 'Kthehet ne karte', cardAmountHint: 'Parazgjedhur: gjithcka qe mbetet ne karte.', cardPending: 'Rimbursimi ne pritje te Stripe', cardDone: 'U kthye ne karte', cardFailed: 'Deshtoi', tryAgain: 'Provo perseri', cardMore: 'Kthe ne karte', cardManual: 'Rimbursimi ne karte kerkon Stripe te lidhur: kthejeni karten me dore ne Stripe, pastaj shenojeni si te rimbursuar.', cardSent: 'Rimbursimi u dergua ne Stripe' },
  en: { refund: 'Refund', refundHint: 'The order ends as refunded and is listed under Exceptions with your name.', refundReason: 'Reason', rr_venue_cancelled: 'The venue cancelled', rr_customer_request: 'The customer asked', rr_payment_error: 'Payment error', rr_other: 'Other', refundNote: 'Note (for other)', moneyBack: 'Money handed back', refundStarted: 'Refund started', refundDone: 'Refund complete', cardPart: 'By card', cashPart: 'In cash', refundedAlready: 'Already refunded', cardAmount: 'Back to the card', cardAmountHint: 'Default: everything still on the card.', cardPending: 'Refund pending at Stripe', cardDone: 'Refunded to the card', cardFailed: 'Failed', tryAgain: 'Try again', cardMore: 'Refund to card', cardManual: 'Card refund needs Stripe connected: refund the card by hand in Stripe, then mark it refunded.', cardSent: 'Refund sent to Stripe' },
  uk: { refund: 'Повернення', refundHint: 'Замовлення завершиться як повернене і з\'явиться у Винятках з вашим ім\'ям.', refundReason: 'Причина', rr_venue_cancelled: 'Заклад скасував', rr_customer_request: 'На прохання клієнта', rr_payment_error: 'Помилка оплати', rr_other: 'Інше', refundNote: 'Примітка (для іншого)', moneyBack: 'Гроші повернуто', refundStarted: 'Повернення розпочато', refundDone: 'Повернення завершено', cardPart: 'Карткою', cashPart: 'Готівкою', refundedAlready: 'Вже повернуто', cardAmount: 'Повернути на картку', cardAmountHint: 'За замовчуванням: усе, що ще на картці.', cardPending: 'Повернення очікує в Stripe', cardDone: 'Повернуто на картку', cardFailed: 'Не вдалося', tryAgain: 'Спробувати ще', cardMore: 'Повернути на картку', cardManual: 'Повернення на картку потребує підключеного Stripe: поверніть кошти вручну в Stripe, потім позначте як повернене.', cardSent: 'Повернення надіслано в Stripe' },
  ru: { refund: 'Возврат', refundHint: 'Заказ завершится как возвращённый и появится в Исключениях с вашим именем.', refundReason: 'Причина', rr_venue_cancelled: 'Заведение отменило', rr_customer_request: 'По просьбе клиента', rr_payment_error: 'Ошибка оплаты', rr_other: 'Другое', refundNote: 'Примечание (для другого)', moneyBack: 'Деньги возвращены', refundStarted: 'Возврат начат', refundDone: 'Возврат завершён', cardPart: 'Картой', cashPart: 'Наличными', refundedAlready: 'Уже возвращено', cardAmount: 'Вернуть на карту', cardAmountHint: 'По умолчанию: всё, что ещё на карте.', cardPending: 'Возврат ожидает в Stripe', cardDone: 'Возвращено на карту', cardFailed: 'Не удалось', tryAgain: 'Попробовать снова', cardMore: 'Вернуть на карту', cardManual: 'Возврат на карту требует подключённого Stripe: верните деньги вручную в Stripe, затем отметьте как возвращённый.', cardSent: 'Возврат отправлен в Stripe' },
};
for (const l of LANGS) Object.assign(T[l], WORDS[l]);

/// `command::refund::RefundReason`, the words the hub parses. `refused_at_door`
/// is the courier's, never the console's.
const REASONS = ['venue_cancelled', 'customer_request', 'payment_error', 'other'];
/// Where the refund door is open (`allowed_next` into REFUNDING).
export const REFUNDABLE = new Set(['CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY']);

const key = id => (globalThis.crypto?.randomUUID?.() || String(Date.now())) + '-' + id;
const send = (id, body, k) => api(`/staff/orders/${encodeURIComponent(id)}/refund`, { method: 'POST', body: withLoc(body), headers: { 'idempotency-key': k } });

/// THE CARD'S SHARE (W-REFUND), read off the order's own evidence the way the
/// hub reads it (`command/refund/card.rs`): what the card paid is what the
/// Stripe webhook RECEIVED; what went back is every attempt Stripe called
/// `succeeded`; what is in flight is queued/pending. The hub decides; this
/// only draws, and its numbers are integers in the venue's units.
const IN_FLIGHT = new Set(['queued', 'pending', 'requires_action']);
export function cardOf(o){
  const paid = o?.payment_intent && o.amount_received > 0 ? o.amount_received : 0;
  const tries = o?.refund?.card?.attempts || [];
  const sum = f => tries.filter(f).reduce((a, x) => a + (x.amount | 0), 0);
  const done = sum(x => x.status === 'succeeded'), flying = sum(x => IN_FLIGHT.has(x.status));
  const owed = o?.refund?.owed ?? ((o?.payments || []).reduce((a, p) => a + (p.amount | 0), 0) + (o?.cash_collected | 0) + paid);
  return { paid, done, flying, left: Math.max(0, paid - done - flying), cash: Math.max(0, owed - paid),
           last: tries[tries.length - 1] || null, manual: !!o?.refund?.card?.manual };
}

function partsHtml(c){
  return `<div class="line"><span class="n" data-t="cardPart"></span>${esc(money(c.paid))}</div>
    <div class="line"><span class="n" data-t="cashPart"></span>${esc(money(c.cash))}</div>
    <div class="line"><span class="n" data-t="refundedAlready"></span>${esc(money(c.done))}</div>`;
}

/// The reason sheet; resolves once the hub answered (or the owner went back).
export function openRefund(id, after){
  let k = key(id);
  const c = cardOf(S.orders.find(x => x.id === id));
  sheet(`<p class="eyebrow">#${esc(id.slice(0, 8))}</p><h2 data-t="refund"></h2><p class="muted small" data-t="refundHint"></p>
    ${c.paid ? partsHtml(c) : ''}
    ${c.left ? field({ id: 'rf-amt', key: 'cardAmount', money: true, value: c.left, hintKey: 'cardAmountHint', autocomplete: 'off', tour: 'refund.amount' }) : ''}
    ${select({ id: 'rf-why', key: 'refundReason', options: REASONS.map(r => ({ value: r, key: 'rr_' + r })), tour: 'refund.reason' })}
    ${field({ id: 'rf-note', key: 'refundNote', maxlength: 140, autocomplete: 'off', tour: 'refund.note' })}
    <div class="btn-row">${btn({ id: 'rfGo', variant: 'danger', icon: 'receipt', key: 'refund', tour: 'refund.go' })}</div>`, { name: 'refund' });
  for (const o of document.querySelectorAll('#rf-why option')) o.textContent = t(o.dataset.t);
  $('#rfGo').onclick = async () => {
    const why = $('#rf-why').value, note = $('#rf-note').value.trim();
    // `other` carries its words in the reason itself (`other:<text>`).
    const reason = why === 'other' ? 'other:' + (note || '-') : why;
    const base = { reason, ...(note && why !== 'other' ? { note } : {}) };
    const amt = $('#rf-amt') ? parseInt($('#rf-amt').value, 10) : NaN;
    try {
      await busy($('#rfGo'), async () => {
        try { return await send(id, Number.isInteger(amt) ? { ...base, card_amount: amt } : base, k); }
        catch (e) {
          // NO STRIPE KEY ON THE HUB: the refund still starts; the card is by hand.
          if (e.status !== 409 || !String(e.message).includes('Stripe connected')) throw e;
          toast(t('cardManual'));
          k = key(id);
          return send(id, base, k);
        }
      });
      toast(t('refundStarted'));
      await after?.();
    } catch (e) { toast(String(e.message || e)); }
  };
}

/// The card refund's state on the order sheet, and its one button: "refund
/// to card" for what is left (the rest of a partial refund, or a try again
/// after `failed`). Nothing while a card refund is still on its way.
export function cardLine(o){
  const c = cardOf(o);
  if (!c.paid || !o.refund) return '';
  const st = c.last?.status;
  const said = c.manual && !c.last ? `<p class="muted small" data-t="cardManual"></p>`
    : IN_FLIGHT.has(st) ? `<p class="muted small" data-t="cardPending"></p>`
    : st === 'succeeded' ? `<p class="small"><span data-t="cardDone"></span> ${esc(money(c.done))}</p>`
    : st === 'failed' || st === 'canceled' ? `<p class="err small"><span data-t="cardFailed"></span>: ${esc(c.last.failure || st)}. <span data-t="tryAgain"></span></p>` : '';
  const more = o.status === 'REFUNDING' && !c.manual && c.left > 0 && !IN_FLIGHT.has(st)
    ? `${field({ id: 'rc-amt', key: 'cardAmount', money: true, value: c.left, autocomplete: 'off', tour: 'order.cardAmount' })}
       <div class="btn-row">${btn({ variant: 'danger', icon: 'credit-card', key: st === 'failed' ? 'tryAgain' : 'cardMore', data: { cardmore: o.id }, tour: 'order.cardMore' })}</div>` : '';
  return `<p class="eyebrow mt-3" data-t="cardPart"></p>${partsHtml(c)}${said}${more}`;
}

/// One more card refund on a REFUNDING order: `{card_amount}` alone.
export async function refundToCard(id, el, after){
  const amt = parseInt($('#rc-amt')?.value, 10);
  try {
    await busy(el, () => send(id, { card_amount: amt }, key(id)));
    toast(t('cardSent'));
    await after?.();
  } catch (e) { toast(String(e.message || e)); }
}

/// REFUNDING -> COMPENSATED_REFUND: the money is back in the customer's hand.
export async function moneyBack(id, el, after){
  try {
    await busy(el, () => send(id, { complete: true }, key(id)));
    toast(t('refundDone'));
    await after?.();
  } catch (e) { toast(String(e.message || e)); }
}
