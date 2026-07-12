import test from 'node:test';
import assert from 'node:assert/strict';
import WebSocket from 'ws';
import { RedisMessageBus, issueCustomerToken } from '@deliveryos/platform';
import { createSessionPool } from '@deliveryos/db';

// P1-WSDUP regression: websocket.ts subscribeToRoom registers exactly one
// messageBus handler per room. Before the fix, re-creating a room (a member
// rejoins after the previous room was torn down) stacked an additional
// messageBus.subscribe handler that was never removed, so a single published
// event was delivered once *per leaked handler*. We assert that after many
// reconnect churns, a single published event yields exactly one client delivery.
//
// Requires the local stack on :3000 (same as the other live tests) and a
// published location with at least one orderable product. We mint a customer
// token directly (no HTTP order needed) so the test is self-contained.

const URL_BASE = process.env.WS_BASE || 'ws://localhost:3000';

function once(ws: WebSocket, type: string, timeoutMs = 4000): Promise<any> {
  return new Promise((res, rej) => {
    const to = setTimeout(() => rej(new Error('timeout waiting for ' + type)), timeoutMs);
    ws.on('message', (d) => {
      const m = JSON.parse(d.toString());
      if (m.type === type) { clearTimeout(to); res(m); }
    });
  });
}

const delay = (ms: number) => new Promise((r) => setTimeout(r, ms));

test('P1-WSDUP: one delivery per event after reconnect churn', async () => {
  const pool = createSessionPool();
  let orderId: string;
  let locationId: string;
  let customerId: string;
  try {
    const ord = await pool.query(
      `SELECT id, location_id, customer_id FROM orders WHERE customer_id IS NOT NULL ORDER BY created_at DESC LIMIT 1`,
    );
    if (ord.rowCount === 0) {
      console.log('SKIP: no seeded order with a customer to scope a token');
      return;
    }
    orderId = ord.rows[0].id;
    locationId = ord.rows[0].location_id;
    customerId = ord.rows[0].customer_id;
  } finally {
    await pool.end();
  }

  const token = await issueCustomerToken({ orderId, locationId, customerId });
  const room = `order:${orderId}`;
  const url = `${URL_BASE}/?token=${token}`;

  async function connectAndSubscribe(): Promise<WebSocket> {
    const ws = new WebSocket(url);
    await new Promise((res, rej) => { ws.on('open', res); ws.on('error', rej); });
    await once(ws, 'auth_success');
    ws.send(JSON.stringify({ type: 'subscribe', room }));
    await once(ws, 'subscribed');
    return ws;
  }

  // Churn the room: each iteration creates the room (1 bus.subscribe) and then
  // empties it on close (deleteRoom -> bus.unsubscribe). A leak would accumulate
  // bus handlers across iterations.
  for (let i = 0; i < 10; i++) {
    const ws = await connectAndSubscribe();
    await delay(80);
    ws.close();
    await delay(220);
  }

  const ws = await connectAndSubscribe();
  let deliveries = 0;
  ws.on('message', (d) => {
    const m = JSON.parse(d.toString());
    if (m.room === room) deliveries++;
  });
  await delay(200);

  const bus = new RedisMessageBus(createSessionPool());
  await bus.connect();
  await delay(300);
  await bus.publish(room, { type: 'evt', seq: 1 });
  await delay(900);
  ws.close();

  assert.equal(
    deliveries, 1,
    `expected exactly 1 delivery for 1 published event after churn, got ${deliveries} (handler leak)`,
  );
});
