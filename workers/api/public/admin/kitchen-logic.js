// The kitchen board, the PURE half: who is signed in and what they may open,
// which column a ticket stands in, how old it is, which button moves it, and
// the all-day counts. No DOM, no clock, no network, so node tests every rule
// (`kitchen-logic.test.mjs`); `admin/kitchen.js` is the DOM half.
//
// THE KERNEL DECIDES, NOT THIS FILE. A bump names an intent the console's own
// order button already sends (`POST /api/owner/orders/:id/action`); the hub's
// FSM refuses an illegal edge. Column names are words for a person, the
// statuses behind them are the kernel's twelve (`vocabulary.sh`).
//
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

/// Which console tabs each capability opens (ANY of the words listed). An
/// owner (no caps list) opens all; a tab absent here is never shown to staff.
/// OPERATOR 2026-09-26: ingredients and stock are ONE screen ("Ingredients &
/// stock", the `stock` tab), opened by either the shelf or the menu word.
export const TAB_CAPS = {
  orders: ['take_orders'],
  kitchen: ['advance'],
  menu: ['catalog'],
  stock: ['stock', 'catalog'],
};

/// The statuses a pass works, and the column each stands in.
export const COLUMNS = [
  ['new', ['PENDING', 'CONFIRMED']],
  ['preparing', ['PREPARING']],
  ['ready', ['READY']],
];
export const OPEN = COLUMNS.flatMap(([, s]) => s);

/// Operator Q6 (2026-09-26): three stations. `kitchen` is also every line
/// that names none (bell_route: absent IS the kitchen).
export const STATIONS = ['sushi', 'kitchen', 'bar'];

/// The five reasons a write-off may give, as the hub spells them.
export const WASTE_REASONS = ['spoiled', 'dropped', 'unsold', 'returned', 'staff_meal'];

/// Minutes before a ticket turns amber, then red, when the venue names no
/// cooking time of its own.
export const WARN_MIN = 10;
export const LATE_MIN = 20;
const MIN_MS = 60_000;

/// The signed claims a token carries, read (NOT verified -- the hub verifies
/// every request) to decide what to draw. `null` for anything unreadable.
export function claimsOf(token){
  try {
    const part = String(token || '').split('.')[1];
    if (!part) return null;
    const b64 = part.replace(/-/g, '+').replace(/_/g, '/');
    const pad = b64 + '='.repeat((4 - (b64.length % 4)) % 4);
    const bytes = Uint8Array.from(atob(pad), c => c.charCodeAt(0));
    return JSON.parse(new TextDecoder().decode(bytes));
  } catch { return null; }
}

/// `{ staff, caps }`: a staff token's capability words, or `staff: false` for
/// anyone else (an owner holds every word).
export function principalOf(token){
  const c = claimsOf(token);
  if (!c || c.role !== 'staff') return { staff: false, caps: new Set() };
  return { staff: true, caps: new Set(String(c.caps || '').split(',').filter(Boolean)) };
}

/// May this principal use `word` (a capability name)?
export const can = (p, word) => !p.staff || p.caps.has(word);

/// The tabs this principal sees, in the order given. An owner sees them all;
/// a member of staff sees a tab only when they hold the word behind it.
export function tabsFor(p, tabs){
  return tabs.filter(id => !p.staff || (TAB_CAPS[id] || []).some(w => p.caps.has(w)));
}

/// The column a status stands in, or null when it is off the pass.
export function columnOf(status){
  const c = COLUMNS.find(([, s]) => s.includes(status));
  return c ? c[0] : null;
}

/// The one bump a ticket offers: the next kitchen edge, or null. A delivery
/// at READY leaves the pass (the courier's); a pickup or a table is handed over.
export function bumpFor(o){
  switch (o && o.status) {
    case 'PENDING': return 'confirm';
    case 'CONFIRMED': return 'preparing';
    case 'PREPARING': return 'ready';
    case 'READY': return (o.fulfilment && o.fulfilment.kind) === 'delivery' ? null : 'collected';
    default: return null;
  }
}

