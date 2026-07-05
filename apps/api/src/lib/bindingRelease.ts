import type { MessageBus } from '@deliveryos/platform';
import { updateOrderStatus } from './orderStatusService.js';

// deliver v2 (D1 / R2-2 / R3-2 / R4-3/4): the SHARED courier-binding-release rail used by BOTH /cancel
// (accept-regret, time-gated) and /abort (en-route, no gate). Centralizing it in lib/ (next to
// deliveryCompletion) kills the per-path drift that re-created the C-2 trap on /cancel (a raw exit that
// bypassed updateOrderStatus + hand-published a false ORDER_CANCELLED).
//
// Runs INSIDE the caller's tx (no BEGIN/COMMIT). It (1) terminalizes the binding + frees the shift
// UNCONDITIONALLY (abort/cancel always free the assignment), then (2) takes an order-side action GUARDED on
// the LOCKED order status — updateOrderStatus is invoked ONLY from IN_DELIVERY (the one state with a legal
// widened exit), so it can never throw an illegal/same-status transition; the flag-ON pre-pickup case takes
// the no-transition branch (drop the mirror + re-offer, converging with the decline path).
//
// Honest signal (ADR-dispatch-recovery Q4): returns `requeued` — the journal INSERT is a
// RE-ENQUEUE, not a re-offer. The genuine re-offer claim (ORDER_ASSIGNMENT_CREATED) is emitted
// only by the dispatch worker once the pumped journal row lands on a real new binding.
export async function releaseBindingAndReoffer(
  client: any,
  args: { assignmentId: string; orderId: string; shiftId: string; asgStatus: string; ordStatus: string; locationId: string; reason: string },
  { messageBus }: { messageBus: MessageBus },
): Promise<{ requeued: boolean }> {
  const { assignmentId, orderId, shiftId, asgStatus, ordStatus, locationId, reason } = args;

  await client.query(
    `UPDATE courier_assignments SET status = 'cancelled', cancelled_at = now(), cancellation_reason = $1 WHERE id = $2`,
    [reason, assignmentId],
  );
  await client.query(`UPDATE courier_shifts SET status = 'available' WHERE id = $1`, [shiftId]);

  const reEnqueue = async () => {
    await client.query(`UPDATE orders SET courier_id = NULL WHERE id = $1`, [orderId]);
    await client.query(
      `INSERT INTO courier_dispatch_queue (order_id, location_id, enqueued_at) VALUES ($1,$2,now())
       ON CONFLICT (order_id) DO UPDATE SET attempts = courier_dispatch_queue.attempts + 1`,
      [orderId, locationId],
    );
  };

  if (ordStatus === 'IN_DELIVERY' && asgStatus === 'picked_up') {
    // Food is out with the failed courier → honest terminal (the no-cash-equivalent CANCELLED).
    await updateOrderStatus(client, orderId, locationId, 'CANCELLED', { messageBus, comment: reason });
    return { requeued: false };
  }
  if (ordStatus === 'IN_DELIVERY') {
    // Legacy flag-OFF force-IN_DELIVERY, pre-pickup → revert to assignable (food still at venue), clear the
    // mirror (updateOrderStatus does not touch orders.courier_id), and re-offer.
    await updateOrderStatus(client, orderId, locationId, 'READY', { messageBus, comment: reason });
    await reEnqueue();
    return { requeued: true };
  }
  // flag-ON accept — the order never advanced (CONFIRMED/PREPARING/READY) → NO status transition (forcing one
  // would throw). Drop the binding + re-offer, converging with the decline path.
  await reEnqueue();
  return { requeued: true };
}
