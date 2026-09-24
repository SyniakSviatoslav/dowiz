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

import { $, esc, icon, t, api, withLoc, toast, sheet, busy } from '/admin/core.js';
import { T, LANGS } from '/admin/i18n.js';

const WORDS = {
  sq: { refund: 'Rimburso', refundHint: 'Porosia mbyllet si e rimbursuar dhe del te Perjashtimet me emrin tuaj.', refundReason: 'Arsyeja', rr_venue_cancelled: 'Lokali e anuloi', rr_customer_request: 'Me kerkese te klientit', rr_payment_error: 'Gabim pagese', rr_other: 'Tjeter', refundNote: 'Shenim (per tjeter)', moneyBack: 'Parate u kthyen', refundStarted: 'Rimbursimi filloi', refundDone: 'Rimbursimi u mbyll' },
  en: { refund: 'Refund', refundHint: 'The order ends as refunded and is listed under Exceptions with your name.', refundReason: 'Reason', rr_venue_cancelled: 'The venue cancelled', rr_customer_request: 'The customer asked', rr_payment_error: 'Payment error', rr_other: 'Other', refundNote: 'Note (for other)', moneyBack: 'Money handed back', refundStarted: 'Refund started', refundDone: 'Refund complete' },
  uk: { refund: 'Повернення', refundHint: 'Замовлення завершиться як повернене і з\'явиться у Винятках з вашим ім\'ям.', refundReason: 'Причина', rr_venue_cancelled: 'Заклад скасував', rr_customer_request: 'На прохання клієнта', rr_payment_error: 'Помилка оплати', rr_other: 'Інше', refundNote: 'Примітка (для іншого)', moneyBack: 'Гроші повернуто', refundStarted: 'Повернення розпочато', refundDone: 'Повернення завершено' },
};
for (const l of LANGS) Object.assign(T[l], WORDS[l]);

/// `command::refund::RefundReason`, the words the hub parses. `refused_at_door`
/// is the courier's, never the console's.
const REASONS = ['venue_cancelled', 'customer_request', 'payment_error', 'other'];
/// Where the refund door is open (`allowed_next` into REFUNDING).
export const REFUNDABLE = new Set(['CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY']);

const key = id => (globalThis.crypto?.randomUUID?.() || String(Date.now())) + '-' + id;
const send = (id, body, k) => api(`/staff/orders/${encodeURIComponent(id)}/refund`, { method: 'POST', body: withLoc(body), headers: { 'idempotency-key': k } });

/// The reason sheet; resolves once the hub answered (or the owner went back).
export function openRefund(id, after){
  const k = key(id);
  sheet(`<p class="eyebrow">#${esc(id.slice(0, 8))}</p><h2 data-t="refund"></h2><p class="muted small" data-t="refundHint"></p>
    <label for="rf-why" data-t="refundReason"></label>
    <select id="rf-why">${REASONS.map(r => `<option value="${r}" data-t="rr_${r}"></option>`).join('')}</select>
    <label for="rf-note" data-t="refundNote"></label><input id="rf-note" maxlength="140" autocomplete="off">
    <div class="btn-row"><button class="btn danger" id="rfGo" type="button">${icon('receipt')}<span data-t="refund"></span></button></div>`, { name: 'refund' });
  for (const o of document.querySelectorAll('#rf-why option')) o.textContent = t(o.dataset.t);
  $('#rfGo').onclick = async () => {
    const why = $('#rf-why').value, note = $('#rf-note').value.trim();
    // `other` carries its words in the reason itself (`other:<text>`).
    const reason = why === 'other' ? 'other:' + (note || '-') : why;
    try {
      await busy($('#rfGo'), () => send(id, { reason, ...(note && why !== 'other' ? { note } : {}) }, k));
      toast(t('refundStarted'));
      await after?.();
    } catch (e) { toast(String(e.message || e)); }
  };
}

/// REFUNDING -> COMPENSATED_REFUND: the money is back in the customer's hand.
export async function moneyBack(id, el, after){
  try {
    await busy(el, () => send(id, { complete: true }, key(id)));
    toast(t('refundDone'));
    await after?.();
  } catch (e) { toast(String(e.message || e)); }
}
