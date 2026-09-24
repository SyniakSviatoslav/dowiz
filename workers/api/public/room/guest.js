// A GUEST'S ROUND, from the QR on the table (A9): born PENDING, unsigned by any
// person, waiting for the room. A waiter with `take_orders` confirms it into
// the kitchen or rejects it (`POST /api/staff/orders/:id/guest`,
// services/orders/room/guest_round.rs). Nothing else about the round differs:
// it is amended, paid and moved like a waiter's own.
import { guestWaiting } from './logic.js';
import { ui, k, act } from './parts.js';

/// The signer word, and "a guest's round still waiting for the room": one
/// definition, in `logic.js`, because the bill (`sittingDue`) needs it too.
export { GUEST, guestWaiting } from './logic.js';

/// Does this sitting hold one?
export const sittingHasGuest = s => (s?.rounds || []).some(guestWaiting);

/// The route and the body the answer goes out as. Pure, so it is tested.
export const guestPath = id => `/staff/orders/${encodeURIComponent(id)}/guest`;
export const guestBody = (loc, action) => ({ location_id: loc, action });

/// The two buttons, only for a waiting guest round and a person who takes orders.
export function renderGuestBar(c, round) {
  if (!guestWaiting(round) || !c.S.caps.has('take_orders')) return '';
  const { t } = c;
  return `<div class="acts guest-round" role="group" aria-label="${ui.esc(t('guestRound'))}">
    ${ui.alert({ tone: 'warning', icon: 'alert-circle', label: k('guestRound') })}
    ${ui.button({ variant: 'success', size: 'lg', block: true, icon: 'check', label: k('guestConfirm'), attrs: act('guestConfirm', {}, 'guest.confirm') })}
    ${ui.button({ variant: 'danger', block: true, icon: 'x', label: k('guestReject'), attrs: act('guestReject', {}, 'guest.reject') })}</div>`;
}

/// Send the answer. A refusal says the server's words; an unreachable network
/// queues it under the key minted at the tap (net.js `write`).
export async function answerGuest(c, round, action) {
  try {
    const r = await c.write(guestPath(round.id), guestBody(c.S.loc, action), 'guest:' + round.id);
    c.toast(c.t(r.landed ? (action === 'confirm' ? 'guestConfirmed' : 'guestRejected') : 'queuedSaved'));
    c.reload();
  } catch (e) {
    c.toast(e.message || c.t('error'));
  }
}
