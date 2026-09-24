// A GUEST'S ROUND, from the QR on the table (A9): born PENDING, unsigned by any
// person, waiting for the room. A waiter with `take_orders` confirms it into
// the kitchen or rejects it (`POST /api/staff/orders/:id/guest`,
// services/orders/room/guest_round.rs). Nothing else about the round differs:
// it is amended, paid and moved like a waiter's own.
import { esc } from './logic.js';

/// The signer word a guest's round carries (`placer::GUEST`).
export const GUEST = 'guest';

/// Is this round a guest's, still waiting for the room?
export const guestWaiting = r => r?.placed_by === GUEST && r?.status === 'PENDING';

/// Does this sitting hold one?
export const sittingHasGuest = s => (s?.rounds || []).some(guestWaiting);

/// The route and the body the answer goes out as. Pure, so it is tested.
export const guestPath = id => `/staff/orders/${encodeURIComponent(id)}/guest`;
export const guestBody = (loc, action) => ({ location_id: loc, action });

/// The two buttons, only for a waiting guest round and a person who takes orders.
export function renderGuestBar(c, round) {
  if (!guestWaiting(round) || !c.S.caps.has('take_orders')) return '';
  const { t } = c;
  return `<div class="acts guest-round" role="group" aria-label="${esc(t('guestRound'))}">
    <p class="muted">${esc(t('guestRound'))}</p>
    <button class="cta" data-act="guestConfirm">${esc(t('guestConfirm'))}</button>
    <button class="btn" data-act="guestReject">${esc(t('guestReject'))}</button></div>`;
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
