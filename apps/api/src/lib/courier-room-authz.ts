import type { Pool, PoolClient } from 'pg';

/**
 * ADR-0013 (courier-realtime-authz) — the single source of truth for "may this courier touch
 * this order?", shared by the WebSocket subscribe gate + the fan-out relay guard
 * (apps/api/src/websocket.ts, courier-relay-guard.ts) AND the order-messages REST routes
 * (apps/api/src/routes/order-messages.ts). One predicate so the surfaces cannot drift (breaker NEW-E).
 *
 * `courier_assignments` is FORCE ROW LEVEL SECURITY keyed on `app.current_tenant`, so the
 * predicate MUST run inside an explicit BEGIN…COMMIT tx that sets the tenant GUC (the
 * assignments.ts:80 shape — a bare `set_config(...,true)` under autocommit dies before the
 * SELECT). This makes the gate correct under BOTH BYPASSRLS (today) and the branch's
 * NOBYPASSRLS hardening — order-independent of B3.
 *
 * Tri-state (Breaker H1/L2 + NEW-A): the read NEVER throws. A connect/query failure is
 * `UNAVAILABLE` (retryable — a pool blip must not be read as "no binding" and fleet-deny);
 * a clean 0-row result is `DENY` (a real negative — fail CLOSED). Callers map:
 *   - WS subscribe:  ALLOW → admit · DENY → Forbidden · UNAVAILABLE → retryable soft error (no close).
 *   - REST routes:   ALLOW → proceed · DENY/UNAVAILABLE → fail closed (404/503).
 *   - fan-out relay: ALLOW → relay · DENY → evict + binding_revoked · UNAVAILABLE → withhold + ceiling.
 */

export type AuthzVerdict = 'ALLOW' | 'DENY' | 'UNAVAILABLE';

// Read access (WS subscribe + REST read + relay): the offer-handshake courier must be able to view
// the order to decide accept/decline, so `offered` is included.
export const BINDING_READ_STATUSES = ['offered', 'assigned', 'accepted', 'picked_up'] as const;
// Send access (REST cu_*/cc_* message emit): an offered-but-not-yet-accepted courier may read,
// not yet speak in the customer thread.
export const BINDING_SEND_STATUSES = ['assigned', 'accepted', 'picked_up'] as const;

async function courierBindingVerdict(
  db: Pool,
  courierSub: string,
  activeLocationId: string,
  orderId: string,
  statuses: readonly string[],
): Promise<AuthzVerdict> {
  // connect() is INSIDE the try so a pool-exhaustion / connect failure is caught and mapped to
  // UNAVAILABLE (never throws out to the caller — a throw on the WS path → ws.close(1008) →
  // reconnect storm; breaker H1).
  let client: PoolClient | undefined;
  try {
    client = await db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [activeLocationId]);
      const r = await client.query(
        `SELECT 1 FROM courier_assignments
           WHERE order_id = $1 AND courier_id = $2 AND status = ANY($3::text[]) LIMIT 1`,
        [orderId, courierSub, statuses as string[]],
      );
      await client.query('COMMIT');
      return (r.rowCount ?? 0) > 0 ? 'ALLOW' : 'DENY';
    } catch (err) {
      try { await client.query('ROLLBACK'); } catch { /* already broken */ }
      throw err;
    }
  } catch (err: any) {
    // connect OR query failure → UNAVAILABLE (retryable), NOT a real negative.
    console.error('[courier-authz] binding query unavailable:', err?.message);
    return 'UNAVAILABLE';
  } finally {
    if (client) client.release();
  }
}

/** Tri-state READ verdict for an order (room subscribe / message-history / fan-out relay). */
export function courierReadVerdict(db: Pool, courierSub: string, activeLocationId: string | undefined, orderId: string): Promise<AuthzVerdict> {
  if (!orderId || !activeLocationId) return Promise.resolve('DENY');
  return courierBindingVerdict(db, courierSub, activeLocationId, orderId, BINDING_READ_STATUSES);
}

/** Tri-state SEND verdict for an order's thread (REST message emit). */
export function courierSendVerdict(db: Pool, courierSub: string, activeLocationId: string | undefined, orderId: string): Promise<AuthzVerdict> {
  if (!orderId || !activeLocationId) return Promise.resolve('DENY');
  return courierBindingVerdict(db, courierSub, activeLocationId, orderId, BINDING_SEND_STATUSES);
}

/**
 * WS room verdict for couriers. `location:*` is the owner dashboard feed → DENY outright (zero DB);
 * couriers may watch only `courier:<self>` (handled by the caller) and `order:<id>` they hold a live
 * READ binding for. Returns DENY for any non-order room.
 */
export function courierRoomVerdict(db: Pool, courierSub: string, activeLocationId: string | undefined, room: string): Promise<AuthzVerdict> {
  if (room.startsWith('order:')) {
    const orderId = room.split(':')[1] ?? '';
    return courierReadVerdict(db, courierSub, activeLocationId, orderId);
  }
  // `location:*` and everything else → no legitimate courier room.
  return Promise.resolve('DENY');
}

// ── Fail-closed boolean wrappers (REST tenant-isolation checks) ──────────────────────────────────
// REST treats UNAVAILABLE as a closed door (404 — indistinguishable from "not your order"); the
// retryable distinction is load-bearing only on the WS paths (admission + fan-out revalidation).

/** Does the courier hold a live READ binding (REST history/read)? UNAVAILABLE → false (fail closed). */
export async function courierCanReadOrder(db: Pool, courierSub: string, activeLocationId: string | undefined, orderId: string): Promise<boolean> {
  return (await courierReadVerdict(db, courierSub, activeLocationId, orderId)) === 'ALLOW';
}

/** Does the courier hold a live SEND binding (REST message emit)? UNAVAILABLE → false (fail closed). */
export async function courierCanSendOrder(db: Pool, courierSub: string, activeLocationId: string | undefined, orderId: string): Promise<boolean> {
  return (await courierSendVerdict(db, courierSub, activeLocationId, orderId)) === 'ALLOW';
}

/** Boolean WS room gate (legacy admission shim). Prefer courierRoomVerdict for the retryable signal. */
export async function courierCanAccessRoom(db: Pool, courierSub: string, activeLocationId: string | undefined, room: string): Promise<boolean> {
  return (await courierRoomVerdict(db, courierSub, activeLocationId, room)) === 'ALLOW';
}
