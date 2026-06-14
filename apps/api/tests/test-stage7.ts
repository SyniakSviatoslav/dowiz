import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';
import crypto from 'crypto';
import { signAuthToken } from '@deliveryos/platform';
import WebSocket from 'ws';

const env = loadEnv();

async function delay(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function runTests() {
  const pool = createSessionPool();
  try {
    console.log('--- Stage 7: E2E Tests ---');

    // Ensure tables exist and delete prior data (except seed data)
    await pool.query(`
      DELETE FROM idempotency_keys;
    `);

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
    const prodInsert = await pool.query(
      `INSERT INTO products (id, location_id, name, price, is_available) VALUES ($1, $2, 'Burger', 1000, true) RETURNING id`,
      [prodId, locId]
    );
    console.log('Inserted product:', prodInsert.rows[0]);

    const ownerToken = await signAuthToken({ role: 'owner', userId }, '15m');

    console.log('Testing Idempotent Order Creation...');
    const idempotencyKey = crypto.randomUUID();
    const orderPayload = {
      locationId: locId,
      type: 'delivery',
      items: [{ productId: prodId, quantity: 2 }],
      customer: { phone: '+380991234567', name: 'John Doe' },
      delivery: { address: '123 Test St', lat: 10, lng: 10 },
      payment: { method: 'cash' },
      idempotencyKey
    };

    const res1 = await fetch(`http://127.0.0.1:3003/orders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(orderPayload)
    });
    const data1 = await res1.json();
    if (res1.status !== 201) throw new Error(`Failed to create order: ${JSON.stringify(data1)}`);
    console.log('✅ Order created:', data1.id, 'Total:', data1.total);

    const res2 = await fetch(`http://127.0.0.1:3003/orders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(orderPayload)
    });
    const data2 = await res2.json() as any;
    if (data2.id !== data1.id) throw new Error('Idempotency failed');
    console.log('✅ Idempotency: same request returned existing order');

    const res3 = await fetch(`http://127.0.0.1:3003/orders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ...orderPayload, customer: { phone: '+380990000000', name: 'John' } })
    });
    if (res3.status !== 422) throw new Error('Idempotency hash check failed');
    console.log('✅ Idempotency: altered request rejected with 422');

    console.log('Testing WS Reconcile...');
    const wsClient = new WebSocket(`ws://127.0.0.1:3004`);
    
    // Buffer for messages received
    const wsMessages: any[] = [];
    wsClient.on('message', (data) => {
      wsMessages.push(JSON.parse(data.toString()));
    });

    await new Promise<void>((resolve, reject) => {
      wsClient.on('open', () => wsClient.send(JSON.stringify({ type: 'auth', token: data1.customerToken })));
      const checkAuth = setInterval(() => {
        if (wsMessages.some(m => m.type === 'auth_success')) {
          clearInterval(checkAuth);
          console.log('✅ Customer WS authenticated on :3004');
          wsClient.send(JSON.stringify({ type: 'subscribe', room: `order:${data1.id}` }));
          
          // Wait for subscribed confirmation before resolving
          const checkSub = setInterval(() => {
            if (wsMessages.some(m => m.type === 'subscribed' && m.room === `order:${data1.id}`)) {
              clearInterval(checkSub);
              resolve();
            }
          }, 50);
        }
      }, 100);
      setTimeout(() => { clearInterval(checkAuth); reject(new Error('Auth timeout')); }, 3000);
    });

    console.log('Testing Anti-Race Condition...');
    const [confirmRes1, confirmRes2] = await Promise.all([
      fetch(`http://127.0.0.1:3003/orders/${data1.id}/status`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${ownerToken}` },
        body: JSON.stringify({ status: 'CONFIRMED' })
      }),
      fetch(`http://127.0.0.1:3004/orders/${data1.id}/status`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${ownerToken}` },
        body: JSON.stringify({ status: 'CONFIRMED' })
      })
    ]);
    
    const cCodes = [confirmRes1.status, confirmRes2.status].sort();
    if (cCodes[0] !== 200 || (cCodes[1] !== 409 && cCodes[1] !== 400)) {
      throw new Error(`Anti-race failed, expected [200, 409] or [200, 400], got [${confirmRes1.status},${confirmRes2.status}]`);
    }
    console.log(`✅ Anti-race: simultaneous updates safely resolved with [${confirmRes1.status}, ${confirmRes2.status}]`);

    await new Promise<void>((resolve, reject) => {
      const checkMsg = setInterval(() => {
        if (wsMessages.some(m => m.room === `order:${data1.id}` && m.data?.type === 'order.status' && m.data?.status === 'CONFIRMED')) {
          clearInterval(checkMsg);
          console.log('✅ WS Reconcile: customer received CONFIRMED broadcast');
          resolve();
        }
      }, 100);
      setTimeout(() => { 
        clearInterval(checkMsg); 
        reject(new Error(`WS timeout waiting for broadcast. Received messages: ${JSON.stringify(wsMessages)}`)); 
      }, 3000);
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