/// Reject before the kitchen accepts; cancel after. Both carry a reason.
export const stopFor = o => (o && o.status === 'PENDING' ? 'reject' : o && o.status === 'CONFIRMED' ? 'cancel' : null);

/// Whole minutes a ticket has waited. Integer arithmetic; never negative.
export const ageMin = (createdMs, nowMs) => Math.max(0, Math.floor((Number(nowMs) - Number(createdMs ?? nowMs)) / MIN_MS));

/// `ok` / `warn` / `late` by age.
export function ageClass(createdMs, nowMs, warn = WARN_MIN, late = LATE_MIN){
  const m = ageMin(createdMs, nowMs);
  return m >= late ? 'late' : m >= warn ? 'warn' : 'ok';
}

/// The venue's thresholds: its own cooking minutes when it names them.
export function thresholds(venue){
  const cook = Number(venue && venue.kitchen && venue.kitchen.defaultCookingMin) || 0;
  return cook > 0 ? { warn: cook, late: cook * 2 } : { warn: WARN_MIN, late: LATE_MIN };
}

/// Where a line is made.
export const stationOf = line => (STATIONS.includes(line && line.station) ? line.station : 'kitchen');

/// The open tickets for a station (`all` = every station), each with only the
/// lines made there, oldest first. A ticket with no line here is not shown.
export function ticketsFor(orders, station = 'all'){
  return (orders || [])
    .filter(o => OPEN.includes(o.status))
    .map(o => ({ ...o, items: (o.items || []).filter(l => station === 'all' || stationOf(l) === station) }))
    .filter(o => o.items.length > 0)
    .sort((a, b) => (a.created_at_ms || 0) - (b.created_at_ms || 0));
}

/// Tickets grouped into the three columns.
export function board(orders, station = 'all'){
  const out = Object.fromEntries(COLUMNS.map(([c]) => [c, []]));
  for (const o of ticketsFor(orders, station)) out[columnOf(o.status)].push(o);
  return out;
}

/// All-day counts: how many of each dish the open, not-yet-ready tickets
/// still need, biggest first. Keyed by product id, named by the line.
export function allDay(orders, station = 'all'){
  const m = new Map();
  for (const o of ticketsFor(orders, station)) {
    if (o.status === 'READY') continue;
    for (const l of o.items) {
      const id = l.product_id || l.name;
      const had = m.get(id) || { id, name: l.name || l.product_id || '', qty: 0 };
      had.qty += Number(l.quantity) | 0;
      m.set(id, had);
    }
  }
  return [...m.values()].sort((a, b) => b.qty - a.qty || String(a.name).localeCompare(String(b.name)));
}

/// How many tickets each station has open, for the filter chips.
export function stationCounts(orders){
  const out = { all: ticketsFor(orders, 'all').length };
  for (const s of STATIONS) out[s] = ticketsFor(orders, s).length;
  return out;
}

/// The short id a pass calls out.
export const shortId = id => String(id || '').slice(-4).toUpperCase();

/// Where the ticket goes, in words the pass uses: a table, pickup, delivery.
export function whereOf(o){
  const f = (o && o.fulfilment) || {};
  if (f.table) return { key: 'kTable', table: String(f.table) };
  return { key: f.kind === 'delivery' ? 'kDelivery' : 'kPickup', table: '' };
}

/// The request a bump or a stop sends: the console's own route and body.
export function actionRequest(orderId, action, loc, reason = ''){
  const why = String(reason || '').trim();
  return { path: `/owner/orders/${encodeURIComponent(orderId)}/action`,
           body: { location_id: loc, action, ...(why ? { reason: why } : {}) } };
}

/// Where the list comes from: the kitchen's stripped read for a member of
/// staff without `take_orders`, the owner's queue for everyone else.
export const ordersPath = (p, loc) => (p.staff && !p.caps.has('take_orders') ? `/staff/kitchen?location_id=${encodeURIComponent(loc)}` : `/owner/orders?location_id=${encodeURIComponent(loc)}`);
