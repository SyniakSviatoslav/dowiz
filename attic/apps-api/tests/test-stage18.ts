import { test } from 'node:test';
import assert from 'node:assert';
import { createSessionPool } from '@deliveryos/db';
import { encryptPII } from '../src/lib/pii-cipher.js';
import { MessageBus, PgMessageBus } from '@deliveryos/platform';
import WebSocket from 'ws';

test('Stage 18: Courier Dispatch & GPS', async (t) => {
  const pool = createSessionPool();
  const messageBus = new PgMessageBus();
  await messageBus.connect();
  const app: FastifyInstance = await buildServer({ db: pool, messageBus });

  const locationId = crypto.randomUUID();
  const ownerId = crypto.randomUUID();
  const orderId = crypto.randomUUID();
  let courierA = crypto.randomUUID();
  let courierB = crypto.randomUUID();
  let customerToken = '';
  let courierAToken = '';
  let ownerToken = '';
  let shiftAId = '';
  
  await t.test('setup db state', async () => {
    const client = await pool.connect();
    try {
      await client.query(`INSERT INTO locations (id, slug, name) VALUES ($1, $2, 'Dispatch Test')`, [locationId, `disp-${Date.now()}`]);
      await client.query(`INSERT INTO users (id, role, location_id, email, password_hash) VALUES ($1, 'owner', $2, 'owner@d.com', 'xyz')`, [ownerId, locationId]);
      
      const phoneEncA = encryptPII('+355691111111');
      const nameEncA = encryptPII('Alice Courier');
      await client.query(`INSERT INTO couriers (id, email_encrypted, phone_encrypted, full_name_encrypted, status) VALUES ($1, 'a', $2, $3, 'active')`, [courierA, phoneEncA, nameEncA]);
      await client.query(`INSERT INTO courier_locations (courier_id, location_id, role) VALUES ($1, $2, 'member')`, [courierA, locationId]);
      
      const phoneEncB = encryptPII('+355692222222');
      const nameEncB = encryptPII('Bob Courier');
      await client.query(`INSERT INTO couriers (id, email_encrypted, phone_encrypted, full_name_encrypted, status) VALUES ($1, 'b', $2, $3, 'active')`, [courierB, phoneEncB, nameEncB]);
      await client.query(`INSERT INTO courier_locations (courier_id, location_id, role) VALUES ($1, $2, 'member')`, [courierB, locationId]);

      await client.query(`INSERT INTO orders (id, location_id, total, status, delivery_pin_lat, delivery_pin_lng) VALUES ($1, $2, 1000, 'confirmed', 41.32, 19.82)`, [orderId, locationId]);
    } finally {
      client.release();
    }
  });

  await t.test('generate tokens', async () => {
    const { signAuthToken } = await import('@deliveryos/platform');
    customerToken = await signAuthToken({ sub: crypto.randomUUID(), role: 'customer', activeLocationId: locationId, orderId });
    courierAToken = await signAuthToken({ sub: courierA, role: 'courier', activeLocationId: locationId });
    ownerToken = await signAuthToken({ sub: ownerId, role: 'owner', activeLocationId: locationId });
  });

  await t.test('mock shift creation', async () => {
    // Manually create shift to simulate the endpoint, since we are only unit testing the workers here
    const client = await pool.connect();
    try {
      const shiftRes = await client.query(`
        INSERT INTO courier_shifts (courier_id, location_id, status, started_at, last_heartbeat_at)
        VALUES ($1, $2, 'available', now(), now())
        RETURNING id
      `, [courierA, locationId]);
      shiftAId = shiftRes.rows[0].id;
      
      await client.query(`
        INSERT INTO courier_positions (courier_id, location_id, shift_id, lat, lng, source)
        VALUES ($1, $2, $3, 41.32765, 19.81765, 'gps')
      `, [courierA, locationId, shiftAId]);
    } finally {
      client.release();
    }
  });

  await t.test('dispatch worker race condition assignment', async () => {
    // Only Courier A is available.
    // Trigger order.confirmed via worker
    const { CourierDispatchWorker } = await import('../src/workers/courier-dispatch.js');
    const mockBoss = { send: async () => {}, work: async () => {}, schedule: async () => {} } as any;
    const worker = new CourierDispatchWorker(pool, mockBoss, messageBus);
    
    // Put into queue manually
    await pool.query(`INSERT INTO courier_dispatch_queue (order_id, location_id) VALUES ($1, $2)`, [orderId, locationId]);

    // Fire 5 concurrent handleDispatch calls
    await Promise.all([
      worker.handleDispatch(orderId, locationId),
      worker.handleDispatch(orderId, locationId),
      worker.handleDispatch(orderId, locationId),
      worker.handleDispatch(orderId, locationId),
      worker.handleDispatch(orderId, locationId)
    ]);

    // Check assignments
    const asgnRes = await pool.query(`SELECT * FROM courier_assignments WHERE order_id = $1`, [orderId]);
    assert.strictEqual(asgnRes.rowCount, 1, 'Only 1 assignment should be created despite concurrent calls');
    assert.strictEqual(asgnRes.rows[0].courier_id, courierA);

    const queueRes = await pool.query(`SELECT * FROM courier_dispatch_queue WHERE order_id = $1`, [orderId]);
    assert.strictEqual(queueRes.rowCount, 0, 'Queue should be empty');
  });

  await t.test('customer ws payload privacy', async () => {
    const { CourierEventsWorker } = await import('../src/workers/courier-events.js');
    
    let publishedMsg: any;
    const mockBus = {
      publish: async (channel: string, msg: any) => {
        if (channel === `order:${orderId}`) {
          publishedMsg = msg;
        }
      },
      subscribe: async () => {}
    } as unknown as MessageBus;

    const evWorker = new CourierEventsWorker(pool, mockBus);
    await evWorker.handleAssignmentEvent({ orderId, locationId, courierId: courierA }, 'heading_to_pickup');

    assert.ok(publishedMsg, 'Should publish to order room');
    assert.strictEqual(publishedMsg.type, 'order.courier_updated');
    
    const p = publishedMsg.payload;
    assert.strictEqual(p.courierName, 'A***', 'Name must be masked');
    assert.strictEqual(p.phoneMasked, '+*** *** 1111', 'Phone must be masked');
    assert.strictEqual(p.status, 'heading_to_pickup');
  });

  await t.test('cleanup', async () => {
    await messageBus.close();
    const client = await pool.connect();
    try {
      await client.query(`DELETE FROM locations WHERE id = $1`, [locationId]); // cascade deletes order, courier_assignments, etc.
      await client.query(`DELETE FROM couriers WHERE id IN ($1, $2)`, [courierA, courierB]);
    } finally {
      client.release();
    }
    await pool.end();
  });
});
