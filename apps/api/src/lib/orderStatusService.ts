import type { PoolClient } from 'pg';
import { assertTransition, type OrderStatus } from '@deliveryos/domain';
import type { MessageBus } from '@deliveryos/platform';
import { BUS_CHANNELS, orderChannel, dashboardChannel } from './registry.js';

// ORDER-TRACKING: per-transition timestamp column for each status the machine
// stamps. Additive instrumentation only \u2014 the transition/guard logic in
// assertTransition() is untouched; this just records WHEN each step happened.
// CONFIRMED/DELIVERED keep their pre-existing dedicated UPDATE branches below.
const STATUS_AT_COLUMN: Partial<Record<OrderStatus, string>> = {
  CONFIRMED: 'confirmed_at',
  PREPARING: 'preparing_at',
  READY: 'ready_at',
  IN_DELIVERY: 'in_delivery_at',
  DELIVERED: 'delivered_at',
  PICKED_UP: 'picked_up_at',
};

async function fetchOrderDelta(client: PoolClient, orderId: string) {
  // P0-3 claim-check: NO item names (dietary/medical-adjacent PII) on the bus \u2014 only
  // itemCount + non-PII status fields. The dashboard reads item names from the
  // authenticated /owner/orders endpoint, not from the realtime payload.
  const res = await client.query(`
    SELECT o.id, o.status, o.total, o.created_at, o.location_id, loc.currency_code,
      o.confirmed_at, o.preparing_at, o.ready_at, o.in_delivery_at, o.delivered_at, o.picked_up_at,
      (SELECT count(*) FROM order_items oi WHERE oi.order_id = o.id)::int as item_count
    FROM orders o
    LEFT JOIN locations loc ON loc.id = o.location_id
    WHERE o.id = $1
  `, [orderId]);
  const row = res.rows[0];
  if (!row) return null;
  return {
    orderId: row.id,
    locationId: row.location_id,
    status: row.status,
    total: row.total,
    currency: row.currency_code || 'ALL',
    createdAt: row.created_at,
    shortId: '#' + row.id.substring(0, 4).toUpperCase(),
    itemCount: row.item_count || 0,
    // Additive per-transition timestamps (nullable until that step is reached).
    confirmedAt: row.confirmed_at,
    preparingAt: row.preparing_at,
    readyAt: row.ready_at,
    inDeliveryAt: row.in_delivery_at,
    deliveredAt: row.delivered_at,
    pickedUpAt: row.picked_up_at,
  };
}

