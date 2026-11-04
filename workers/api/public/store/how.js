// PURE: HOW AN ORDER REACHES THE CUSTOMER, as the storefront asks it. No
// imports, no DOM -- the session store is passed in -- so `how.test.mjs` can
// call every rule the way `bag-core.js` is called.
//
// THE DEFECT THIS CLOSES (W-SERVICE-MODE, 2026-11-03): the basket was born a
// delivery (`state.how = 'delivery'`), the cart sheet drew the delivery fee on
// its first paint with no way to change it, and the checkout offered only
// delivery and pickup. A customer sitting in the venue's hall could not say so
// unless they had scanned the QR on the table (`table.js`), so the fee stood
// on every order they placed from the app. The hub already prices a `dine_in`
// order at no fee (`services/ordering/fulfilment.rs::fee`) and refuses one
// that names no table: this module is the browser's half of those two rules,
// and the hall is offered exactly when the venue has published a plan with a
// table in it (`GET /api/public/locations/:slug/tables`, the booking's plan).
//
// Money stays in integer minor units; nothing here formats a number.
// ASCII QUOTES ONLY in this file.

export const DELIVERY = 'delivery';
export const PICKUP = 'pickup';
export const DINE_IN = 'dine_in';
/// `table.js`: a round from the signed QR on a table. Not a choice here, and
/// never a trip.
export const TABLE = 'table';

/// The choice lives in sessionStorage: this visit, this tab. The next time the
/// same phone opens the storefront at home it is a delivery again.
export const KEY = 'dw_how';
/// The plan is asked for a slot this far ahead: the hub refuses a slot before
/// its own clock, and a phone's clock can run a few minutes behind it.
export const PLAN_SLOT_LEAD_MIN = 10;

/// Does this order leave the building? Mirrors `fulfilment::leaves_the_building`.
export const leavesTheBuilding = how => how === DELIVERY;

/// The modes this venue offers, in the order they are shown. Delivery always
/// (it is what the storefront has always sold); pickup when the venue's switch
/// is on; the hall when the plan has at least one table to order to.
export function offered(L, plan){
  const out = [DELIVERY];
  if (L && L.pickup) out.push(PICKUP);
  if (hallTables(plan).length) out.push(DINE_IN);
  return out;
}

/// The mode to use: the one asked for when the venue offers it, else the first.
export const pick = (how, modes) => (modes.includes(how) ? how : modes[0]);

/// What this order pays for its journey. THE SAME RULE AS THE HUB'S
/// (`fulfilment::fee`): the free-delivery threshold first and for every kind,
/// then no trip means no fee. The number shown is the number charged only
/// because both sides compute it this way; the hub's is the one that counts.
export function tripFee(how, subtotal, L){
  if (!L) return 0;
  if (L.freeDeliveryThreshold != null && subtotal >= L.freeDeliveryThreshold) return 0;
  if (!leavesTheBuilding(how)) return 0;
  return Number.isInteger(L.deliveryFee) ? L.deliveryFee : 0;
}

/// Every table of the venue's plan, flat: `{ zone, name, n }`. A plan that is
/// not a plan (an error body, `zones: []`) is a hall with no tables.
export function hallTables(plan){
  const zones = plan && Array.isArray(plan.zones) ? plan.zones : [];
  const out = [];
  for (const z of zones) {
    if (!z || typeof z.id !== 'string' || !z.id) continue;
    for (const t of (Array.isArray(z.tables) ? z.tables : [])) {
      if (t && Number.isInteger(t.n)) out.push({ zone: z.id, name: typeof z.name === 'string' && z.name ? z.name : z.id, n: t.n });
    }
  }
  return out;
}

/// The table as the hub writes it on the ticket (`room::table_link::table_text`).
export const tableText = (zone, n) => `${zone}:${n}`;

/// The chosen table, only if the plan still has it; a remembered table the
/// owner has since removed is no table.
export function hallTable(plan, choice){
  if (!choice) return null;
  return hallTables(plan).find(t => t.zone === choice.zone && t.n === choice.n) || null;
}

/// What is missing before this order can be placed, or null: the hub's
/// `fulfilment::needs`, said here first and in the guest's language.
export function missing(how, { addressLine = '', table = null } = {}){
  if (how === DELIVERY && !String(addressLine || '').trim()) return 'address';
  if (how === DINE_IN && !table) return 'table';
  return null;
}

/// The order body's `fulfilment` member for each mode. `address` is the
/// delivery address as the checkout composed it (line, parts, pin); the note
/// rides on the address for a delivery and beside the kind otherwise, which is
/// where the hub reads it (`FulfilmentIn`).
export function fulfilmentBody(how, { address = {}, note = '', table = null } = {}){
  const n = note || null;
  if (how === PICKUP) return { kind: PICKUP, note: n };
  if (how === DINE_IN) return { kind: DINE_IN, table: table ? tableText(table.zone, table.n) : null, note: n };
  return { kind: DELIVERY, address: { ...address, note: n } };
}

/// Remember the choice for the visit. Every touch of the store is wrapped:
/// private mode throws, and a choice that cannot be remembered is still the
/// choice on screen.
export function keep(session, how, table){
  try { session.setItem(KEY, JSON.stringify({ how, table: table ? { zone: table.zone, n: table.n } : null })); } catch {}
}
export function recall(session){
  try {
    const v = JSON.parse(session.getItem(KEY) || 'null');
    if (!v || ![DELIVERY, PICKUP, DINE_IN].includes(v.how)) return null;
    const t = v.table;
    return { how: v.how, table: t && typeof t.zone === 'string' && Number.isInteger(t.n) ? { zone: t.zone, n: t.n } : null };
  } catch { return null; }
}

/// The slot the plan is asked for, in minutes since the epoch.
export const planSlotMin = nowMs => Math.floor(nowMs / 60000) + PLAN_SLOT_LEAD_MIN;
