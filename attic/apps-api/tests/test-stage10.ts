import { test } from 'node:test';
import assert from 'node:assert';
import Fastify from 'fastify';
import { createSessionPool } from '@deliveryos/db';
import orderRoutes from '../src/routes/orders.js';
import { loadEnv } from '@deliveryos/config';
import crypto from 'crypto';
import jwt from 'jsonwebtoken';

const env = loadEnv();

test('Stage 10: Server Pricing Engine & Idempotency', async (t) => {
  const db = createSessionPool();
  
  // Clean up
  await db.query('DELETE FROM order_items');
  await db.query('DELETE FROM orders');
  await db.query('DELETE FROM customers');
  await db.query('DELETE FROM idempotency_keys');
  
  const fastify = Fastify();
  
  // Mock fastify.verifyAuth
  fastify.decorate('verifyAuth', async (req: any, reply: any) => {
    req.user = { role: 'customer', userId: 'cust-123' };
  });
  fastify.decorate('requireRole', (roles: string[]) => async () => {});
  
  const mockQueue = {
    enqueue: async () => 'job-123',
    work: async () => {},
    start: async () => {},
    stop: async () => {},
  };
  
  const mockMessageBus = {
    publish: async () => {},
    subscribe: async () => {},
    close: async () => {},
  };

  await fastify.register(orderRoutes, { db, messageBus: mockMessageBus, queue: mockQueue });

  let locationId = '';
  let productId = '';
  let mod1Id = '';
  let mod2Id = '';
  
  await t.test('setup seed data', async () => {
    const locRes = await db.query(`
      INSERT INTO locations (name, slug, email, confirm_timeout_min, busy_mode, currency_code, currency_minor_unit, min_order_value, free_delivery_threshold, delivery_fee_flat)
      VALUES ('Stage 10 Pizza', 'stage10', 's10@example.com', 10, false, 'ALL', 0, 1000, 5000, 200)
      RETURNING id
    `);
    locationId = locRes.rows[0].id;
    
    // Delivery tiers
    await db.query(`INSERT INTO delivery_tiers (location_id, max_distance_km, fee) VALUES ($1, 2.0, 150), ($1, 5.0, 300)`, [locationId]);

    const catRes = await db.query(`INSERT INTO categories (location_id, name) VALUES ($1, 'Pizzas') RETURNING id`, [locationId]);
    const catId = catRes.rows[0].id;
    
    const prodRes = await db.query(`INSERT INTO products (location_id, category_id, name, price, available) VALUES ($1, $2, 'Margherita', 800, true) RETURNING id`, [locationId, catId]);
    productId = prodRes.rows[0].id;

    // Modifiers
    const groupRes = await db.query(`INSERT INTO modifier_groups (location_id, name, min_select, max_select, required) VALUES ($1, 'Extras', 0, 2, false) RETURNING id`, [locationId]);
    const groupId = groupRes.rows[0].id;

    await db.query(`INSERT INTO product_modifier_groups (product_id, group_id) VALUES ($1, $2)`, [productId, groupId]);

    const m1 = await db.query(`INSERT INTO modifiers (location_id, group_id, name, price_delta) VALUES ($1, $2, 'Extra Cheese', 150) RETURNING id`, [locationId, groupId]);
    mod1Id = m1.rows[0].id;

    const m2 = await db.query(`INSERT INTO modifiers (location_id, group_id, name, price_delta) VALUES ($1, $2, 'Mushrooms', 100) RETURNING id`, [locationId, groupId]);
    mod2Id = m2.rows[0].id;
  });

  await t.test('creates order with server pricing correctly', async () => {
    const idempotency_key = crypto.randomUUID();
    const res = await fastify.inject({
      method: 'POST',
      url: '/orders',
      payload: {
        locationId,
        type: 'delivery',
        idempotency_key,
        customer: { phone: '123456789' },
        delivery: { pin: { lat: 41.3275, lng: 19.8187 } },
        payment: { method: 'cash' },
        items: [
          {
            product_id: productId,
            quantity: 2, // 2 pizzas
            modifier_ids: [mod1Id, mod2Id] // +150, +100 = 250 per pizza
          }
        ]
      }
    });

    assert.strictEqual(res.statusCode, 201, res.payload);
    const body = JSON.parse(res.payload);
    // Pizza 800 + mod1 150 + mod2 100 = 1050 * 2 = 2100 subtotal
    // Delivery fee fallback flat = 200 (since location lat/lng is null in seed)
    assert.strictEqual(body.subtotal, 2100);
    assert.strictEqual(body.total, 2300);
  });
  
  await t.test('idempotency exact match returns 200', async () => {
    const idempotency_key = crypto.randomUUID();
    const payload = {
      locationId,
      type: 'delivery',
      idempotency_key,
      customer: { phone: '123456789' },
      delivery: { pin: { lat: 41.3275, lng: 19.8187 } },
      payment: { method: 'cash' },
      items: [
        {
          product_id: productId,
          quantity: 1,
          modifier_ids: [mod2Id, mod1Id] // Unordered
        }
      ]
    };
    
    const res1 = await fastify.inject({ method: 'POST', url: '/orders', payload });
    assert.strictEqual(res1.statusCode, 201);
    
    // Subtotal: 800 + 250 = 1050. Flat delivery 200 => Total 1250.
    
    // Exact same payload (different modifier array order should map to same hash)
    const res2 = await fastify.inject({ method: 'POST', url: '/orders', payload });
    assert.strictEqual(res2.statusCode, 200, res2.payload);
    
    const b1 = JSON.parse(res1.payload);
    const b2 = JSON.parse(res2.payload);
    assert.strictEqual(b1.id, b2.id);
  });
  
  await t.test('idempotency key reused with different body returns 422', async () => {
    const idempotency_key = crypto.randomUUID();
    const payload1 = {
      locationId, type: 'delivery', idempotency_key,
      customer: { phone: '123456789' }, delivery: { pin: { lat: 41.3, lng: 19.8 } },
      payment: { method: 'cash' },
      items: [{ product_id: productId, quantity: 1, modifier_ids: [] }]
    };
    
    await fastify.inject({ method: 'POST', url: '/orders', payload: payload1 });
    
    const payload2 = {
      ...payload1,
      items: [{ product_id: productId, quantity: 2, modifier_ids: [] }]
    };
    
    const res = await fastify.inject({ method: 'POST', url: '/orders', payload: payload2 });
    assert.strictEqual(res.statusCode, 422);
    assert.strictEqual(JSON.parse(res.payload).code, 'IDEMPOTENCY_KEY_REUSED');
  });
  
  await db.end();
});
