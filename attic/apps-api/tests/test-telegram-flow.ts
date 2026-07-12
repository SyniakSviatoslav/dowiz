import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';
import crypto from 'crypto';
import { signAuthToken } from '@deliveryos/platform';

const env = loadEnv();

const API_BASE = 'http://127.0.0.1:3003';
const BOT_SECRET = env.***REDACTED*** || 'webhook-test-secret';
const WEBHOOK_URL = `${API_BASE}/webhook/telegram/${BOT_SECRET}`;

interface TestResult {
  name: string;
  passed: boolean;
  error?: string;
}

const results: TestResult[] = [];

function assert(condition: boolean, name: string, error?: string) {
  if (condition) {
    results.push({ name, passed: true });
    console.log(`  ✅ ${name}`);
  } else {
    results.push({ name, passed: false, error });
    console.log(`  ❌ ${name}: ${error || 'assertion failed'}`);
  }
}

async function sendTelegramUpdate(body: any, headers?: Record<string, string>) {
  return fetch(WEBHOOK_URL, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'x-telegram-bot-api-secret-token': BOT_SECRET,
      ...headers,
    },
    body: JSON.stringify(body),
  });
}

async function runTests() {
  const pool = createSessionPool();
  const client = await pool.connect();

  try {
    console.log('\n═══════════════════════════════════════');
    console.log('  Telegram Integration Tests');
    console.log('═══════════════════════════════════════\n');

    // ── Setup: create test data ──
    console.log('📦 Setup: creating test data...');

    const orgId = crypto.randomUUID();
    const userId = crypto.randomUUID();
    const locId = crypto.randomUUID();
    const orderId = crypto.randomUUID();

    await client.query(`DELETE FROM idempotency_keys`);

    const userInsert = await client.query(
      `INSERT INTO users (id, email) VALUES ($1, $2) RETURNING id`,
      [userId, `telegram-test-${Date.now()}@test.com`]
    );
    assert(userInsert.rows.length === 1, 'User created', `Got ${userInsert.rows.length} rows`);

    const orgInsert = await client.query(
      `INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'TG Test Org', $2) RETURNING id`,
      [orgId, userId]
    );
    assert(orgInsert.rows.length === 1, 'Org created');

    // Insert location with unique slug to avoid ON CONFLICT
    const uniqueSlug = `tg-test-loc-${Date.now()}`;
    await client.query(
      `INSERT INTO locations (id, org_id, slug, name, phone, status, busy_mode, confirm_timeout_min,
         menu_version, widget_enabled, customer_otp_required, default_locale, supported_locales,
         currency_code, currency_minor_unit, tax_rate, price_includes_tax, require_phone_otp,
         onboarding_state, dwell_thresholds, retention_days, fallback_config, rate_limit_overrides)
        VALUES ($1, $2, $3, 'TG Test Loc', '123', 'open', false, 1,
          1, false, false, 'en', ARRAY['en']::text[],
          'ALL', 2, 0, false, false,
          '{}'::jsonb, '{"dwell_warn_min":15,"dwell_critical_min":30}'::jsonb, 30, '{}'::jsonb, '{}'::jsonb)
        RETURNING id`,
      [locId, orgId, uniqueSlug]
    );

    await client.query(
      `INSERT INTO memberships (user_id, location_id, role, status)
       VALUES ($1, $2, 'owner', 'active') ON CONFLICT DO NOTHING`,
      [userId, locId]
    );

    console.log('  Test data created.\n');

    // ── T-1: Account Linking ──
    console.log('── T-1: Account Linking ──');

    // 1a. Create a connect token via the owner route
    const tokenRes = await fetch(`${API_BASE}/api/owner/locations/${locId}/notifications/telegram/connect-init`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${await signAuthToken({ role: 'owner', userId }, '15m')}`,
      },
    });
    const tokenData = await tokenRes.json() as any;
    const connectToken = tokenData.token;

    assert(!!connectToken, 'Connect token created', `Got: ${JSON.stringify(tokenData)}`);

    // 1b. Simulate /start <token> from Telegram
    const startRes = await sendTelegramUpdate({
      message: {
        text: `/start ${connectToken}`,
        chat: { id: 999001, type: 'private' },
        from: { id: 999001, first_name: 'Test' },
      },
    });
    assert(startRes.status === 200, '/start returns 200');

    // Verify linkage was created
    const linkageRes = await client.query(
      `SELECT * FROM owner_notification_targets
       WHERE location_id = $1 AND channel = 'telegram' AND address = '999001'`,
      [locId]
    );
    assert(linkageRes.rows.length === 1, 'Linkage row created', `Found ${linkageRes.rows.length} rows`);
    assert(linkageRes.rows[0]?.status === 'active', 'Linkage status is active');
    assert(linkageRes.rows[0]?.user_id === userId, 'Linkage user_id matches owner');

    // 1c. Duplicate /start should upsert (not fail)
    const startRes2 = await sendTelegramUpdate({
      message: {
        text: `/start ${connectToken}`,
        chat: { id: 999001, type: 'private' },
        from: { id: 999001, first_name: 'Test' },
      },
    });
    assert(startRes2.status === 200, 'Duplicate /start returns 200');

    // 1d. /stop disconnects
    const stopRes = await sendTelegramUpdate({
      message: {
        text: '/stop',
        chat: { id: 999001, type: 'private' },
        from: { id: 999001, first_name: 'Test' },
      },
    });
    assert(stopRes.status === 200, '/stop returns 200');

    const disabledRes = await client.query(
      `SELECT status FROM owner_notification_targets
       WHERE location_id = $1 AND channel = 'telegram' AND address = '999001'`,
      [locId]
    );
    assert(disabledRes.rows[0]?.status === 'disabled', 'Target disabled after /stop');

    // 1e. Re-link for further tests
    const reStartRes = await sendTelegramUpdate({
      message: {
        text: `/start ${connectToken}`,
        chat: { id: 999001, type: 'private' },
        from: { id: 999001, first_name: 'Test' },
      },
    });
    assert(reStartRes.status === 200, 'Re-link /start returns 200');

    // 1f. Invalid token
    const badTokenRes = await sendTelegramUpdate({
      message: {
        text: `/start 00000000-0000-0000-0000-000000000000`,
        chat: { id: 999001, type: 'private' },
        from: { id: 999001, first_name: 'Test' },
      },
    });
    assert(badTokenRes.status === 200, 'Invalid token still returns 200 (best-effort)');

    console.log('');

    // ── T-2: Webhook Security ──
    console.log('── T-2: Webhook Security ──');

    // 2a. Missing secret token
    const noSecretRes = await sendTelegramUpdate(
      { message: { text: '/stop', chat: { id: 999001 }, from: { id: 999001 } } },
      { 'x-telegram-bot-api-secret-token': '' }
    );
    assert(noSecretRes.status === 401, 'Missing secret → 401');

    // 2b. Wrong secret token
    const wrongSecretRes = await sendTelegramUpdate(
      { message: { text: '/stop', chat: { id: 999001 }, from: { id: 999001 } } },
      { 'x-telegram-bot-api-secret-token': 'wrong-secret' }
    );
    assert(wrongSecretRes.status === 401, 'Wrong secret → 401');

    // 2c. Correct secret
    const correctSecretRes = await sendTelegramUpdate({
      message: { text: '/stop', chat: { id: 999001 }, from: { id: 999001 } },
    });
    assert(correctSecretRes.status === 200, 'Correct secret → 200');

    // Re-link again
    await sendTelegramUpdate({
      message: {
        text: `/start ${connectToken}`,
        chat: { id: 999001, type: 'private' },
        from: { id: 999001, first_name: 'Test' },
      },
    });

    console.log('');

    // ── T-3: Inbound Callback Queries ──
    console.log('── T-3: Inbound Callback Queries ──');

    // Create an order to act on
    await client.query("SELECT set_config('app.current_tenant', $1, true)", [locId]);
    const prodId = crypto.randomUUID();
    await client.query(
      `INSERT INTO products (id, location_id, name, price, is_available)
       VALUES ($1, $2, 'Pizza', 800, true) ON CONFLICT DO NOTHING`,
      [prodId, locId]
    );

     await client.query(
      `INSERT INTO orders (id, location_id, status, type, subtotal, total, payment_method, payment_outcome, preferences)
        VALUES ($1, $2, 'PENDING', 'delivery', 1600, 1600, 'cash', 'pending', '{}'::jsonb)
        ON CONFLICT DO NOTHING`,
      [orderId, locId]
    );

    // 3a. Confirm order via callback
    const confirmRes = await sendTelegramUpdate({
      callback_query: {
        id: 'cb_test_1',
        data: `order.confirm:${orderId}`,
        from: { id: 999001, first_name: 'Test' },
        message: {
          chat: { id: 999001 },
          message_id: 100,
          text: 'New order #test',
        },
      },
    });
    assert(confirmRes.status === 200, 'Confirm callback → 200');

    const orderStatus = await client.query(
      `SELECT status FROM orders WHERE id = $1`,
      [orderId]
    );
    assert(orderStatus.rows[0]?.status === 'CONFIRMED', 'Order confirmed');

    // 3b. Confirm again (idempotent)
    const confirmRes2 = await sendTelegramUpdate({
      callback_query: {
        id: 'cb_test_2',
        data: `order.confirm:${orderId}`,
        from: { id: 999001, first_name: 'Test' },
        message: { chat: { id: 999001 }, message_id: 101, text: '...' },
      },
    });
    assert(confirmRes2.status === 200, 'Double confirm → 200 (idempotent)');

    // 3c. Reject via reason button on new order
    const rejectOrderId = crypto.randomUUID();
    await client.query(
      `INSERT INTO orders (id, location_id, status, type, subtotal, total, payment_method, payment_outcome, preferences)
        VALUES ($1, $2, 'PENDING', 'delivery', 1600, 1600, 'cash', 'pending', '{}'::jsonb)
        ON CONFLICT DO NOTHING`,
      [rejectOrderId, locId]
    );

    const rejectRes = await sendTelegramUpdate({
      callback_query: {
        id: 'cb_test_3',
        data: `order.reject_reason_1:${rejectOrderId}`,
        from: { id: 999001, first_name: 'Test' },
        message: { chat: { id: 999001 }, message_id: 102, text: '...' },
      },
    });
    assert(rejectRes.status === 200, 'Reject with reason → 200');

    const rejectStatus = await client.query(
      `SELECT status FROM orders WHERE id = $1`,
      [rejectOrderId]
    );
    assert(rejectStatus.rows[0]?.status === 'REJECTED', 'Order rejected');

    // 3d. Unlinked user (different chat) → callback rejected
    const unlinkedRes = await sendTelegramUpdate({
      callback_query: {
        id: 'cb_test_4',
        data: `order.confirm:${orderId}`,
        from: { id: 123456, first_name: 'Hacker' },
        message: { chat: { id: 123456 }, message_id: 103, text: '...' },
      },
    });
    assert(unlinkedRes.status === 200, 'Unlinked callback → 200 (best-effort, answers "not linked")');

    // 3e. Unknown action
    const unknownRes = await sendTelegramUpdate({
      callback_query: {
        id: 'cb_test_5',
        data: 'unknown_action:123',
        from: { id: 999001, first_name: 'Test' },
        message: { chat: { id: 999001 }, message_id: 104, text: '...' },
      },
    });
    assert(unknownRes.status === 200, 'Unknown action → 200 (best-effort)');

    console.log('');

    // ── T-4: PII Absence ──
    console.log('── T-4: PII Absence in Webhook ──');

    // Verify no customer PII leaked into callback data
    const piiRes = await client.query(
      `SELECT * FROM owner_notification_targets WHERE address = '999001'`
    );
    assert(piiRes.rows.length > 0, 'Target exists for PII check');
    assert(!piiRes.rows[0]?.address?.includes('@'), 'No email in Telegram address field');
    assert(!piiRes.rows[0]?.address?.includes('+'), 'No phone in Telegram address field');

    console.log('');

    // ── T-5: Best-Effort Principles ──
    console.log('── T-5: Best-Effort Principles ──');

    // 5a. Malformed body → still returns 200 to Telegram
    const malformedRes = await sendTelegramUpdate({ invalid: 'json' });
    assert(malformedRes.status === 200, 'Malformed body → 200 (don\'t block Telegram)');

    // 5b. Empty body
    const emptyRes = await fetch(WEBHOOK_URL, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-telegram-bot-api-secret-token': BOT_SECRET,
      },
      body: '',
    });
    assert(emptyRes.status === 200 || emptyRes.status === 400, 'Empty body handled gracefully');

    // 5c. Callback on non-existent order
    const ghostOrderRes = await sendTelegramUpdate({
      callback_query: {
        id: 'cb_test_ghost',
        data: `order.confirm:${crypto.randomUUID()}`,
        from: { id: 999001, first_name: 'Test' },
        message: { chat: { id: 999001 }, message_id: 105, text: '...' },
      },
    });
    assert(ghostOrderRes.status === 200, 'Ghost order callback → 200 (best-effort)');

    console.log('');

    // ── T-6: Tenant Isolation ──
    console.log('── T-6: Tenant Isolation ──');

    // Create a second location + order
    const locId2 = crypto.randomUUID();
    const orgId2 = crypto.randomUUID();
    const userId2 = crypto.randomUUID();
    await client.query(
      `INSERT INTO users (id, email) VALUES ($1, 'tg-test-2@test.com') ON CONFLICT DO NOTHING`,
      [userId2]
    );
    await client.query(
      `INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'TG Test Org 2', $2) ON CONFLICT DO NOTHING`,
      [orgId2, userId2]
    );
    const uniqueSlug2 = `tg-test-loc-2-${Date.now()}`;
    await client.query(
      `INSERT INTO locations (id, org_id, slug, name, phone, status, busy_mode, confirm_timeout_min,
         menu_version, widget_enabled, customer_otp_required, default_locale, supported_locales,
         currency_code, currency_minor_unit, tax_rate, price_includes_tax, require_phone_otp,
         onboarding_state, dwell_thresholds, retention_days, fallback_config, rate_limit_overrides)
        VALUES ($1, $2, $3, 'TG Test Loc 2', '456', 'open', false, 1,
          1, false, false, 'en', ARRAY['en']::text[],
          'ALL', 2, 0, false, false,
          '{}'::jsonb, '{}'::jsonb, 30, '{}'::jsonb, '{}'::jsonb)
        RETURNING id`,
      [locId2, orgId2, uniqueSlug2]
    );

    const crossLocOrderId = crypto.randomUUID();
    await client.query(
      `INSERT INTO orders (id, location_id, status, type, subtotal, total, payment_method, payment_outcome, preferences)
        VALUES ($1, $2, 'PENDING', 'delivery', 1600, 1600, 'cash', 'pending', '{}'::jsonb)
        ON CONFLICT DO NOTHING`,
      [crossLocOrderId, locId2]
    );

    // Try to confirm order from loc2 using loc1's linked Telegram
    const crossLocRes = await sendTelegramUpdate({
      callback_query: {
        id: 'cb_test_cross',
        data: `order.confirm:${crossLocOrderId}`,
        from: { id: 999001, first_name: 'Test' },
        message: { chat: { id: 999001 }, message_id: 106, text: '...' },
      },
    });
    assert(crossLocRes.status === 200, 'Cross-tenant callback → 200 (best-effort, answers "Unauthorized")');

    const crossLocStatus = await client.query(
      `SELECT status FROM orders WHERE id = $1`,
      [crossLocOrderId]
    );
    assert(crossLocStatus.rows[0]?.status === 'PENDING', 'Cross-tenant order unchanged');

    console.log('');

    // ── Summary ──
    const passed = results.filter(r => r.passed).length;
    const failed = results.filter(r => !r.passed).length;
    console.log('═══════════════════════════════════════');
    console.log(`  Results: ${passed} passed, ${failed} failed, ${results.length} total`);
    console.log('═══════════════════════════════════════');

    if (failed > 0) {
      console.log('\nFailed tests:');
      results.filter(r => !r.passed).forEach(r => {
        console.log(`  ❌ ${r.name}: ${r.error}`);
      });
      process.exit(1);
    }
  } finally {
    await client.release();
    await pool.end();
  }
}

runTests().catch(err => {
  console.error('Test runner failed:', err);
  process.exit(1);
});