export async function updateOrderStatus(
  client: PoolClient,
  orderId: string,
  locationId: string,
  newStatus: OrderStatus,
  // ORDER-TRACKING: `comment` is additive (optional) — a human-readable reason
  // (rejection/cancellation) recorded on order_status_history. No `notify`.
  opts: { messageBus: MessageBus; comment?: string | null }
): Promise<void> {
  // 1. Read current status
  const cur = await client.query(
    `SELECT id, status, location_id FROM orders WHERE id = $1`,
    [orderId]
  );

  if (!cur.rowCount || cur.rowCount === 0) {
    throw { statusCode: 404, error: 'Order not found' };
  }

  const currentStatus: string = cur.rows[0].status;

  // 2. State machine validation (before SQL)
  try {
    assertTransition(currentStatus as OrderStatus, newStatus);
  } catch (e: unknown) {
    const err = e as Error;
    if (err.name === 'IllegalTransitionError' || err.name === 'ScaffoldDisabledError') {
      throw { statusCode: 400, error: err.message, code: err.name };
    }
    if (err.name === 'SameStatusError') {
      throw { statusCode: 400, error: err.message, code: err.name };
    }
    throw e;
  }

  // 3. Status-guarded UPDATE (anti-race)
  let res;
  if (newStatus === 'CONFIRMED') {
    res = await client.query(
      `UPDATE orders SET status = $1, confirmed_at = now(), timeout_at = NULL
       WHERE id = $2 AND status = $3 RETURNING id`,
      [newStatus, orderId, currentStatus]
    );
  } else if (newStatus === 'DELIVERED') {
    // Stamp delivered_at on the ORDER. /verify, /owner/orders and the analytics
    // avg-delivery-time metric all read orders.delivered_at, which was otherwise
    // never written (only courier_assignments.delivered_at was set) — so the
    // metric was permanently 0 and the order's deliveredAt always NULL.
    res = await client.query(
      `UPDATE orders SET status = $1, delivered_at = now(), timeout_at = NULL
       WHERE id = $2 AND status = $3 RETURNING id`,
      [newStatus, orderId, currentStatus]
    );
  } else {
    // ORDER-TRACKING: additively stamp the matching *_at for this transition
    // (PREPARING/READY/IN_DELIVERY/PICKED_UP). Column comes from a fixed
    // allowlist (STATUS_AT_COLUMN) — never from user input — so it is safe to
    // interpolate. REJECTED/CANCELLED have no column and fall through unchanged.
    const stampCol = STATUS_AT_COLUMN[newStatus];
    const setStamp = stampCol ? `, ${stampCol} = now()` : '';
    res = await client.query(
      `UPDATE orders SET status = $1, timeout_at = NULL${setStamp}
       WHERE id = $2 AND status = $3 RETURNING id`,
      [newStatus, orderId, currentStatus]
    );
  }

  if (!res.rowCount || res.rowCount === 0) {
    throw { statusCode: 409, error: 'Order status already changed', code: 'CONFLICT' };
  }

<<<<<<< Updated upstream
=======
  // deliver v2 (R2-3 shared invariant — ADR-deliver-v2-cash-as-proof + offer-sweep-cancel addendum):
  // NO order leaves to a terminal/assignable downgrade without its active courier assignment terminalized
  // in the SAME tx. Folded centrally here so EVERY caller (owner no-show signals.ts, owner PATCH orders.ts,
  // courier cancel/abort, reassign revert, dispatch-grace worker) is covered, present and future.
  // Addendum widening: runs on ANY newStatus==='CANCELLED' (not only from IN_DELIVERY) so a widened
  // CONFIRMED/PREPARING/READY→CANCELLED edge can never strand a binding; plus the IN_DELIVERY→READY revert.
  // Idempotent: an already-terminal row (incl. a just-set 'delivered' or a completeDelivery 'cancelled') is
  // a no-op, and a PENDING→CANCELLED with no active binding matches 0 rows. Cash-safe: terminalizing writes
  // NO courier_cash_ledger 'hold' (the hold is written only by completeDelivery at DELIVERED).
  // 'delivered' ∉ {CANCELLED,READY} so a delivered row is never reverted.
  if (newStatus === 'CANCELLED' || (currentStatus === 'IN_DELIVERY' && newStatus === 'READY')) {
    await client.query(
      `WITH freed AS (
         UPDATE courier_assignments SET status = 'cancelled', cancelled_at = now(),
                cancellation_reason = COALESCE($2, 'order_' || lower($3))
          WHERE order_id = $1 AND status IN ('offered','assigned','accepted','picked_up')
         RETURNING shift_id)
       UPDATE courier_shifts SET status = 'available'
        WHERE id IN (SELECT shift_id FROM freed WHERE shift_id IS NOT NULL)`,
      [orderId, opts.comment ?? null, newStatus],
    );
  }

>>>>>>> Stashed changes
  // ORDER-TRACKING: audit-trail row with optional reason. Best-effort — a
  // history-insert failure must never roll back the (already-applied) status
  // change. Wrapped in a SAVEPOINT so a failed insert (e.g. RLS denial) cannot
  // poison the caller's surrounding transaction. RLS-scoped by location_id
  // (FORCE row security on order_status_history).
  try {
    await client.query('SAVEPOINT order_status_history_ins');
    await client.query(
      `INSERT INTO order_status_history (order_id, location_id, from_status, to_status, actor, comment)
       VALUES ($1, $2, $3, $4, $5, $6)`,
      [orderId, cur.rows[0].location_id, currentStatus, newStatus, 'system:updateOrderStatus', opts.comment ?? null]
    );
    await client.query('RELEASE SAVEPOINT order_status_history_ins');
  } catch {
    // Audit trail is advisory; the canonical state lives on orders.status.
    try { await client.query('ROLLBACK TO SAVEPOINT order_status_history_ins'); } catch { /* no tx */ }
  }

  // 4. Broadcast via MessageBus
  // ORDER-TRACKING: include the just-stamped *_at field additively so the
  // customer page can light up the matching step from the live delta without
  // a refetch. `statusAtField` names the camelCase key the FE merges (or null
  // for REJECTED/CANCELLED, which have no timestamp column).
  const stampedCol = STATUS_AT_COLUMN[newStatus];
  const stampedField = stampedCol
    ? stampedCol.replace(/_([a-z])/g, (_, c) => c.toUpperCase())
    : null;
  const nowIso = new Date().toISOString();
  await opts.messageBus.publish(orderChannel(orderId), {
    type: 'order.status',
    orderId,
    status: newStatus,
    locationId: cur.rows[0].location_id,
    timestamp: nowIso,
    statusAtField: stampedField,
    statusAt: stampedField ? nowIso : null,
  });

  const dbLocationId = cur.rows[0].location_id;

  // Forward to dashboard room for live owner dashboard
  // Uses 'order.status' type — FE merges by id, no full GET needed
  if (dbLocationId) {
    const delta = await fetchOrderDelta(client, orderId);
    if (delta) {
      await opts.messageBus.publish(dashboardChannel(dbLocationId), {
        type: 'order.status',
        data: { ...delta, statusUpdatedAt: new Date().toISOString() },
      });
    }
  }

  // 5. Publish lifecycle event for notification fan-out
  if (newStatus === 'CONFIRMED' && dbLocationId) {
    await opts.messageBus.publish(BUS_CHANNELS.ORDER_CONFIRMED, { orderId, locationId: dbLocationId });
  } else if (newStatus === 'REJECTED' && dbLocationId) {
    await opts.messageBus.publish(BUS_CHANNELS.ORDER_REJECTED, { orderId, locationId: dbLocationId });
  }
}