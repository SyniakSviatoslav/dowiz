import type { PoolClient } from 'pg';
import type { MessageBus } from '@deliveryos/platform';
import { updateOrderStatus } from './orderStatusService.js';

// deliver v2 (Cash-as-Proof) — the SINGLE completion primitive (R2-1). BOTH delivery-completion paths (the
// courier app `delivered` handler and the owner-proxy `/deliver` handler) call this, so the cash-as-proof
// HOLD + `payment_outcome` + the immutable `delivery_trace` crumb are structurally guaranteed on EVERY
// delivered order (the owner-proxy path previously wrote none). Runs INSIDE the caller's transaction (takes a
// client; the caller owns BEGIN/COMMIT + the status-guarded SELECT … FOR UPDATE of the assignment row).
// ADR-deliver-v2-cash-as-proof.

export type PaymentOutcome =
  | 'paid_full'
  | 'delivered_prepaid'   // C1: crypto-prepaid delivery — no cash, no till-hold (precond payment_status='paid')
  | 'refused_goods'
  | 'refused_payment'
  | 'customer_cancelled_on_door';

export class CompletionError extends Error {
  constructor(readonly code: string, readonly meta?: Record<string, unknown>) {
    super(code);
    this.name = 'CompletionError';
  }
}

export interface CompleteDeliveryArgs {
  assignmentId: string;
  orderId: string;
  locationId: string;
  courierId: string;
  shiftId: string | null;
  total: number;
  paymentOutcome: PaymentOutcome;
  cashAmount?: number | null;
  // passive crumbs (never thresholded — recorded, not gated):
  gpsLat?: number | null;
  gpsLng?: number | null;
  routeDistanceM?: number | null;
  expectedDeliveryMin?: number | null;
  nameSnapshot?: unknown;
  priceSnapshot?: number | null;
}

/**
 * Complete a delivery from a single tapped outcome. `paid_full` → assignment 'delivered', order DELIVERED,
 * ledger 'hold' (the cash bond = till accountability). The no-cash tail (refused / cancelled_on_door) →
 * assignment 'cancelled', order CANCELLED (so the customer never sees "Delivered" for refused food), NO hold.
 * Server-authoritative coherence: paid_full REQUIRES cash_amount === total; `paid_partial`/`pending` are
 * forbidden as delivered outcomes (caller's Zod). Persists payment_outcome to orders + delivery_trace.
 * Idempotent: trace ON CONFLICT(order_id), ledger ON CONFLICT(order_id,type).
 */
export async function completeDelivery(
  client: PoolClient,
  args: CompleteDeliveryArgs,
  opts: { messageBus: MessageBus },
): Promise<{ orderStatus: 'DELIVERED' | 'CANCELLED' }> {
  const isPaidFull = args.paymentOutcome === 'paid_full';
  // C1 (ADR-0017): prepaid (crypto) delivery — the money is already in via the payments ledger, so completion
  // collects NO cash and writes NO courier 'hold' (a paid order must never create a till-debt).
  const isPrepaid = args.paymentOutcome === 'delivered_prepaid';

  // Coherence: paid_full requires the full cash in hand (no-partial-handover rule). 422 before any mutation.
  if (isPaidFull && args.cashAmount !== args.total) {
    throw new CompletionError('CASH_AMOUNT_MISMATCH', { expected: args.total });
  }
  // Prepaid precondition: the order must actually be paid (the webhook flipped payment_status='paid') — else
  // 409, never silently mark a not-yet-confirmed crypto order delivered.
  if (isPrepaid) {
    const ps = await client.query(`SELECT payment_status FROM orders WHERE id = $1`, [args.orderId]);
    if (ps.rows[0]?.payment_status !== 'paid') {
      throw new CompletionError('PREPAID_NOT_PAID', {});
    }
  }

  const cashCollected = isPaidFull;                       // prepaid collects no cash
  const isDelivered = isPaidFull || isPrepaid;
  const assignmentStatus = isDelivered ? 'delivered' : 'cancelled';
  const orderStatus: 'DELIVERED' | 'CANCELLED' = isDelivered ? 'DELIVERED' : 'CANCELLED';

  // 1. Terminalize the assignment + free the shift.
  await client.query(
    `UPDATE courier_assignments
        SET status = $1,
            delivered_at = CASE WHEN $1 = 'delivered' THEN now() ELSE delivered_at END,
            cancelled_at = CASE WHEN $1 = 'cancelled' THEN now() ELSE cancelled_at END,
            cancellation_reason = CASE WHEN $1 = 'cancelled' THEN $2 ELSE cancellation_reason END,
            cash_collected = $3, cash_amount = $4
      WHERE id = $5`,
    [assignmentStatus, args.paymentOutcome, cashCollected, cashCollected ? args.cashAmount : null, args.assignmentId],
  );
  if (args.shiftId) {
    await client.query(`UPDATE courier_shifts SET status = 'available' WHERE id = $1`, [args.shiftId]);
  }

  // 2. Canonical order transition (DELIVERED or the no-cash CANCELLED tail) + WS. The central fold in
  //    updateOrderStatus is a no-op here (the assignment is already terminal above).
  await updateOrderStatus(client, args.orderId, args.locationId, orderStatus, {
    messageBus: opts.messageBus,
    comment: isDelivered ? undefined : args.paymentOutcome,
  });

  // 3. Persist the distinguishing crumb server-authoritatively (orders + immutable trace).
  await client.query(`UPDATE orders SET payment_outcome = $1 WHERE id = $2`, [args.paymentOutcome, args.orderId]);
  await client.query(
    `INSERT INTO delivery_trace
       (order_id, location_id, courier_id, total, delivered_at, route_distance_m, expected_delivery_min,
        payment_outcome, cash_amount, gps_lat, gps_lng, name_snapshot, price_snapshot)
     VALUES ($1,$2,$3,$4, now(), $5,$6,$7,$8,$9,$10,$11,$12)
     ON CONFLICT (order_id) DO NOTHING`,
    [
      args.orderId, args.locationId, args.courierId, args.total, args.routeDistanceM ?? null,
      args.expectedDeliveryMin ?? null, args.paymentOutcome, cashCollected ? args.cashAmount ?? null : null,
      args.gpsLat ?? null, args.gpsLng ?? null, args.nameSnapshot ?? null, args.priceSnapshot ?? null,
    ],
  );

  // 4. Cash-as-proof HOLD (paid_full only) — the courier's till-accountability bond until shift reconciliation.
  if (isPaidFull) {
    await client.query(
      `INSERT INTO courier_cash_ledger (courier_id, location_id, order_id, type, amount)
       VALUES ($1, $2, $3, 'hold', $4) ON CONFLICT (order_id, type) DO NOTHING`,
      [args.courierId, args.locationId, args.orderId, args.cashAmount],
    );
  }

  // 5. C2 (ADR-0017) — refused/cancelled prepaid refund obligation. The 'refund_due' insert that
  //    lived here moved into the L-A fold in updateOrderStatus (ADR-audit-fix-money §3.2): step 2
  //    above already recorded it in this same tx for the CANCELLED tail (single writer restored;
  //    the per-payment unique makes the transition race-safe). The owner sends the crypto back
  //    out-of-band → records 'refund_sent' → payment_status='refunded'.

  return { orderStatus };
}
