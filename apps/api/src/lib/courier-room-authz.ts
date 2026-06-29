import type { Pool, PoolClient } from 'pg';

/**
 * ADR-0013 (courier-realtime-authz) — the single source of truth for "may this courier touch
 * this order?", shared by the WebSocket subscribe gate (apps/api/src/websocket.ts) AND the
 * order-messages REST routes (apps/api/src/routes/order-messages.ts). One helper so the two
 * surfaces cannot drift (breaker NEW-E).
 *
 * `courier_assignments` is FORCE ROW LEVEL SECURITY keyed on `app.current_tenant`, so the
 * predicate MUST run inside an explicit BEGIN…COMMIT tx that sets the tenant GUC (the
 * assignments.ts:80 shape — a bare `set_config(...,true)` under autocommit dies before the
 * SELECT). This makes the gate correct under BOTH BYPASSRLS (today) and the branch's
 * NOBYPASSRLS hardening — order-independent of B3. Fail CLOSED on any error.
 */

// Read access (WS subscribe + REST read): the offer-handshake courier must be able to view the
// order to decide accept/decline, so `offered` is included.
export const BINDING_READ_STATUSES = ['offered', 'assigned', 'accepted', 'picked_up'] as const;
// Send access (REST cu_*/cc_* message emit): an offered-but-not-yet-accepted courier may read,
// not yet speak in the customer thread.
export const BINDING_SEND_STATUSES = ['assigned', 'accepted', 'picked_up'] as const;

async function courierHasBinding(
  db: Pool,
  courierSub: string,
  activeLocationId: string,
  orderId: string,
  statuses: readonly string[],
): Promise<boolean> {
  // connect() is INSIDE the try so a pool-exhaustion / connect failure fails CLOSED (denies),
  // never throws out to the caller (breaker fail-closed requirement).
  let client: PoolClient | undefined;
  try {
    client = await db.connect();
    await client.query('BEGIN');
    await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [activeLocationId]);
    const r = await client.query(
      `SELECT 1 FROM courier_assignments
         WHERE order_id = $1 AND courier_id = $2 AND status = ANY($3::text[]) LIMIT 1`,
      [orderId, courierSub, statuses as string[]],
    );
    await client.query('COMMIT');
    return (r.rowCount ?? 0) > 0;
  } catch (err: any) {
    if (client) { try { await client.query('ROLLBACK'); } catch { /* already broken */ } }
    console.error('[courier-authz] binding query failed:', err?.message);
    return false; // fail closed
  } finally {
    if (client) client.release();
  }
}

/** Does the courier hold a live binding allowing them to READ this order (room / message thread)? */
export function courierCanReadOrder(db: Pool, courierSub: string, activeLocationId: string | undefined, orderId: string): Promise<boolean> {
  if (!orderId || !activeLocationId) return Promise.resolve(false);
  return courierHasBinding(db, courierSub, activeLocationId, orderId, BINDING_READ_STATUSES);
}

/** Does the courier hold a binding allowing them to SEND in this order's thread? */
export function courierCanSendOrder(db: Pool, courierSub: string, activeLocationId: string | undefined, orderId: string): Promise<boolean> {
  if (!orderId || !activeLocationId) return Promise.resolve(false);
  return courierHasBinding(db, courierSub, activeLocationId, orderId, BINDING_SEND_STATUSES);
}

/**
 * WS room gate for couriers. `location:*` is the owner dashboard feed → denied outright; couriers
 * may watch only `courier:<self>` (handled by the caller) and `order:<id>` they hold a binding for.
 * Pure room-routing here; the DB read is delegated to courierCanReadOrder. Returns false for any
 * non-order room (the caller handles `courier:<self>` before delegating here).
 */
export function courierCanAccessRoom(db: Pool, courierSub: string, activeLocationId: string | undefined, room: string): Promise<boolean> {
  if (room.startsWith('order:')) {
    const orderId = room.split(':')[1] ?? '';
    return courierCanReadOrder(db, courierSub, activeLocationId, orderId);
  }
  // `location:*` and everything else → no legitimate courier room.
  return Promise.resolve(false);
}
