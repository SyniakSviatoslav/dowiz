/* eslint-disable local/no-hardcoded-string -- internal runtime/error strings (WASM glue, error codes, header names, selectors) and test seams -- not user-facing UI copy; do not wrap in t() */
// port of kernel/src/analytics.rs
//
// Plain-JS mirror of `ChannelLedger` + `reduce_anomalies` over a JS event
// array, so the owner dashboard is functional standalone before the WASM
// binding wave lands. Logic is a literal port of analytics.rs:
//   * ingest: duplicate order_id is ignored (channel locked to first sighting,
//     status updates the funnel). Counts are exact integers, no float.
//   * orders_by_channel: distinct orders per channel, descending by count then name.
//   * funnel(channel): status distribution in canonical OrderStatus enum order,
//     fixed shape (missing stages read as 0).
//   * reduce_anomalies: groups (order_id, status, at_ms), sorts by at_ms, folds
//     through the decide/fold Law, counts orders with an illegal sequence.
//
// Event shape (ChannelEvent-like JS object):
//   { order_id: string, channel: string, status: string, at_ms: number }
// `status` is the canonical UPPER_SNAKE string (e.g. "PENDING", "DELIVERED"),
// mirroring OrderStatus::as_str() in kernel/src/order_machine.rs.

// Canonical funnel stage ordering — mirrors analytics.rs `funnel()` stage_order.
const STAGE_ORDER = [
  'PENDING',
  'CONFIRMED',
  'PREPARING',
  'READY',
  'IN_DELIVERY',
  'DELIVERED',
  'REJECTED',
  'CANCELLED',
  'SCHEDULED',
  'PICKED_UP',
];

// Transition table — 1:1 port of order_machine.rs `allowed_next`.
const ALLOWED_NEXT = {
  PENDING: ['CONFIRMED', 'REJECTED', 'CANCELLED'],
  CONFIRMED: ['PREPARING', 'IN_DELIVERY'],
  PREPARING: ['READY'],
  READY: ['IN_DELIVERY', 'PICKED_UP'],
  IN_DELIVERY: ['DELIVERED'],
  DELIVERED: [],
  REJECTED: [],
  CANCELLED: [],
  SCHEDULED: [],
  PICKED_UP: [],
};

// assert_transition — returns null on valid, else an error code string.
// Mirrors order_machine.rs assert_transition (SameStatus / ScaffoldDisabled /
// Illegal). SCHEDULED is the scaffold-only status.
function assertTransition(from, to) {
  if (from === to) return 'SameStatus';
  if (to === 'SCHEDULED' || from === 'SCHEDULED') return 'ScaffoldDisabled';
  const allowed = ALLOWED_NEXT[from] || [];
  if (!allowed.includes(to)) return 'Illegal';
  return null;
}

// fold_transitions — mirrors order_machine.rs fold_transitions.
// start is the seed (first observed status); steps are the remaining statuses.
// Returns { ok, status, error }.
function foldTransitions(start, steps) {
  let cur = start;
  for (const next of steps) {
    const err = assertTransition(cur, next);
    if (err) return { ok: false, status: cur, error: err };
    cur = next;
  }
  return { ok: true, status: cur, error: null };
}

// ChannelLedger mirror.
export function createLedger() {
  /** @type {Map<string, { channel: string, status: string }>} */
  const orders = new Map();
  /** @type {Map<string, number>} channel -> count */
  const byChannel = new Map();
  /** @type {Map<string, number>} `${channel}|${status}` -> count */
  const funnelCounts = new Map();

  function funnelKey(channel, status) {
    return `${channel}|${status}`;
  }

  // ingest — port of ChannelLedger::ingest. Returns true if newly recorded.
  function ingest(ev) {
    const existing = orders.get(ev.order_id);
    if (existing) {
      // Duplicate order_id: ignore re-attribution. Update funnel to new status.
      const oldStatus = existing.status;
      const channel = existing.channel;
      const k = funnelKey(channel, oldStatus);
      funnelCounts.set(k, Math.max(0, (funnelCounts.get(k) || 0) - 1));
      existing.status = ev.status;
      const nk = funnelKey(channel, ev.status);
      funnelCounts.set(nk, (funnelCounts.get(nk) || 0) + 1);
      return false;
    }
    orders.set(ev.order_id, { channel: ev.channel, status: ev.status });
    byChannel.set(ev.channel, (byChannel.get(ev.channel) || 0) + 1);
    const k = funnelKey(ev.channel, ev.status);
    funnelCounts.set(k, (funnelCounts.get(k) || 0) + 1);
    return true;
  }

  // orders_by_channel — descending by count then channel name.
  function ordersByChannel() {
    const out = [...byChannel.entries()].map(([channel, count]) => [channel, count]);
    out.sort((a, b) => {
      if (b[1] !== a[1]) return b[1] - a[1];
      return a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0;
    });
    return out;
  }

  // funnel(channel) — canonical stage order, fixed shape (0 for missing).
  function funnel(channel) {
    return STAGE_ORDER.map((status) => {
      const c = funnelCounts.get(funnelKey(channel, status)) || 0;
      return [status, c];
    });
  }

  return { ingest, ordersByChannel, funnel };
}

// reduce_anomalies — port of analytics::reduce_anomalies.
// events: Array<{ order_id, status, at_ms }>
export function reduceAnomalies(events) {
  // order_id -> BTreeMap<at_ms, status> (sorted ascending by at_ms).
  const byOrder = new Map();
  for (const ev of events) {
    if (!byOrder.has(ev.order_id)) byOrder.set(ev.order_id, new Map());
    byOrder.get(ev.order_id).set(ev.at_ms, ev.status);
  }

  let anomalies = 0;
  for (const seq of byOrder.values()) {
    const statuses = [...seq.values()];
    if (statuses.length === 0) continue;
    const start = statuses[0];
    const steps = statuses.slice(1);
    const res = foldTransitions(start, steps);
    if (!res.ok) anomalies += 1;
  }
  return anomalies;
}
