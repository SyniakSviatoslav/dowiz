import type { PoolClient } from 'pg';
import type { MessageBus } from '@deliveryos/platform';
import { loadEnv } from '@deliveryos/config';
import { BUS_CHANNELS } from './registry.js';

export async function acceptCourierAssignment(
  client: PoolClient,
  assignmentId: string,
  locationId: string,
  courierId: string,
  opts: { messageBus: MessageBus }
): Promise<{ orderId: string }> {
  const env = loadEnv();
  const acceptWindowMs = parseInt((env as any).COURIER_ACCEPT_WINDOW_MS || '30000', 10);

  // 1. Read the assignment with lock to prevent race conditions. MUST scope by
  // courier_id (cross-courier IDOR fix — ADR courier-assignment-idor): RLS isolates
  // only by location, so without this predicate any courier in the same location
  // could accept (hijack) another courier's assignment. Mirrors reject/picked-up/
  // delivered/cancel, which all inline `AND courier_id = $2`.
  const assignmentRes = await client.query(
    `SELECT order_id, assigned_at, status, courier_id
     FROM courier_assignments
     WHERE id = $1 AND courier_id = $2
     FOR UPDATE`,
    [assignmentId, courierId]
  );

  if (assignmentRes.rowCount === 0) {
    throw { statusCode: 404, error: 'Assignment not found' };
  }

  const assignment = assignmentRes.rows[0];

  // 2. Check if the assignment is in the correct state
  if (assignment.status !== 'assigned') {
    throw { statusCode: 400, error: 'Assignment is not in assigned state' };
  }

  // 3. Check if the acceptance window has expired
  const elapsedMs = Date.now() - new Date(assignment.assigned_at).getTime();
  if (elapsedMs > acceptWindowMs) {
    throw { statusCode: 410, error: 'Acceptance window expired' };
  }

  // 4. Update the assignment to accepted (scoped by courier_id — defense in depth)
  await client.query(
    `UPDATE courier_assignments
     SET status = 'accepted', accepted_at = now()
     WHERE id = $1 AND courier_id = $2`,
    [assignmentId, courierId]
  );

  // 5. Broadcast via MessageBus
  await opts.messageBus.publish(BUS_CHANNELS.ORDER_COURIER_ACCEPTED, {
    orderId: assignment.order_id,
    locationId,
    courierId: assignment.courier_id
  });

  return { orderId: assignment.order_id };
}