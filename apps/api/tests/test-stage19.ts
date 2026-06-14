import { test } from 'node:test';
import assert from 'node:assert';
import { createSessionPool } from '@deliveryos/db';
import { encryptPII } from '../src/lib/pii-cipher.js';

test('Stage 19: Courier Cash Cycle', async (t) => {
  const pool = createSessionPool();
  
  const locationId = crypto.randomUUID();
  const ownerId = crypto.randomUUID();
  const customerId = crypto.randomUUID();
  const orderId = crypto.randomUUID();
  const courierA = crypto.randomUUID();
  const assignmentId = crypto.randomUUID();

  await t.test('setup db state', async () => {
    const client = await pool.connect();
    try {
      const orgId = crypto.randomUUID();
      const email = `owner-${Date.now()}@c.com`;
      await client.query(`INSERT INTO users (id, email) VALUES ($1, $2)`, [ownerId, email]);
      await client.query(`INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'Cash Org', $2)`, [orgId, ownerId]);
      await client.query(`INSERT INTO locations (id, org_id, slug, name, phone, currency_code) VALUES ($1, $2, $3, 'Cash Loc', '+355691111111', 'ALL')`, [locationId, orgId, `cash-${Date.now()}`]);
      await client.query(`INSERT INTO memberships (user_id, location_id, role, status) VALUES ($1, $2, 'owner', 'active')`, [ownerId, locationId]);
      await client.query(`INSERT INTO customers (id, location_id, phone, name) VALUES ($1, $2, '+355691111111', 'Cust')`, [customerId, locationId]);
      
      const phoneEncA = encryptPII('+355691111111');
      const nameEncA = encryptPII('Alice Cash Courier');
      const eh = `eh-${crypto.randomUUID()}`;
      const ph = `ph-${crypto.randomUUID()}`;
      await client.query(`INSERT INTO couriers (id, email_encrypted, email_hash, phone_encrypted, phone_hash, full_name_encrypted, password_hash, status) VALUES ($1, 'cash_c', $2, $3, $4, $5, 'pw', 'active')`, [courierA, eh, phoneEncA, ph, nameEncA]);
      await client.query(`INSERT INTO courier_locations (courier_id, location_id, role) VALUES ($1, $2, 'courier')`, [courierA, locationId]);
      
      // Insert order IN_DELIVERY
      await client.query(`
        INSERT INTO orders (id, location_id, customer_id, subtotal, total, status, request_hash) 
        VALUES ($1, $2, $3, 1000, 1000, 'IN_DELIVERY', 'fakehash')
      `, [orderId, locationId, customerId]);

      // Insert assignment
      await client.query(`
        INSERT INTO courier_assignments (id, order_id, courier_id, location_id, status)
        VALUES ($1, $2, $3, $4, 'picked_up')
      `, [assignmentId, orderId, courierA, locationId]);

    } finally {
      client.release();
    }
  });

  await t.test('cash immutability check', async () => {
    const client = await pool.connect();
    try {
      // First, artificially deliver it to get cash collected
      await client.query(`
        UPDATE courier_assignments 
        SET status = 'delivered', cash_collected = true, cash_amount = 1000, delivered_at = now()
        WHERE id = $1
      `, [assignmentId]);

      // Now, try to mutate cash_collected without reversal flag
      await assert.rejects(async () => {
        await client.query(`UPDATE courier_assignments SET cash_collected = false WHERE id = $1`, [assignmentId]);
      }, /cash_collected\/cash_amount immutable except via settlement reversal/);

      // Now, try with the reversal flag
      await client.query('BEGIN');
      await client.query(`SET LOCAL app.settlement_reversal = 'true'`);
      await client.query(`UPDATE courier_assignments SET cash_collected = false WHERE id = $1`, [assignmentId]);
      
      // Set it back for the next tests
      await client.query(`UPDATE courier_assignments SET cash_collected = true, cash_amount = 1000 WHERE id = $1`, [assignmentId]);
      await client.query('COMMIT');
    } finally {
      client.release();
    }
  });

  await t.test('settlement cron idempotency', async () => {
    const { SettlementCronWorker } = await import('../src/workers/settlement-cron.js');
    const mockBoss = { work: async () => {}, schedule: async () => {} } as any;
    const worker = new SettlementCronWorker(pool, mockBoss);

    // Generate settlements for TODAY's deliveries by passing TOMORROW as the reference date
    const tomorrow = new Date(Date.now() + 86400000);
    // Call it twice concurrently
    await Promise.all([
      worker.handleGenerate(tomorrow),
      worker.handleGenerate(tomorrow)
    ]);

    const res = await pool.query(`SELECT * FROM settlement_items WHERE assignment_id = $1`, [assignmentId]);
    assert.strictEqual(res.rowCount, 1, 'Should only create exactly one settlement item');
    assert.strictEqual(res.rows[0].amount, 1000);
  });

  await t.test('cleanup', async () => {
    const client = await pool.connect();
    try {
      // Delete in correct dependency order
      await client.query(`DELETE FROM settlement_audit_log WHERE location_id = $1`, [locationId]);
      await client.query(`UPDATE courier_assignments SET settlement_item_id = NULL WHERE location_id = $1`, [locationId]);
      await client.query(`DELETE FROM settlement_items WHERE location_id = $1`, [locationId]);
      await client.query(`DELETE FROM courier_payouts WHERE location_id = $1`, [locationId]);
      await client.query(`DELETE FROM courier_assignments WHERE id = $1`, [assignmentId]);
      await client.query(`DELETE FROM orders WHERE id = $1`, [orderId]);
      await client.query(`DELETE FROM customers WHERE id = $1`, [customerId]);
      await client.query(`DELETE FROM courier_locations WHERE courier_id = $1`, [courierA]);
      await client.query(`DELETE FROM couriers WHERE id = $1`, [courierA]);
      await client.query(`DELETE FROM memberships WHERE user_id = $1`, [ownerId]);
      await client.query(`DELETE FROM locations WHERE id = $1`, [locationId]);
      await client.query(`DELETE FROM organizations WHERE owner_id = $1`, [ownerId]);
      await client.query(`DELETE FROM users WHERE id = $1`, [ownerId]);
    } finally {
      client.release();
    }
    await pool.end();
  });
});
