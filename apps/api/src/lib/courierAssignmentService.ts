import type { PoolClient } from 'pg';
import type { MessageBus } from '@deliveryos/platform';
import { loadEnv } from '@deliveryos/config';

export async function acceptCourierAssignment(
  client: PoolClient,
  assignmentId: string,
  locationId: string,
  opts: { messageBus: MessageBus }
): Promise<void> {
  const env = loadEnv();
  const acceptWindowMs = parseInt((env as any).COURIER_ACCEPT_WINDOW_MS || '30000', 10);

  // 1. Read the assignment with lock to prevent race conditions
  const assignmentRes = await client.query(
    `SELECT order_id, assigned_at, status, courier_id
     FROM courier_assignments
     WHERE id = $1
     FOR UPDATE`,
    [assignmentId]
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

  // 4. Update the assignment to accepted
  await client.query(
    `UPDATE courier_assignments
     SET status = 'accepted', accepted_at = now()
     WHERE id = $1`,
    [assignmentId]
  );

  // 5. Broadcast via MessageBus
  await opts.messageBus.publish('order.courier_accepted', {
    orderId: assignment.order_id,
    locationId,
    courierId: assignment.courier_id
  });
}