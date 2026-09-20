// The console's own copy of the queue.
//
// WHAT IT IS FOR. A kitchen's console is the one screen in this system that
// must not go blank. It is open all day on a phone or a tablet at the pass,
// on whatever the venue's wifi is doing at six in the evening; every reload
// used to mean an empty screen until a request came back, and every outage
// meant a screen that stopped being true without saying so.
//
// The replica is the orders as the server last described them, in this
// browser, with the generation they were current at. On boot the console draws
// it IMMEDIATELY -- no request, no spinner -- and then reconciles. While the
// network is gone it keeps drawing, and it says how old what it is drawing is.
//
// THE SERVER IS STILL THE AUTHORITY, and this is a PREDICTION in exactly the
// sense game netcode means: events that arrive over the socket are applied
// locally so the screen keeps up, and the next successful read replaces the
// lot. A replica that argued with the server would be a second fold, and two
// folds disagree eventually.
//
// STORAGE CAN FAIL AND THAT IS NORMAL: a private window, blocked site data, a
// full quota. Every read and write is wrapped, and a replica that cannot be
// stored simply is not one -- the console then behaves exactly as it did
// before this file existed.

const KEY = 'dowiz.replica.v1';
/// Older than this and it is history, not a queue: drawn, but announced as
/// stale rather than presented as now.
export const STALE_MS = 15 * 60 * 1000;

const read = () => {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return null;
    const v = JSON.parse(raw);
    if (!v || !Array.isArray(v.orders) || typeof v.generation !== 'number') return null;
    return v;
  } catch { return null; }
};

const write = v => { try { localStorage.setItem(KEY, JSON.stringify(v)); } catch { /* full, private, blocked */ } };

/// The stored copy, or an empty one. `at` is when the server last answered.
export function load(venue){
  const v = read();
  if (!v || v.venue !== venue) return { venue, generation: -1, at: 0, orders: [] };
  return v;
}

/// Replace the copy with what the server just said.
export function replace(venue, orders, generation){
  const v = { venue, generation, at: Date.now(), orders };
  write(v);
  return v;
}

/// Apply what changed since. Returns the new copy, or `null` when the changes
/// cannot be applied on top of what is held -- a gap, a different venue -- in
/// which case the caller reads the list.
///
/// AN UNKNOWN ORDER IS NOT INVENTED. A change for an order this copy has never
/// seen means the copy is missing something: a delta folded onto nothing would
/// render a row with a status and no items, and a kitchen would read it as an
/// order it has to cook.
export function apply(current, changes, generation){
  if (!current || !Array.isArray(changes)) return null;
  if (generation <= current.generation) return current;
  const byId = new Map(current.orders.map(o => [o.id, o]));
  for (const c of changes) {
    if (!c || !c.order_id) return null;
    let payload;
    try { payload = JSON.parse(c.payload || '{}'); } catch { return null; }
    if (!payload || typeof payload !== 'object') return null;
    const have = byId.get(c.order_id);
    if (!have) {
      // A placement carries the whole order, so it can be added; anything else
      // is a change to something this copy does not hold.
      if (payload._d) return null;
      byId.set(c.order_id, { ...payload, id: c.order_id });
      continue;
    }
    byId.set(c.order_id, payload._d ? merge(have, payload) : { ...payload, id: c.order_id });
  }
  const next = { venue: current.venue, generation, at: Date.now(), orders: [...byId.values()] };
  write(next);
  return next;
}

/// The same merge the server folds with: `null` deletes, objects recurse,
/// everything else is replaced whole. The marker never reaches the order.
function merge(base, delta){
  const out = { ...base };
  for (const [k, v] of Object.entries(delta)) {
    if (k === '_d') continue;
    if (v === null) { delete out[k]; continue; }
    if (v && typeof v === 'object' && !Array.isArray(v)
        && out[k] && typeof out[k] === 'object' && !Array.isArray(out[k])) {
      out[k] = merge(out[k], v);
      continue;
    }
    out[k] = v;
  }
  return out;
}

/// How old this copy is, in milliseconds, and whether that is too old to
/// present as the present.
export const ageOf = copy => (copy?.at ? Date.now() - copy.at : Infinity);
export const isStale = copy => ageOf(copy) > STALE_MS;

/// Forget it. Used when the console logs out: the next person at this screen
/// is not entitled to the last one's queue.
export function forget(){ try { localStorage.removeItem(KEY); } catch {} }
