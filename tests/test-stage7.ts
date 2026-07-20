import { loadEnv } from '@deliveryos/config';
import { createOperationalPool } from '@deliveryos/db';
import crypto from 'crypto';
import { signAuthToken } from '@deliveryos/platform';
import WebSocket from 'ws';

const env = loadEnv();

async function delay(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function runTests() {
  const pool = createOperationalPool();
  try {
    console.log('--- Stage 7: E2E Tests ---');

    const locId = crypto.randomUUID();
    const orgId = crypto.randomUUID();
    const userId = crypto.randomUUID();
    const prodId = crypto.randomUUID();

    await pool.query(
      `INSERT INTO users (id, email) VALUES ($1, $2) ON CONFLICT DO NOTHING`,
      [userId, `test-owner-${Date.now()}@test.com`]
    );
    await pool.query(
      `INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'Test Org', $2) ON CONFLICT DO NOTHING`,
      [orgId, userId]
    );
    await pool.query(
      `INSERT INTO locations (id, org_id, slug, name, phone, status, confirm_timeout_min) 
       VALUES ($1, $2, $3, 'Test Loc', '123', 'open', 1) ON CONFLICT DO NOTHING`,
      [locId, orgId, `test-loc-${Date.now()}`]
    );
    await pool.query(
      `INSERT INTO memberships (user_id, location_id, role, status) VALUES ($1, $2, 'owner', 'active') ON CONFLICT DO NOTHING`,
      [userId, locId]
    );
    await pool.query(
      `INSERT INTO products (id, location_id, name, price, is_available) VALUES ($1, $2, 'Burger', 1000, true) ON CONFLICT DO NOTHING`,
      [prodId, locId]
    );

    const ownerToken = await signAuthToken({ role: 'owner', userId }, '15m');

    console.log('Testing Idempotent Order Creation...');
    const idempotencyKey = crypto.randomUUID();
    const orderPayload = {
      type: 'delivery',
      items: [{ productId: prodId, quantity: 2 }],
      customer: { phone: '+380991234567', name: 'John Doe' },
      delivery: { address: '123 Test St', lat: 10, lng: 10 },
      payment: { method: 'cash' },
      idempotencyKey
    };

    const res1 = await fetch(`http://127.0.0.1:3000/orders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(orderPayload)
    });
    const data1 = await res1.json();
    if (res1.status !== 201) throw new Error(`Failed to create order: ${JSON.stringify(data1)}`);
    console.log('✅ Order created:', data1.orderId, 'Total:', data1.total);

    const res2 = await fetch(`http://127.0.0.1:3000/orders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(orderPayload)
    });
    const data2 = await res2.json();
    if (data2.orderId !== data1.orderId) throw new Error('Idempotency failed');
    console.log('✅ Idempotency: same request returned existing order');

    const res3 = await fetch(`http://127.0.0.1:3000/orders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ...orderPayload, customer: { phone: '+380990000000', name: 'John' } })
    });
    if (res3.status !== 422) throw new Error('Idempotency hash check failed');
    console.log('✅ Idempotency: altered request rejected with 422');

    console.log('Testing WS Reconcile...');
    const wsClient = new WebSocket(`ws://127.0.0.1:3001`);
    await new Promise<void>((resolve, reject) => {
      wsClient.on('open', () => wsClient.send(JSON.stringify({ type: 'auth', token: data1.token })));
      wsClient.on('message', (data) => {
        const msg = JSON.parse(data.toString());
        if (msg.type === 'auth_success') {
          console.log('✅ Customer WS authenticated on :3001');
          resolve();
        }
      });
      wsClient.on('error', reject);
    });

    console.log('Testing Anti-Race Condition...');
    const [confirmRes1, confirmRes2] = await Promise.all([
      fetch(`http://127.0.0.1:3000/orders/${data1.orderId}/status`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${ownerToken}` },
        body: JSON.stringify({ status: 'CONFIRMED' })
      }),
      fetch(`http://127.0.0.1:3001/orders/${data1.orderId}/status`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${ownerToken}` },
        body: JSON.stringify({ status: 'CONFIRMED' })
      })
    ]);
    
    const statusCodes = [confirmRes1.status, confirmRes2.status];
    if (statusCodes.includes(200) && statusCodes.includes(409)) {
      console.log('✅ Anti-race condition verified (one 200, one 409)');
    } else {
      throw new Error(`Anti-race failed, expected [200, 409], got [${statusCodes}]`);
    }

    await new Promise<void>((resolve, reject) => {
      wsClient.on('message', (data) => {
        const msg = JSON.parse(data.toString());
        if (msg.type === 'order_updated' && msg.status === 'CONFIRMED') {
          console.log('✅ WS received cross-instance broadcast (CONFIRMED)');
          resolve();
        }
      });
      setTimeout(() => reject(new Error('WS timeout waiting for broadcast')), 2000);
    });

    wsClient.close();
    console.log('🎉 All Stage 7 tests passed!');
  } finally {
    await pool.end();
  }
}

runTests().catch(err => {
  console.error('Test failed:', err);
  process.exit(1);
});
