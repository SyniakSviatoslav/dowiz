// Prep-Time + Client ETA (v1) · P4 — DB-gathering wrapper for the pure etaService.
//
// COMPUTE-ON-READ (council-endorsed): the customer order-status endpoint calls this on each read.
// Reads are naturally event-driven — the FE (re)fetches on load, on the 30s watchdog, and on the
// `order.status` WS transition — so the range recomputes on confirmation / assignment / status change
// WITHOUT a server-side event-subscriber, Redis cache, or N² fan-out (the breaker's B-SCALE/B-FAIL
// concerns were about that subscriber design, which this deliberately avoids).
//
// Privacy (D2 redesign): the Phase-2 leg origin is the courier's LATEST courier_positions row (already
// consented + 24h-purged) — no new coordinate is persisted. Falls back to the venue when absent.
//
// Cost: ≤2 extra indexed queries per call (order-prep always; kitchen-ahead OR courier-ahead, never
// both). Per-client-read, not per-event fan-out. The deliberately-rough courier-ahead model
// (stops × PER_STOP_MIN) keeps it to one cheap COUNT.

import { computeEtaRange, deliveryLegMinutes, type EtaRange } from './etaService.js';

// Each delivery stop ahead of this order on the same courier adds ~this many minutes (a rough v1
// constant — most carried orders are already cooked, so the dominant cost is per-stop driving).
const PER_STOP_MIN = 12;

const TERMINAL = ['DELIVERED', 'REJECTED', 'CANCELLED'];

function num(v: unknown): number | null {
  if (v == null) return null;
  const n = typeof v === 'number' ? v : Number(v);
  return Number.isFinite(n) ? n : null;
}

function minutesSince(ts: string | null | undefined): number | null {
  if (!ts) return null;
  const t = new Date(ts).getTime();
  if (!Number.isFinite(t)) return null;
  return (Date.now() - t) / 60000;
}

export interface GatherArgs {
  orderId: string;
  status: string;
  locationId: string;
  createdAt: string | null;
  preparingAt: string | null;
  deliveryLat: number | string | null;
  deliveryLng: number | string | null;
  locationLat: number | string | null;
  locationLng: number | string | null;
  courierId: string | null;
  assignmentStatus: string | null;
  courierLat: number | string | null; // latest consented courier position (already fetched by caller)
  courierLng: number | string | null;
}

// Minimal db shape (the fastify pg pool / client).
interface Queryable { query: (sql: string, params?: any[]) => Promise<{ rows: any[] }> }

/**
 * Gather inputs + compute the ETA range for one order. Returns null for terminal orders (no ETA).
 * Total/defensive: any query hiccup degrades the relevant input to 0/fallback rather than throwing,
 * so the order-status endpoint never fails because of ETA.
 */
export async function gatherOrderEtaRange(db: Queryable, a: GatherArgs): Promise<EtaRange | null> {
  if (TERMINAL.includes(a.status)) return null;

  const assigned = !!a.courierId && ['accepted', 'picked_up'].includes(a.assignmentStatus || '');
  const phase = assigned ? 'assigned' : 'pre_assign';

  // Order prep = the slowest dish gates the order (kitchen parallelizes within one order). v1 choice.
  let orderPrep = 15;
  try {
    const r = await db.query(
      `SELECT COALESCE(MAX(p.prep_time_minutes), 15) AS prep
         FROM order_items oi JOIN products p ON p.id = oi.product_id
        WHERE oi.order_id = $1`,
      [a.orderId],
    );
    orderPrep = num(r.rows[0]?.prep) ?? 15;
  } catch { /* fall back to 15 */ }

  // Remaining kitchen time decays once PREPARING; 0 once the food is out of the kitchen.
  let prepRemaining = orderPrep;
  if (['READY', 'IN_DELIVERY', 'PICKED_UP'].includes(a.status)) {
    prepRemaining = 0;
  } else if (a.status === 'PREPARING') {
    const since = minutesSince(a.preparingAt);
    if (since != null) prepRemaining = Math.max(0, orderPrep - since);
  }

  // Phase 1: kitchen queue ahead (orders placed earlier still cooking). Decayed by their own start.
  let kitchenAhead = 0;
  if (!assigned) {
    try {
      const r = await db.query(
        `SELECT COALESCE(SUM(GREATEST(0,
                  op.prep - COALESCE(EXTRACT(EPOCH FROM (now() - o2.preparing_at)) / 60, 0))), 0) AS mins
           FROM orders o2
           JOIN LATERAL (
             SELECT COALESCE(MAX(p.prep_time_minutes), 15) AS prep
               FROM order_items oi JOIN products p ON p.id = oi.product_id
              WHERE oi.order_id = o2.id
           ) op ON true
          WHERE o2.location_id = $1 AND o2.status = 'PREPARING'
            AND o2.created_at < $2 AND o2.id <> $3`,
        [a.locationId, a.createdAt, a.orderId],
      );
      kitchenAhead = num(r.rows[0]?.mins) ?? 0;
    } catch { /* 0 */ }
  }

  // Phase 2: stops ahead of this order on the same courier × a rough per-stop driving cost.
  let courierAhead = 0;
  if (assigned && a.courierId) {
    try {
      const r = await db.query(
        `SELECT COUNT(*)::int AS ahead
           FROM courier_assignments
          WHERE courier_id = $1 AND status IN ('accepted', 'picked_up') AND order_id <> $2`,
        [a.courierId, a.orderId],
      );
      courierAhead = (num(r.rows[0]?.ahead) ?? 0) * PER_STOP_MIN;
    } catch { /* 0 */ }
  }

  // Delivery leg origin: courier's latest consented position when assigned, else the venue.
  const originLat = assigned ? num(a.courierLat) ?? num(a.locationLat) : num(a.locationLat);
  const originLng = assigned ? num(a.courierLng) ?? num(a.locationLng) : num(a.locationLng);
  const leg = deliveryLegMinutes(originLat, originLng, num(a.deliveryLat), num(a.deliveryLng));

  return computeEtaRange({
    phase,
    status: a.status,
    prepRemainingMinutes: prepRemaining,
    kitchenQueueAheadMinutes: kitchenAhead,
    courierQueueAheadMinutes: courierAhead,
    deliveryLegMinutes: leg,
    elapsedSincePlacedMinutes: minutesSince(a.createdAt) ?? undefined,
  });
}
