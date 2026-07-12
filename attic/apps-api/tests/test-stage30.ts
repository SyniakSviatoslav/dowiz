import { test } from 'node:test';
import assert from 'node:assert';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';
import { signAuthToken } from '@deliveryos/platform';
import { AnonymizerService } from '../src/lib/anonymizer/index.js';

const env = loadEnv();
const BASE = `http://127.0.0.1:${env.PORT || 3003}`;

async function serverAvailable(): Promise<boolean> {
  try {
    const res = await fetch(`${BASE}/health`, { signal: AbortSignal.timeout(2000) });
    return res.ok;
  } catch {
    return false;
  }
}

test('Stage 30: Anonymizer (P5-0)', async (t) => {
  const pool = createSessionPool();
  const orgId = crypto.randomUUID();
  const orgId2 = crypto.randomUUID();
  const locId = crypto.randomUUID();
  const locIdB = crypto.randomUUID();
  const locId2 = crypto.randomUUID();
  const userId = crypto.randomUUID();
  const userId2 = crypto.randomUUID();
  const custId = crypto.randomUUID();
  const prodId = crypto.randomUUID();
  const orderId = crypto.randomUUID();
  const custIdR4 = crypto.randomUUID();
  const custIdOld = crypto.randomUUID();
  const custIdYoung = crypto.randomUUID();
  const custIdGdpr = crypto.randomUUID();
  let custIdAvatar: string | undefined;

  let ownerToken: string;
  let ownerToken2: string;
  let customerToken: string;

  const events: Array<{ event: string; payload: any }> = [];
  const messageBus = {
    publish: async (event: string, payload: any) => { events.push({ event, payload }); },
  };
  const storageDeleteCalls: string[] = [];
  const storageProvider = {
    put: async () => {},
    get: async () => null,
    delete: async (key: string) => { storageDeleteCalls.push(key); },
  };
  let anonymizer: AnonymizerService;

  // ─── Setup ────────────────────────────────────────────────────────
  await t.test('setup test data', async () => {
    await pool.query(`INSERT INTO users (id, email) VALUES ($1, $2) ON CONFLICT DO NOTHING`,
      [userId, `owner-p30-${Date.now()}@test.com`]);
    await pool.query(`INSERT INTO users (id, email) VALUES ($1, $2) ON CONFLICT DO NOTHING`,
      [userId2, `owner2-p30-${Date.now()}@test.com`]);

    await pool.query(`INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'P30 Org', $2) ON CONFLICT DO NOTHING`,
      [orgId, userId]);
    await pool.query(`INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'P30 Org 2', $2) ON CONFLICT DO NOTHING`,
      [orgId2, userId2]);

    await pool.query(
      `INSERT INTO locations (id, org_id, slug, name, phone, status) VALUES ($1, $2, $3, 'P30 Loc', '123', 'open') ON CONFLICT DO NOTHING`,
      [locId, orgId, `p30-loc-${Date.now()}`]);
    await pool.query(
      `INSERT INTO locations (id, org_id, slug, name, phone, status) VALUES ($1, $2, $3, 'P30 Loc B', '456', 'open') ON CONFLICT DO NOTHING`,
      [locIdB, orgId, `p30-loc-b-${Date.now()}`]);
    await pool.query(
      `INSERT INTO locations (id, org_id, slug, name, phone, status) VALUES ($1, $2, $3, 'P30 Loc Cross', '789', 'open') ON CONFLICT DO NOTHING`,
      [locId2, orgId2, `p30-loc-cross-${Date.now()}`]);

    await pool.query(
      `INSERT INTO customers (id, location_id, phone, name, no_show_count, completed_count) VALUES ($1, $2, '+355691234567', 'Test Customer', 3, 5) ON CONFLICT DO NOTHING`,
      [custId, locId]);

    await pool.query(
      `INSERT INTO products (id, location_id, name, price, is_available) VALUES ($1, $2, 'Test Product', 500, true) ON CONFLICT DO NOTHING`,
      [prodId, locId]);

    const reqHash = crypto.createHash('sha256').update(crypto.randomUUID()).digest('hex');
    await pool.query(
      `INSERT INTO orders (id, location_id, customer_id, request_hash, subtotal, total, delivery_fee, discount_total, tax_total, client_ip_hash, delivery_address, type, status, payment_method, payment_outcome, created_at)
       VALUES ($1, $2, $3, $4, 1000, 1200, 100, 50, 150, $5, '123 Main St', 'delivery', 'PENDING', 'cash', 'pending', now())
       ON CONFLICT DO NOTHING`,
      [orderId, locId, custId, reqHash, crypto.createHash('sha256').update('192.168.1.1').digest('hex')]);

    await pool.query(
      `INSERT INTO order_items (order_id, product_id, name_snapshot, price_snapshot, quantity) VALUES ($1, $2, 'Test Product', 500, 2) ON CONFLICT DO NOTHING`,
      [orderId, prodId]);

    ownerToken = await signAuthToken({ role: 'owner', userId, activeLocationId: locId }, '15m');
    ownerToken2 = await signAuthToken({ role: 'owner', userId: userId2, activeLocationId: locId2 }, '15m');
    customerToken = await signAuthToken({ role: 'customer', userId: custId, activeLocationId: locId }, '15m');

    events.length = 0;
    anonymizer = new AnonymizerService(pool, messageBus);
  });

  // ═══════════════════════════════════════════════════════════════
  // R1: Single mechanism — two triggers
  // ═══════════════════════════════════════════════════════════════
  await t.test('R1.1: AnonymizerService has exactly 1 anonymize method', () => {
    const proto = Object.getOwnPropertyNames(AnonymizerService.prototype);
    const anonMethods = proto.filter(m => m === 'anonymize');
    assert.strictEqual(anonMethods.length, 1);
  });

  await t.test('R1.2: both workers import and call anonymizerService.anonymize', () => {
    const retentionPath = path.resolve('apps/api/src/workers/anonymizer-retention.ts');
    const gdprPath = path.resolve('apps/api/src/workers/anonymizer-gdpr.ts');

    const retentionContent = fs.readFileSync(retentionPath, 'utf8');
    const gdprContent = fs.readFileSync(gdprPath, 'utf8');

    assert.ok(retentionContent.includes("import { AnonymizerService }"), 'retention worker imports AnonymizerService');
    assert.ok(gdprContent.includes("import { AnonymizerService }"), 'GDPR worker imports AnonymizerService');

    assert.ok(retentionContent.includes("anonymizerService.anonymize({"), 'retention worker calls .anonymize()');
    assert.ok(gdprContent.includes("anonymizerService.anonymize({"), 'GDPR worker calls .anonymize()');
  });

  // ═══════════════════════════════════════════════════════════════
  // R2: Anonymize, NOT delete
  // ═══════════════════════════════════════════════════════════════
  await t.test('R2: customer anonymized, NOT deleted', async () => {
    events.length = 0;

    const result = await anonymizer.anonymize({
      scope: 'retention',
      subject: { customerId: custId, locationId: locId },
    });

    assert.strictEqual(result.customersAnonymized, 1);
    assert.strictEqual(result.ordersAnonymized, 0);
    // Customer-scope anonymization does NOT cascade to orders
    // Order PII is only cleared via order-scope or retention sweep
    assert.strictEqual(result.skipped, 0);

    // Row still exists (NOT deleted)
    const countRes = await pool.query(`SELECT count(*)::int AS cnt FROM customers WHERE id = $1`, [custId]);
    assert.strictEqual(countRes.rows[0].cnt, 1, 'customer row must still exist, NOT deleted');

    // Phone → anon_ prefix
    const phoneRes = await pool.query(`SELECT phone FROM customers WHERE id = $1`, [custId]);
    assert.ok(phoneRes.rows[0].phone.startsWith('anon_'), `phone must start with anon_, got ${phoneRes.rows[0].phone}`);

    // Name → NULL
    const nameRes = await pool.query(`SELECT name FROM customers WHERE id = $1`, [custId]);
    assert.strictEqual(nameRes.rows[0].name, null);

    // marketing_opt_in → false
    const mktRes = await pool.query(`SELECT marketing_opt_in FROM customers WHERE id = $1`, [custId]);
    assert.strictEqual(mktRes.rows[0].marketing_opt_in, false);

    // Business fields unchanged
    const bizRes = await pool.query(`SELECT no_show_count, completed_count FROM customers WHERE id = $1`, [custId]);
    assert.strictEqual(bizRes.rows[0].no_show_count, 3);
    assert.strictEqual(bizRes.rows[0].completed_count, 5);

    // anonymized_at → IS NOT NULL
    const anonAtRes = await pool.query(`SELECT anonymized_at FROM customers WHERE id = $1`, [custId]);
    assert.ok(anonAtRes.rows[0].anonymized_at !== null);

    // Order client_ip_hash → unchanged (customer-scope does not cascade to orders)
    const ipRes = await pool.query(`SELECT client_ip_hash FROM orders WHERE id = $1`, [orderId]);
    assert.ok(ipRes.rows[0].client_ip_hash !== null, 'client_ip_hash must remain unchanged in customer-scope anonymization');

    // Order business fields unchanged
    const obRes = await pool.query(
      `SELECT total, subtotal, tax_total, delivery_fee, discount_total FROM orders WHERE id = $1`,
      [orderId],
    );
    assert.strictEqual(obRes.rows[0].total, 1200);
    assert.strictEqual(obRes.rows[0].subtotal, 1000);
    assert.strictEqual(obRes.rows[0].tax_total, 150);
    assert.strictEqual(obRes.rows[0].delivery_fee, 100);
    assert.strictEqual(obRes.rows[0].discount_total, 50);

    // Order items count unchanged
    const oiRes = await pool.query(`SELECT count(*)::int AS cnt FROM order_items WHERE order_id = $1`, [orderId]);
    assert.strictEqual(oiRes.rows[0].cnt, 1);
  });

  // ═══════════════════════════════════════════════════════════════
  // R3: Storage + R2
  // ═══════════════════════════════════════════════════════════════
  await t.test('R3: avatar_key cleanup on anonymization', async () => {
    // Skip if avatar_key column does not exist in test schema
    const colRes = await pool.query(
      `SELECT TRUE FROM pg_attribute WHERE attrelid = 'customers'::regclass AND attname = 'avatar_key' AND NOT attisdropped`,
    );
    if (colRes.rowCount === 0) {
      console.log('Skipping R3 — avatar_key column does not exist in test schema');
      return;
    }

    custIdAvatar = crypto.randomUUID();
    await pool.query(
      `INSERT INTO customers (id, location_id, phone, name, avatar_key) VALUES ($1, $2, '+355699990009', 'Avatar Test', 'avatars/test.png') ON CONFLICT DO NOTHING`,
      [custIdAvatar, locId],
    );

    storageDeleteCalls.length = 0;
    const anonymizerWithStorage = new AnonymizerService(pool, messageBus, storageProvider);

    const result = await anonymizerWithStorage.anonymize({
      scope: 'gdpr',
      subject: { customerId: custIdAvatar, locationId: locId },
    });

    assert.strictEqual(result.customersAnonymized, 1);
    assert.strictEqual(result.storagePurged, 1);
    assert.strictEqual(storageDeleteCalls.length, 1);
    assert.strictEqual(storageDeleteCalls[0], 'avatars/test.png');
  });

  // ═══════════════════════════════════════════════════════════════
  // R4: Idempotency
  // ═══════════════════════════════════════════════════════════════
  await t.test('R4: anonymize is idempotent', async () => {
    await pool.query(
      `INSERT INTO customers (id, location_id, phone, name, no_show_count, completed_count) VALUES ($1, $2, '+355699990002', 'Idempotent Test', 0, 0) ON CONFLICT DO NOTHING`,
      [custIdR4, locId],
    );

    // First call — should anonymize
    const first = await anonymizer.anonymize({
      scope: 'retention',
      subject: { customerId: custIdR4, locationId: locId },
    });
    assert.strictEqual(first.customersAnonymized, 1);
    assert.strictEqual(first.skipped, 0);

    const phone1 = await pool.query(`SELECT phone FROM customers WHERE id = $1`, [custIdR4]);
    assert.ok(phone1.rows[0].phone.startsWith('anon_'));

    const anonAt1 = await pool.query(`SELECT anonymized_at FROM customers WHERE id = $1`, [custIdR4]);
    const anonAtVal = anonAt1.rows[0].anonymized_at;

    // Second call — should skip (already anonymized)
    const second = await anonymizer.anonymize({
      scope: 'retention',
      subject: { customerId: custIdR4, locationId: locId },
    });
    assert.strictEqual(second.customersAnonymized, 0);
    assert.strictEqual(second.skipped, 1);

    // Verify no changes
    const phone2 = await pool.query(`SELECT phone FROM customers WHERE id = $1`, [custIdR4]);
    assert.strictEqual(phone2.rows[0].phone, phone1.rows[0].phone, 'phone must not change on second call');

    const anonAt2 = await pool.query(`SELECT anonymized_at FROM customers WHERE id = $1`, [custIdR4]);
    assert.strictEqual(anonAt2.rows[0].anonymized_at.toISOString(), anonAtVal.toISOString(), 'anonymized_at must not change');
  });

  // ═══════════════════════════════════════════════════════════════
  // R5: Security
  // ═══════════════════════════════════════════════════════════════
  const serverAvail = await serverAvailable();
  await t.test('R5.1: cross-tenant GDPR request returns 404', { skip: !serverAvail }, async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId2}/gdpr-requests`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ customerId: custId }),
    });
    assert.strictEqual(res.status, 404);
  });

  await t.test('R5.2: non-owner cannot create GDPR request', { skip: !serverAvail }, async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/gdpr-requests`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${customerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ customerId: custId }),
    });
    assert.strictEqual(res.status, 403);
  });

  await t.test('R5.3: Zod strict on GDPR endpoint returns 400', { skip: !serverAvail }, async () => {
    const res = await fetch(`${BASE}/api/owner/locations/${locId}/gdpr-requests`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ customerId: custId, extraField: 'must-not-pass' }),
    });
    assert.strictEqual(res.status, 400);
  });

  // ═══════════════════════════════════════════════════════════════
  // R6: Schema + RLS
  // ═══════════════════════════════════════════════════════════════
  await t.test('R6.1: retention_days CHECK rejects 29', async () => {
    try {
      await pool.query(
        `INSERT INTO locations (id, org_id, slug, name, phone, status, retention_days)
         VALUES ($1, $2, $3, 'R6 Check 29', '000', 'open', 29)`,
        [crypto.randomUUID(), orgId, `r6-29-${Date.now()}`],
      );
      assert.fail('Should have thrown CHECK constraint violation for retention_days = 29');
    } catch (err: any) {
      assert.ok(
        err.message.includes('violates check constraint') || err.message.includes('CHECK'),
        `Expected CHECK constraint error, got: ${err.message}`,
      );
    }
  });

  await t.test('R6.2: retention_days CHECK rejects 2556', async () => {
    try {
      await pool.query(
        `INSERT INTO locations (id, org_id, slug, name, phone, status, retention_days)
         VALUES ($1, $2, $3, 'R6 Check 2556', '001', 'open', 2556)`,
        [crypto.randomUUID(), orgId, `r6-2556-${Date.now()}`],
      );
      assert.fail('Should have thrown CHECK constraint violation for retention_days = 2556');
    } catch (err: any) {
      assert.ok(
        err.message.includes('violates check constraint') || err.message.includes('CHECK'),
        `Expected CHECK constraint error, got: ${err.message}`,
      );
    }
  });

  await t.test('R6.3: retention_days defaults to 365', async () => {
    const res = await pool.query(`SELECT retention_days FROM locations WHERE id = $1`, [locId]);
    assert.strictEqual(res.rows[0].retention_days, 365);
  });

  // ═══════════════════════════════════════════════════════════════
  // R7: Functional — Retention
  // ═══════════════════════════════════════════════════════════════
  await t.test('R7.1: retention anonymizes expired customers', async () => {
    const locRet = crypto.randomUUID();
    await pool.query(
      `INSERT INTO locations (id, org_id, slug, name, phone, status, retention_days)
       VALUES ($1, $2, $3, 'P30 Retention', '777', 'open', 365) ON CONFLICT DO NOTHING`,
      [locRet, orgId, `p30-ret-${Date.now()}`],
    );

    await pool.query(
      `INSERT INTO customers (id, location_id, phone, name, no_show_count, completed_count, created_at)
       VALUES ($1, $2, '+355699990003', 'Old Customer', 1, 2, now() - interval '400 days') ON CONFLICT DO NOTHING`,
      [custIdOld, locRet],
    );

    // Retention batch mode: no subject → scans all locations
    const result = await anonymizer.anonymize({ scope: 'retention' });

    assert.ok(result.customersAnonymized >= 1, `Expected at least 1 anonymized customer, got ${result.customersAnonymized}`);

    const check = await pool.query(`SELECT anonymized_at FROM customers WHERE id = $1`, [custIdOld]);
    assert.ok(check.rows[0].anonymized_at !== null, 'old customer must be anonymized');

    await pool.query(`DELETE FROM customers WHERE id = $1`, [custIdOld]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locRet]);
  });

  await t.test('R7.2: retention skips young (non-expired) customers', async () => {
    const locRet2 = crypto.randomUUID();
    await pool.query(
      `INSERT INTO locations (id, org_id, slug, name, phone, status, retention_days)
       VALUES ($1, $2, $3, 'P30 Retention 2', '778', 'open', 365) ON CONFLICT DO NOTHING`,
      [locRet2, orgId, `p30-ret2-${Date.now()}`],
    );

    await pool.query(
      `INSERT INTO customers (id, location_id, phone, name, no_show_count, completed_count, created_at)
       VALUES ($1, $2, '+355699990004', 'Young Customer', 0, 1, now() - interval '100 days') ON CONFLICT DO NOTHING`,
      [custIdYoung, locRet2],
    );

    // Young customer is only 100 days old, retention_days is 365 → not expired
    const result = await anonymizer.anonymize({ scope: 'retention' });

    assert.strictEqual(result.customersAnonymized, 0, 'young customer must NOT be anonymized');
    assert.strictEqual(result.skipped, 0, 'no skipped customers either');

    const check = await pool.query(`SELECT anonymized_at FROM customers WHERE id = $1`, [custIdYoung]);
    assert.strictEqual(check.rows[0].anonymized_at, null, 'young customer anonymized_at must remain NULL');

    await pool.query(`DELETE FROM customers WHERE id = $1`, [custIdYoung]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locRet2]);
  });

  // ═══════════════════════════════════════════════════════════════
  // R8: Functional — GDPR
  // ═══════════════════════════════════════════════════════════════
  await t.test('R8: GDPR request flow', { skip: !serverAvail }, async () => {
    await pool.query(
      `INSERT INTO customers (id, location_id, phone, name, no_show_count, completed_count)
       VALUES ($1, $2, '+355699990005', 'GDPR Customer', 0, 0) ON CONFLICT DO NOTHING`,
      [custIdGdpr, locId],
    );

    // Create GDPR request via API
    const createRes = await fetch(`${BASE}/api/owner/locations/${locId}/gdpr-requests`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ customerId: custIdGdpr, reason: 'Test GDPR erasure request' }),
    });
    assert.strictEqual(createRes.status, 201);
    const createData = await createRes.json();
    assert.ok(createData.requestId, 'must return requestId');
    assert.strictEqual(createData.status, 'pending');
    const requestId: string = createData.requestId;

    // Verify DB has status = 'pending'
    const pendRes = await pool.query(`SELECT status FROM gdpr_erasure_requests WHERE id = $1`, [requestId]);
    assert.strictEqual(pendRes.rows[0].status, 'pending');

    // Process — simulate worker: mark in_progress, call anonymize, mark completed
    await pool.query(`UPDATE gdpr_erasure_requests SET status = 'in_progress' WHERE id = $1`, [requestId]);

    const gdprResult = await anonymizer.anonymize({
      scope: 'gdpr',
      subject: { customerId: custIdGdpr, locationId: locId },
    });
    assert.strictEqual(gdprResult.customersAnonymized, 1);

    await pool.query(
      `UPDATE gdpr_erasure_requests SET status = 'completed', completed_at = now(), metadata = $1 WHERE id = $2`,
      [JSON.stringify(gdprResult), requestId],
    );

    // Verify completed
    const compRes = await pool.query(
      `SELECT status, metadata FROM gdpr_erasure_requests WHERE id = $1`,
      [requestId],
    );
    assert.strictEqual(compRes.rows[0].status, 'completed');
    const meta = compRes.rows[0].metadata;
    assert.strictEqual(meta.customersAnonymized, 1);

    // Verify customer anonymized
    const gdprCust = await pool.query(`SELECT name, anonymized_at FROM customers WHERE id = $1`, [custIdGdpr]);
    assert.strictEqual(gdprCust.rows[0].name, null);
    assert.ok(gdprCust.rows[0].anonymized_at !== null);

    // Verify audit log via API
    const getRes = await fetch(
      `${BASE}/api/owner/locations/${locId}/gdpr-requests/${requestId}`,
      { headers: { Authorization: `Bearer ${ownerToken}` } },
    );
    assert.strictEqual(getRes.status, 200);
    const getData = await getRes.json();
    assert.strictEqual(getData.status, 'completed');
    assert.ok(Array.isArray(getData.auditLogs));
    assert.ok(getData.auditLogs.length >= 1);
  });

  // ═══════════════════════════════════════════════════════════════
  // R9: Audit log — structure, RLS, 0 PII
  // ═══════════════════════════════════════════════════════════════
  await t.test('R9: audit log has correct structure and no PII', async () => {
    const auditRes = await pool.query(
      `SELECT scope, subject_kind, subject_id, location_id, actor_kind, actor_id, metadata
       FROM anonymization_audit_log
       WHERE subject_id IN ($1, $2)
       ORDER BY created_at DESC`,
      [custId, custIdR4],
    );
    assert.ok(auditRes.rowCount >= 1, 'must have at least 1 audit log entry');

    for (const row of auditRes.rows) {
      assert.ok(['retention', 'gdpr'].includes(row.scope), `scope must be retention|gdpr, got ${row.scope}`);
      assert.strictEqual(row.subject_kind, 'customer');
      assert.ok(row.subject_id, 'subject_id must be set');
      assert.ok(row.location_id, 'location_id must be set');
      assert.ok(['system', 'owner'].includes(row.actor_kind), `actor_kind must be system|owner, got ${row.actor_kind}`);

      // Metadata must contain 0 PII
      const metaStr = JSON.stringify(row.metadata);
      assert.ok(!/\+?\d{7,15}/.test(metaStr), `Metadata must not contain phone numbers: ${metaStr}`);
      assert.ok(!/@/.test(metaStr), `Metadata must not contain emails: ${metaStr}`);
    }
  });

  // ═══════════════════════════════════════════════════════════════
  // R10: WS events — 0 PII
  // ═══════════════════════════════════════════════════════════════
  await t.test('R10: customer.anonymized event published without PII', async () => {
    const anonEvents = events.filter(e => e.event === 'customer.anonymized');
    assert.ok(anonEvents.length >= 1, 'must have published at least one customer.anonymized event');

    for (const evt of anonEvents) {
      const p = evt.payload;
      assert.ok(p.customerId, 'payload must include customerId');
      assert.ok(p.locationId, 'payload must include locationId');
      assert.ok(p.scope, 'payload must include scope');
      assert.ok(p.timestamp, 'payload must include timestamp');

      // Claim-check only — no PII (phone requires + prefix to avoid matching UUID digits)
      const payloadStr = JSON.stringify(p);
      assert.ok(!/\+[\d\-() ]{6,18}/.test(payloadStr), `Event must not contain phone numbers: ${payloadStr}`);
      assert.ok(!/@/.test(payloadStr), `Event must not contain emails: ${payloadStr}`);
      assert.ok(!payloadStr.includes('name'), 'Event must not contain customer name');
    }
  });

  // ═══════════════════════════════════════════════════════════════
  // CLEANUP
  // ═══════════════════════════════════════════════════════════════
  await t.test('cleanup test data', async () => {
    await pool.query(`DELETE FROM anonymization_audit_log WHERE location_id IN ($1, $2, $3)`, [locId, locIdB, locId2]);
    await pool.query(`DELETE FROM gdpr_erasure_requests WHERE location_id IN ($1, $2, $3)`, [locId, locIdB, locId2]);
    await pool.query(`DELETE FROM order_items WHERE order_id = $1`, [orderId]);
    await pool.query(`DELETE FROM orders WHERE id = $1`, [orderId]);
    await pool.query(`DELETE FROM products WHERE id = $1`, [prodId]);
    if (custIdAvatar) {
      await pool.query(`DELETE FROM customers WHERE id = $1`, [custIdAvatar]);
    }
    await pool.query(`DELETE FROM customers WHERE id IN ($1, $2, $3)`, [custId, custIdR4, custIdGdpr]);
    await pool.query(`DELETE FROM locations WHERE id IN ($1, $2, $3)`, [locId, locIdB, locId2]);
    await pool.query(`DELETE FROM organizations WHERE id IN ($1, $2)`, [orgId, orgId2]);
    await pool.query(`DELETE FROM users WHERE id IN ($1, $2)`, [userId, userId2]);
  });

  await pool.end();
});
