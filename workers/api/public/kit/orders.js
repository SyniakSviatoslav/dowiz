// The customer's own orders — what this device has placed, and how to read one
// back from the hub.
//
// WHY THIS EXISTS. `track-order` and `my-orders` drew `#FD785462`,
// `ItaliaCrisp Pizza` and a timeline stopped on "On the Way" — the Figma frame's
// order, identical for every customer, at every venue, forever. A customer who
// had just paid tapped "View Order" and was shown somebody else's fictional
// pizza. The kit could place a real order and then could not show it.
//
// THE KEY IS RETURNED ONCE. `POST …/orders` mints a token scoped to that one
// order (`storefront.rs`: "minted once, here, and returned exactly once") and
// the hub keeps no copy. `/api/order/:id` refuses without it — an order id is
// not a secret, and behind it sit a name, a phone and a street address. So the
// token is kept beside the id on the device that placed the order, and losing
// this browser's storage means losing the ability to read it back. That is the
// same bargain the storefront makes, and it is the honest one: the alternative
// is an id anybody can walk.
//
// NOTHING HERE DECIDES A STATE. `kernel/src/order_machine.rs` folds events into
// the status; this module carries the hub's answer to a screen. Which statuses
// mean the order ended without the venue keeping the money is the KERNEL's
// question, so it is imported rather than retyped -- see `/lib/vocab.js`.

import { REFUSED } from '/lib/vocab.js';

const KEY = 'dowiz.kit.orders';

const read = () => {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) || 'null');
    if (!Array.isArray(raw)) return [];
    // Storage is untrusted: another version of this app wrote it, or a person
    // did. An entry with no id cannot be read back and is not kept.
    return raw.filter(o => o && typeof o === 'object' && typeof o.id === 'string' && o.id);
  } catch { return []; }
};

const write = list => {
  try { localStorage.setItem(KEY, JSON.stringify(list.slice(0, 20))); } catch { /* private window */ }
};

/** Every order this device has placed, newest first. */
export const all = () => read();

/** The most recent one, or null. */
export const last = () => read()[0] || null;

export const find = id => read().find(o => o.id === id) || null;

/**
 * Keep what the hub just answered, plus what the basket knows.
 *
 * The LINES are kept locally because the hub's envelope names products by id and
 * this screen wants the dish's name — and because a receipt should still be
 * readable with no network at all.
 */
export function remember(answer, { lines, currency, address, placedAtMs = Date.now() }){
  if (!answer || !answer.id) return null;
  const entry = {
    id: answer.id,
    token: answer.access_token || '',
    status: answer.status || 'PENDING',
    total: Number.isInteger(answer.total) ? answer.total : null,
    currency: currency || '',
    address: address || null,
    placedAtMs,
    lines: (lines || []).map(l => ({ name: l.name, kind: l.kind || '', variant: l.variant || '',
                                     cents: l.cents, qty: l.qty, addons: l.addons || [] })),
  };
  write([entry, ...read().filter(o => o.id !== entry.id)]);
  return entry;
}

/**
 * Ask the hub what this order is doing now.
 *
 * Returns the stored entry updated with the hub's status, or the stored entry
 * unchanged when there is no network or no key — a tracking screen that goes
 * blank because the wifi dropped is worse than one showing the last known step.
 */
export async function refresh(id){
  const kept = find(id);
  if (!kept) return null;
  if (!kept.token) return kept;
  try {
    const r = await fetch(`/api/order/${encodeURIComponent(kept.id)}`,
                          { headers: { authorization: `Bearer ${kept.token}` } });
    if (!r.ok) return kept;                      // 401 after seven days: the key expired
    const body = await r.json();
    const next = { ...kept,
      status: typeof body.status === 'string' ? body.status : kept.status,
      total: Number.isInteger(body.total) ? body.total : kept.total,
      courierName: body.courier_name || kept.courierName || '',
      events: Array.isArray(body.events) ? body.events : (kept.events || []) };
    write([next, ...read().filter(o => o.id !== next.id)]);
    return next;
  } catch { return kept; }                       // offline: the last known step stands
}

/// The kernel's states, in the order they happen. A status this list does not
/// know (`REJECTED`, `CANCELLED`) is not a step on a timeline and is drawn as
/// what it is: an ending.
export const FLOW = ['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED'];
/// The endings, from the kernel's own `took_money`. Spelled out here as
/// `s === 'REJECTED' || s === 'CANCELLED'`, it was short by
/// `COMPENSATED_REFUND` -- a refunded order would have sat under "Active" for
/// ever, because nothing else moves it.
export const isOver = s => REFUSED.has(s);
