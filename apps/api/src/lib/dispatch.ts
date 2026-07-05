import type { MessageBus } from '@deliveryos/platform';
import { updateOrderStatus } from './orderStatusService.js';
import { dashboardChannel } from './registry.js';

// §5 / R2-1 / R3-2 — HONEST DISPATCH (the no-trap red-line F1), extracted so it is deterministically testable.
// Runs INSIDE the caller's tx (no BEGIN/COMMIT). For an IN_DELIVERY target on a delivery order it finds a
// courier BEFORE advancing the status: an order must NEVER reach IN_DELIVERY with no courier (that is an
// orphan with no recovery affordance). No courier → DO NOT advance (stay at the current status), report
// {dispatched:false,reason:'no_courier'}; the owner re-taps when a courier comes on shift. An order already
// carrying an active binding (incl 'offered' from the offer-handshake) → no double-bind. Flag-independent.
export async function attemptHonestDispatch(
  client: any,
  args: { orderId: string; locationId: string; currentStatus: string },
  { messageBus }: { messageBus: MessageBus },
): Promise<{ status: string; dispatched: boolean; reason?: string }> {
  const { orderId, locationId, currentStatus } = args;

  const bound = await client.query(
    `SELECT 1 FROM courier_assignments
     WHERE order_id = $1 AND status IN ('offered','assigned','accepted','picked_up') LIMIT 1`,
    [orderId],
  );
  if ((bound.rowCount ?? 0) > 0) {
    return { status: currentStatus, dispatched: false, reason: 'already_assigned' };
  }

  const availRes = await client.query(
    `SELECT c.id AS courier_id, cs.id AS shift_id
       FROM couriers c
       JOIN courier_locations cl ON cl.courier_id = c.id
       JOIN courier_shifts cs ON cs.courier_id = c.id
      WHERE cl.location_id = $1 AND c.status = 'active' AND cs.status = 'available'
        AND c.id NOT IN (
          SELECT courier_id FROM courier_assignments
          WHERE status IN ('offered','assigned','accepted','picked_up') AND courier_id IS NOT NULL
        )
      ORDER BY cs.last_heartbeat_at DESC NULLS LAST, c.id ASC
      LIMIT 1`,
    [locationId],
  );
  if ((availRes.rowCount ?? 0) === 0) {
    return { status: currentStatus, dispatched: false, reason: 'no_courier' };
  }

  // Courier found → NOW advance to IN_DELIVERY and assign, atomically.
  await updateOrderStatus(client, orderId, locationId, 'IN_DELIVERY', { messageBus });
  const { courier_id, shift_id } = availRes.rows[0];
  await client.query(
    `INSERT INTO courier_assignments (order_id, location_id, courier_id, shift_id, status, assigned_at)
     VALUES ($1, $2, $3, $4, 'assigned', now())`,
    [orderId, locationId, courier_id, shift_id],
  );
  await client.query(`UPDATE courier_shifts SET status = 'on_delivery' WHERE id = $1`, [shift_id]);
  await messageBus.publish(dashboardChannel(locationId), { type: 'assignment.created', orderId, courierId: courier_id });
  await messageBus.publish(`courier:${courier_id}`, { type: 'task_assigned', payload: { id: orderId, orderId, status: 'assigned', courierId: courier_id } });
  return { status: 'IN_DELIVERY', dispatched: true };
}
