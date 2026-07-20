import type { PoolClient } from 'pg';
import type { MessageBus } from '@deliveryos/platform';
import { loadEnv } from '@deliveryos/config';
import { shiftChannel } from './registry.js';

export async function openShift(
  client: PoolClient,
  courierId: string,
  locationId: string,
  opts: { messageBus: MessageBus }
): Promise<{ shiftId: string; status: string; startedAt: string }> {
  const env = loadEnv();

  // 1. Read the courier's shift for today with lock to prevent race conditions
  const shiftRes = await client.query(
    `SELECT id, status, started_at
     FROM courier_shifts
     WHERE courier_id = $1 AND location_id = $2 AND DATE(started_at) = CURRENT_DATE
     ORDER BY started_at DESC
     LIMIT 1
     FOR UPDATE`,
    [courierId, locationId]
  );

  let shiftId: string;
  let status: string;
  let startedAt: string;

  if ((shiftRes.rowCount ?? 0) > 0) {
    const shift = shiftRes.rows[0];
    // If the shift is already available or on_delivery, we just update it
    if (shift.status === 'available' || shift.status === 'on_delivery') {
      await client.query(
        `UPDATE courier_shifts
         SET status = 'available', ended_at = NULL, started_at = COALESCE(started_at, now()), last_heartbeat_at = now()
         WHERE id = $1`,
        [shift.id]
      );
      shiftId = shift.id;
      status = 'available';
      startedAt = new Date().toISOString();
    } else {
      // If the shift is in another state, we cannot open it
      throw { statusCode: 400, error: `Cannot open shift in status ${shift.status}` };
    }
  } else {
    // No shift for today, create a new one
    const insertRes = await client.query(
      `INSERT INTO courier_shifts (courier_id, location_id, status, started_at, last_heartbeat_at)
       VALUES ($1, $2, 'available', now(), now())
       RETURNING id`,
      [courierId, locationId]
    );
    shiftId = insertRes.rows[0].id;
    status = 'available';
    startedAt = new Date().toISOString();
  }

  // 2. Broadcast via MessageBus
  await opts.messageBus.publish(shiftChannel(courierId), {
    type: 'shift.opened',
    shiftId,
    status,
    startedAt,
    locationId
  });

  return { shiftId, status, startedAt };
}