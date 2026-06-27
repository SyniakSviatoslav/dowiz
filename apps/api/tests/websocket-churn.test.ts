import test from 'node:test';
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
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
  let otherOrderId: string;
  try {
    const ord = await pool.query(
      `SELECT id, location_id, customer_id FROM orders WHERE customer_id IS NOT NULL ORDER BY created_at DESC LIMIT 1`,
    );
    if (ord.rowCount === 0) {
      // Test Integrity #7: no silent return that greens a zero-assertion run.
      assert.fail('SETUP: no seeded order with a customer — seed a customer-scoped order before running this WS test');
    }
    orderId = ord.rows[0].id;
    locationId = ord.rows[0].location_id;
    customerId = ord.rows[0].customer_id;
    otherOrderId = randomUUID();
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

  // IDOR: the customer token is scoped to `order:${orderId}`. Subscribing to ANY
  // other order room with the same token must be rejected ('Forbidden room'), never
  // 'subscribed'. websocket.ts gates customer subscribes by exact room match, so a
  // different valid order id exercises the real authz decision (not a 404-by-absence).
  // TODO(staging): repeat with a REAL second tenant's order id for full cross-tenant
  // coverage against a live stack.
  {
    const idorWs = new WebSocket(url);
    await new Promise((res, rej) => { idorWs.on('open', res); idorWs.on('error', rej); });
    await once(idorWs, 'auth_success');
    idorWs.send(JSON.stringify({ type: 'subscribe', room: `order:${otherOrderId}` }));
    const reply = await new Promise<any>((res, rej) => {
      const to = setTimeout(() => rej(new Error('timeout waiting for IDOR reply')), 4000);
      idorWs.on('message', (d) => {
        const m = JSON.parse(d.toString());
        if (m.type === 'error' || m.type === 'subscribed') { clearTimeout(to); res(m); }
      });
    });
    idorWs.close();
    assert.equal(reply.type, 'error', `cross-order subscribe must be rejected, got ${reply.type}`);
    assert.equal(reply.error, 'Forbidden room');
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
  // Event-driven gate: wait until the delivery count has been stable (no new
  // in-flight messages) for >=200ms, capped at 3s — a leak keeps re-incrementing
  // and resets the stability window, so duplicates can't slip past a fixed sleep.
  let lastCount = -1;
  let stableSince = Date.now();
  const deadline = Date.now() + 3000;
  while (Date.now() < deadline) {
    if (deliveries === lastCount) {
      if (Date.now() - stableSince >= 200) break;
    } else {
      lastCount = deliveries;
      stableSince = Date.now();
    }
    await delay(50);
  }
  ws.close();

  assert.equal(
    deliveries, 1,
    `expected exactly 1 delivery for 1 published event after churn, got ${deliveries} (handler leak)`,
  );
});

// Negative-path auth: a tampered/invalid token must never authenticate. The server
// (websocket.ts) never emits auth_success and closes the socket with 1008 when an
// unauthenticated client sends a non-auth frame (or on the 5s auth timeout). This is
// the control proving the happy-path auth gate isn't silently accepting everyone.
test('WS auth: tampered token is rejected (no auth_success, closes 1008)', async () => {
  const badToken = 'eyJhbGciOiJub25lIn0.eyJzdWIiOiJhdHRhY2tlciJ9.tampered-signature';
  const ws = new WebSocket(`${URL_BASE}/?token=${badToken}`);
  await new Promise((res, rej) => { ws.on('open', res); ws.on('error', rej); });

  let gotAuthSuccess = false;
  ws.on('message', (d) => {
    const m = JSON.parse(d.toString());
    if (m.type === 'auth_success') gotAuthSuccess = true;
  });

  // Poke the server so the unauthenticated branch closes the socket promptly.
  ws.send(JSON.stringify({ type: 'subscribe', room: 'order:anything' }));

  const closeCode = await new Promise<number>((res, rej) => {
    const to = setTimeout(() => rej(new Error('timeout waiting for socket close')), 7000);
    ws.on('close', (code) => { clearTimeout(to); res(code); });
  });

  assert.equal(gotAuthSuccess, false, 'tampered token must NOT yield auth_success');
  assert.equal(closeCode, 1008, `expected 1008 policy-violation close, got ${closeCode}`);
});
