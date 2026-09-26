// The owner's voice, the PURE half: a confirmed instruction as the request the
// console's own button makes. No imports, no DOM, so node tests it
// (`admin/voice.test.mjs`); `admin/voice.js` is the DOM half.
//
// A CONFIRMATION CARRIES ITS OWN INSTRUCTION (services/engagement/voice.rs):
// the order, the dish or the state here come from the signed token the hub
// handed back, never from what this screen remembers of the read-back.

/// The hub's order verbs the console's buttons already send
/// (`POST /api/owner/orders/:id/action`, owner.rs).
export const ORDER_VERBS = new Set(['confirm', 'preparing', 'ready', 'reject', 'cancel']);
/// The venue states the header chip sets.
export const STATES = ['open', 'busy', 'closed'];

/// A refusal or a cancellation carries a reason, as the button's dialog asks.
export const needsReason = verb => verb === 'reject' || verb === 'cancel';

/// `{ path, body }` for a confirmed instruction, or null for one this console
/// does not act on (a courier's, or a malformed one).
export function planOf(done, loc, reason = '') {
  if (!done || !done.verb) return null;
  const v = done.verb, a = done.args || {};
  if (ORDER_VERBS.has(v) && done.orderId) {
    const why = needsReason(v) && String(reason || '').trim();
    return { path: `/owner/orders/${encodeURIComponent(done.orderId)}/action`, body: { location_id: loc, action: v, ...(why ? { reason: why } : {}) } };
  }
  if ((v === 'dish_off' || v === 'dish_on') && a.productId) {
    return { path: `/owner/products/${encodeURIComponent(a.productId)}`, body: { location_id: loc, available: v === 'dish_on' } };
  }
  if (v === 'venue' && STATES.includes(a.state)) return { path: '/owner/location', body: { location_id: loc, status: a.state } };
  // THE KITCHEN'S SHELF (voice/kitchen.rs): the stock route takes the venue in
  // its query and refuses any field it does not know.
  const q = `?location_id=${encodeURIComponent(loc)}`;
  if (v === 'receive' && a.itemId && Number.isInteger(a.qty)) return { path: `/owner/stock/received${q}`, body: { item: a.itemId, qty: a.qty } };
  if (v === 'waste' && a.itemId && Number.isInteger(a.qty) && a.reason) return { path: `/owner/stock/wasted${q}`, body: { item: a.itemId, qty: a.qty, reason: a.reason } };
  return null;
}

/// Where a question goes: the kitchen's assistant (no customer data) for a
/// member of staff, the owner's for the owner.
export const assistPath = (staff, loc) => (staff ? `/staff/assist?location_id=${encodeURIComponent(loc)}` : '/owner/assist');

/// The line a read-only answer shows, or null when it is not one.
export function lineOf(r, t) {
  if (r && r.action === 'status') return t('voiceStatus').replace('{open}', r.open).replace('{waiting}', r.waiting);
  return null;
}
