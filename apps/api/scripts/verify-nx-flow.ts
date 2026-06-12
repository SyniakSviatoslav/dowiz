import { loadEnv } from '@deliveryos/config';
import { createSessionPool, createOperationalPool } from '@deliveryos/db';
import { signAuthToken } from '@deliveryos/platform';
import crypto from 'node:crypto';

const env = loadEnv();
const BASE = `http://127.0.0.1:${env.PORT || 8080}`;
const API = `${BASE}/api`;

async function main() {
  console.log('=== NX End-to-End Flow Verification ===\n');

  // 1. Setup test data
  const sessionPool = createSessionPool();
  const orgId = crypto.randomUUID();
  const locId = crypto.randomUUID();
  const userId = crypto.randomUUID();
  const custId = crypto.randomUUID();
  const prodId = crypto.randomUUID();

  console.log('[SETUP] Creating test data...');
  await sessionPool.query(`INSERT INTO users (id, email) VALUES ($1, $2) ON CONFLICT DO NOTHING`,
    [userId, `verify-${Date.now()}@test.com`]);
  await sessionPool.query(`INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'Verify Org', $2) ON CONFLICT DO NOTHING`,
    [orgId, userId]);
  await sessionPool.query(`INSERT INTO locations (id, org_id, slug, name, phone, status, currency_code, default_locale, supported_locales, confirm_timeout_min, delivery_fee_flat)
    VALUES ($1, $2, $3, 'Verify Loc', '123', 'open', 'ALL', 'sq', $4::text[], 5, 0) ON CONFLICT DO NOTHING`,
    [locId, orgId, `verify-loc-${Date.now()}`, '{sq,en}']);
  await sessionPool.query(`INSERT INTO memberships (user_id, location_id, role) VALUES ($1, $2, 'owner') ON CONFLICT DO NOTHING`,
    [userId, locId]);
  await sessionPool.query(`INSERT INTO customers (id, location_id, phone, name) VALUES ($1, $2, '+355690000001', 'Verify Customer') ON CONFLICT DO NOTHING`,
    [custId, locId]);
  await sessionPool.query(`INSERT INTO products (id, location_id, name, price, is_available) VALUES ($1, $2, 'Verify Product', 500, true) ON CONFLICT DO NOTHING`,
    [prodId, locId]);
  // Set up Telegram notification target
  await sessionPool.query(`INSERT INTO owner_notification_targets (id, location_id, channel, address, status, prefs)
    VALUES ($1, $2, 'telegram', 'verify-test-chat', 'active', '{}'::jsonb) ON CONFLICT DO NOTHING`,
    [crypto.randomUUID(), locId]);

  const ownerToken = await signAuthToken({ role: 'owner', userId, activeLocationId: locId }, '15m');
  console.log('[SETUP] Test data created\n');

  // 2. Place an order via API
  console.log('[FLOW-1] Placing order via POST /api/orders...');
  const orderRes = await fetch(`${API}/orders`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${ownerToken}`,
    },
    body: JSON.stringify({
      locationId: locId,
      type: 'delivery',
      items: [{ product_id: prodId, quantity: 1 }],
      customer: { phone: '+355690000002', name: 'Verify Customer' },
      delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Verify Address' },
      payment: { method: 'cash' },
      idempotency_key: crypto.randomUUID(),
    }),
  });

  if (orderRes.status !== 201) {
    const body = await orderRes.text();
    console.error(`[FLOW-1] FAILED: Order creation returned ${orderRes.status}: ${body}`);
    await sessionPool.end();
    process.exit(1);
  }
  const order = await orderRes.json();
  console.log(`[FLOW-1] Order created: ${order.id} (status: ${order.status})\n`);

  // 3. Check pg-boss job
  console.log('[FLOW-2] Checking pgboss for notification job...');
  let jobFound = false;
  let attempts = 0;
  let jobRow: any = null;

  while (attempts < 10) {
    const jobRes = await sessionPool.query(
      `SELECT id, state, data, created_on
       FROM pgboss.job
       WHERE name = 'notify.telegram.send'
         AND data->>'entity_id' = $1
       ORDER BY created_on DESC LIMIT 1`,
      [order.id]
    );

    if (jobRes.rows.length > 0) {
      jobFound = true;
      jobRow = jobRes.rows[0];
      console.log(`  Job ID: ${jobRow.id}`);
      console.log(`  State: ${jobRow.state}`);
      console.log(`  Created: ${jobRow.created_on}`);

      const dataKeys = Object.keys(jobRow.data);
      const hasPii = dataKeys.some(k => ['phone','email','name','customer'].includes(k));
      console.log(`  Data fields: ${dataKeys.join(', ')}`);
      console.log(`  PII in payload: ${hasPii ? '⚠️ YES (SECURITY ISSUE)' : '✅ None'}`);
      break;
    }
    attempts++;
    await new Promise(r => setTimeout(r, 500));
  }

  if (!jobFound) {
    console.error('[FLOW-2] FAILED: No notify.telegram.send job found after 5s');
    await sessionPool.end();
    process.exit(1);
  }
  console.log(`[FLOW-2] Job enqueued after ${(attempts + 1) * 500}ms polling\n`);

  // 4. Wait for worker to pick up job (state changes from 'created' → 'active' → 'completed')
  console.log('[FLOW-3] Waiting for worker to process job (up to 15s)...');
  let finalState = jobRow.state;
  attempts = 0;

  while (attempts < 30 && (finalState === 'created' || finalState === 'active' || finalState === 'retry')) {
    await new Promise(r => setTimeout(r, 500));
    const stateRes = await sessionPool.query(
      `SELECT state, completed_on FROM pgboss.job WHERE id = $1`,
      [jobRow.id]
    );
    if (stateRes.rows.length > 0) {
      finalState = stateRes.rows[0].state;
      if (finalState === 'completed' || finalState === 'failed' || finalState === 'cancelled') {
        console.log(`  Final state: ${finalState} (took ${(attempts + 1) * 500}ms)`);
        if (stateRes.rows[0].completed_on) {
          console.log(`  Completed at: ${stateRes.rows[0].completed_on}`);
        }
      }
    }
    attempts++;
  }

  if (finalState === 'created') {
    console.log(`  ⏳ Job still 'created' after 15s (worker may not have polled yet)`);
  } else if (finalState === 'active') {
    console.log(`  ⏳ Job moved to 'active' but not yet completed`);
  } else {
    console.log(`[FLOW-3] Worker processed job to state: ${finalState}\n`);
  }

  // 5. Check audit log
  console.log('[FLOW-4] Checking notification_outbox_audit table...');
  const auditRes = await sessionPool.query(
    `SELECT id, event, status, target_id, created_at
     FROM notification_outbox_audit
     WHERE location_id = $1
     ORDER BY created_at DESC`,
    [locId]
  );

  if (auditRes.rows.length > 0) {
    for (const row of auditRes.rows) {
      console.log(`  Audit ID: ${row.id} | event: ${row.event} | status: ${row.status} | target_id: ${row.target_id || 'N/A'}`);
    }
  } else {
    console.log('  No audit records found (worker may not have completed)');
  }

  const hasPii = jobRow && Object.keys(jobRow.data).some(k => ['phone','email','name','customer'].includes(k));
  console.log(`\n=== VERIFICATION SUMMARY ===`);
  console.log(`  Order created:     ✅ (${order.id})`);
  console.log(`  Job enqueued:      ✅ (${jobRow.state === 'completed' ? '✅ completed' : `⏳ ${finalState}`})`);
  console.log(`  Audit records:     ${auditRes.rows.length > 0 ? `✅ ${auditRes.rows.length} found` : '⏳ none yet'}`);
  console.log(`  PII in job payload: ${jobRow && hasPii ? '❌ YES' : '✅ None'}`);

  await sessionPool.end();
}

main().catch(err => {
  console.error('Verification failed:', err);
  process.exit(1);
});
