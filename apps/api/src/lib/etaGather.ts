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

// ─── SENSOR-BUS §1.1: promised_window (frozen) + live_eta (mutable) synthesis ────────────────
// ESTOP-1 split (ADR-0009 v4): the synthesised window is persisted onto the order so the customer
// reads a live, collapsing estimate (live_eta_*) while §8 measurement reads the frozen first promise
// (promised_window_*). Both bounds only — range-never-point at the schema shape — and value-level via
// the cap-last clamp below.

export interface WindowCaps {
  /** locations.eta_cap_min — absolute ceiling on the high bound. */
  etaCapMin: number;
  /** locations.min_window_width_min — the band is never narrower than this (range-never-point). */
  minWindowWidthMin: number;
}

/**
 * R3-M1 cap-last clamp: clamp `lo` FIRST (so the cap survives the width floor) → apply the width
 * floor → clamp `hi` to eta_cap LAST so the cap is truly absolute and `hi` can never exceed it
 * (the v3 floor-after-cap bug let lo=92→hi=97>90). Total: every degenerate input maps to a valid
 * {1 ≤ lo < hi ≤ max(cap, lo+1)} band — never a point, never below 1, never above the cap.
 */
export function clampWindow(lowMin: number, highMin: number, caps: WindowCaps): { loMin: number; hiMin: number } {
  const cap = Number.isFinite(caps.etaCapMin) && caps.etaCapMin > 0 ? Math.round(caps.etaCapMin) : 90;
  const width = Math.max(1, Number.isFinite(caps.minWindowWidthMin) ? Math.round(caps.minWindowWidthMin) : 10);
  let lo = Math.max(1, Math.round(num(lowMin) ?? 1));
  let hi = Math.max(lo, Math.round(num(highMin) ?? lo + width));
  // clamp lo first — leave room for the width floor under the cap.
  if (lo > cap - width) lo = Math.max(1, cap - width);
  // width floor (range-never-point, value-level).
  if (hi < lo + width) hi = lo + width;
  // eta_cap LAST — absolute ceiling (R3-M1).
  if (hi > cap) hi = cap;
  // belt-and-braces: never collapse to a point even when the cap is below lo+width.
  if (hi < lo + 1) hi = lo + 1;
  return { loMin: lo, hiMin: hi };
}

const SYNTH_TERMINAL = ['DELIVERED', 'REJECTED', 'CANCELLED'];

/**
 * Synthesise + persist the order's ETA window at a status transition. Frozen promised_window_* is
 * written exactly once at CONFIRMED (guarded `WHERE promised_window_lo_min IS NULL` so the DB
 * set-once trigger never raises); mutable live_eta_* is recomputed on EVERY stage with the width
 * floor + cap-last clamp. Best-effort & total: any read hiccup degrades inside gatherOrderEtaRange,
 * and the caller wraps this in a SAVEPOINT so a synthesis failure NEVER fails the status transition
 * (observe-don't-control, brief §0.1). No-op for terminal statuses (no ETA past delivery).
 */
export async function synthesizeAndPersistEtaWindow(
  client: Queryable,
  orderId: string,
  newStatus: string,
): Promise<{ loMin: number; hiMin: number } | null> {
  if (SYNTH_TERMINAL.includes(newStatus)) return null;

  const r = await client.query(
    `SELECT o.location_id, o.created_at::text AS created_at, o.preparing_at::text AS preparing_at,
            o.delivery_lat, o.delivery_lng, o.promised_window_lo_min,
            l.lat AS loc_lat, l.lng AS loc_lng,
            COALESCE(l.eta_cap_min, 90)          AS eta_cap_min,
            COALESCE(l.min_window_width_min, 10) AS min_window_width_min,
            ca.courier_id, ca.status AS assignment_status,
            cp.lat AS courier_lat, cp.lng AS courier_lng
       FROM orders o
       JOIN locations l ON l.id = o.location_id
       LEFT JOIN courier_assignments ca ON ca.order_id = o.id AND ca.status IN ('accepted', 'picked_up')
       LEFT JOIN LATERAL (
         SELECT lat, lng FROM courier_positions
          WHERE courier_id = ca.courier_id ORDER BY recorded_at DESC LIMIT 1
       ) cp ON true
      WHERE o.id = $1`,
    [orderId],
  );
  const g = r.rows[0];
  if (!g) return null;

  const range = await gatherOrderEtaRange(client, {
    orderId,
    status: newStatus,
    locationId: g.location_id,
    createdAt: g.created_at,
    preparingAt: g.preparing_at,
    deliveryLat: g.delivery_lat,
    deliveryLng: g.delivery_lng,
    locationLat: g.loc_lat,
    locationLng: g.loc_lng,
    courierId: g.courier_id ?? null,
    assignmentStatus: g.assignment_status ?? null,
    courierLat: g.courier_lat,
    courierLng: g.courier_lng,
  });
  if (!range) return null;

  const { loMin, hiMin } = clampWindow(range.lowMin, range.highMin, {
    etaCapMin: Number(g.eta_cap_min),
    minWindowWidthMin: Number(g.min_window_width_min),
  });

  // Frozen promise: set ONCE at confirm. The `IS NULL` guard makes a re-confirm a no-op so the
  // BEFORE-UPDATE set-once trigger (on the promised_window_* pair) can never raise.
  if (newStatus === 'CONFIRMED') {
    await client.query(
      `UPDATE orders SET promised_window_lo_min = $2, promised_window_hi_min = $3
        WHERE id = $1 AND promised_window_lo_min IS NULL`,
      [orderId, loMin, hiMin],
    );
  }
  // Live channel: always rewritten — never touches the frozen pair, so the trigger is never armed.
  await client.query(
    `UPDATE orders SET live_eta_lo_min = $2, live_eta_hi_min = $3 WHERE id = $1`,
    [orderId, loMin, hiMin],
  );

  return { loMin, hiMin };
}
