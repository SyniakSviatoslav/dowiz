import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const BOT_SECRET = process.env.TELEGRAM_BOT_SECRET;
const WEBHOOK_URL = `${BASE}/webhook/telegram/${BOT_SECRET}`;
const BOT_TOKEN = process.env.TELEGRAM_BOT_TOKEN;

let authToken: string;
let userId: string;
let locationId: string;
let locationSlug: string;
let connectToken: string;
let productId: string;
let notificationTargetId: string;

const CHAT_ID = 999999;
const TEST_SLUG = `tg-e2e-${Date.now()}`;

function uuid() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => {
    const r = Math.random() * 16 | 0;
    return (c === 'x' ? r : (r & 0x3 | 0x8)).toString(16);
  });
}

async function sendWebhook(data: any): Promise<number> {
  const resp = await fetch(WEBHOOK_URL, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'x-telegram-bot-api-secret-token': BOT_SECRET,
    },
    body: JSON.stringify(data),
  });
  return resp.status;
}

async function authHeaders() {
  return { Authorization: `Bearer ${authToken}`, 'Content-Type': 'application/json' };
}

test.describe('Telegram Complete Flow — Live (VITE_BASE_URL, staging-default)', () => {

  test.describe.configure({ mode: 'serial' });

  // This suite MUTATES state (creates a location, products, links a Telegram chat) and
  // exercises the webhook secret path. Fail fast if pointed at prod, or if the bot env is
  // absent — otherwise WEBHOOK_URL would end in '/undefined' and the secret-mismatch checks
  // (P6-NO-SECRET / P6-WRONG-SECRET) would false-green. (Test Integrity #6.)
  test.beforeAll(() => {
    requireStaging(BASE);
    expect(BOT_SECRET, 'TELEGRAM_BOT_SECRET must be set').toMatch(/^[\w-]{1,256}$/);
    expect(BOT_TOKEN, 'TELEGRAM_BOT_TOKEN must be set').toMatch(/^\d+:[\w-]+$/);
  });

  // ════════════════════════════════════════════════════════════════
  // PHASE 1: SETUP — Auth + Location + Product
  // ════════════════════════════════════════════════════════════════

  test('P1-AUTH: mock-auth returns valid owner token', async ({ request }) => {
    const res = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expectJwt(body.access_token, 'access_token');
    authToken = body.access_token;
    userId = body.userId;
  });

  test('P1-LOCATION: create test location via onboarding', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/onboarding/start`, {
      headers: await authHeaders(),
      data: {
        name: 'TG E2E Test',
        phone: '+355600000000',
        slug: TEST_SLUG,
        currency_code: 'ALL',
        default_locale: 'sq',
        supported_locales: ['sq', 'en'],
      },
    });
    expect(res.status()).toBe(201);
    const body = await res.json();
    expectUuid(body.locationId, 'locationId');
    locationId = body.locationId;
    locationSlug = body.slug;
  });

  test('P1-ONBOARD-1: complete step 1 (Location Basics)', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
      headers: await authHeaders(),
      data: { step: 1 },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.currentStep).toBe(2);
  });

  test('P1-ONBOARD-2: complete step 2 (Import Menu)', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
      headers: await authHeaders(),
      data: { step: 2 },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.currentStep).toBe(3);
  });

  test('P1-ONBOARD-3: complete step 3 (Review & Fix Menu)', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
      headers: await authHeaders(),
      data: { step: 3 },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.currentStep).toBe(4);
  });

  test('P1-ONBOARD-4: skip step 4 (Branding)', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/onboarding/${locationId}/step/4/skip`, {
      headers: await authHeaders(),
    });
    expect(res.status()).toBe(200);
  });

  test('P1-ONBOARD-5: skip step 5 (Delivery Settings)', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/onboarding/${locationId}/step/5/skip`, {
      headers: await authHeaders(),
    });
    expect(res.status()).toBe(200);
  });

  test('P1-ONBOARD-6: complete step 6 (Publish & Share)', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
      headers: await authHeaders(),
      data: { step: 6 },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.currentStep).toBe(7);
  });

  test('P1-ONBOARD-7: skip step 7 (Telegram Alerts — will test separately)', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/onboarding/${locationId}/step/7/skip`, {
      headers: await authHeaders(),
    });
    expect(res.status()).toBe(200);
  });

  test('P1-ONBOARD-8: complete step 8 (Test Order & Go Live)', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
      headers: await authHeaders(),
      data: { step: 8 },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.completed).toBe(true);
  });

  test('P1-PRODUCT: create test product', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/locations/${locationId}/products`, {
      headers: await authHeaders(),
      data: {
        name: 'TG E2E Pizza',
        price: 1200,
        category_id: null,
        available: true,
      },
    });
    expect(res.status()).toBe(201);
    const body = await res.json();
    productId = body.id;
  });

  // ════════════════════════════════════════════════════════════════
  // PHASE 2: TELEGRAM CONNECT
  // ════════════════════════════════════════════════════════════════

  test('P2-CONNECT-INIT: generate Telegram connect token', async ({ request }) => {
    const res = await request.post(
      `${BASE}/api/owner/locations/${locationId}/notifications/telegram/connect-init`,
      { headers: await authHeaders() },
    );
    expect(res.status()).toBe(200);
    const body = await res.json();
    expectUuid(body.token, 'connectToken');
    connectToken = body.token;
  });

  test('P2-START: simulate /start <token> from Telegram', async () => {
    const status = await sendWebhook({
      message: {
        text: `/start ${connectToken}`,
        chat: { id: CHAT_ID, type: 'private' },
        from: { id: CHAT_ID, first_name: 'E2E', last_name: 'Tester' },
      },
    });
    expect(status).toBe(200);
  });

  test('P2-VERIFY: notification target is active after /start', async ({ request }) => {
    const res = await request.get(
      `${BASE}/api/owner/locations/${locationId}/notifications/targets`,
      { headers: await authHeaders() },
    );
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.targets).toBeDefined();
    console.log('P2-VERIFY targets:', JSON.stringify(body.targets));
    const tgTargets = body.targets.filter((t: any) => t.channel === 'telegram');
    expect(tgTargets.length).toBeGreaterThanOrEqual(1);
    const ourTarget = tgTargets.find((t: any) => t.address === String(CHAT_ID));
    expect(ourTarget).toBeTruthy();
    expect(ourTarget.status).toBe('active');
    notificationTargetId = ourTarget.id;
  });

  test('P2-DUPLICATE-START: duplicate /start returns 200 (idempotent)', async () => {
    const status = await sendWebhook({
      message: {
        text: `/start ${connectToken}`,
        chat: { id: CHAT_ID, type: 'private' },
        from: { id: CHAT_ID, first_name: 'E2E', last_name: 'Tester' },
      },
    });
    expect(status).toBe(200);
  });

  test('P2-INVALID-TOKEN: /start with invalid token returns 200 (best-effort)', async () => {
    const status = await sendWebhook({
      message: {
        text: `/start 00000000-0000-0000-0000-000000000000`,
        chat: { id: 888888, type: 'private' },
        from: { id: 888888, first_name: 'Bad' },
      },
    });
    expect(status).toBe(200);
  });

  // ════════════════════════════════════════════════════════════════
  // PHASE 3: TEST NOTIFICATION DISPATCH
  // ════════════════════════════════════════════════════════════════

  test('P3-TEST-NOTIFY: test notification endpoint enqueues dispatch job', async ({ request }) => {
    const res = await request.post(
      `${BASE}/api/owner/locations/${locationId}/notifications/test`,
      {
        headers: await authHeaders(),
        data: { targetId: notificationTargetId },
      },
    );
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.enqueued).toBe(1);
    // TODO(needs_staging): this only proves the job was queued, not delivered. There is no
    // last_sent_at column on owner_notification_targets to read back, so actual delivery via
    // the live notify worker can only be verified against a running staging worker. (Finding #5.)
  });

  // ════════════════════════════════════════════════════════════════
  // PHASE 4: CALLBACK ACTIONS (best-effort — handler always returns 200)
  // ════════════════════════════════════════════════════════════════

  test('P4-UNKNOWN-ACTION: unknown callback action returns 200', async () => {
    const status = await sendWebhook({
      callback_query: {
        id: 'cb_unknown',
        data: `unknown_action:none`,
        from: { id: CHAT_ID, first_name: 'E2E' },
        message: { chat: { id: CHAT_ID }, message_id: 203, text: '...' },
      },
    });
    expect(status).toBe(200);
  });

  test('P4-GHOST-ORDER: callback on non-existent order returns 200', async () => {
    const status = await sendWebhook({
      callback_query: {
        id: 'cb_ghost',
        data: `order.confirm:${uuid()}`,
        from: { id: CHAT_ID, first_name: 'E2E' },
        message: { chat: { id: CHAT_ID }, message_id: 204, text: '...' },
      },
    });
    expect(status).toBe(200);
  });

  test('P4-UNLINKED-USER: unlinked chat gets 200 (internally unauthorized)', async ({ request }) => {
    const status = await sendWebhook({
      callback_query: {
        id: 'cb_unlinked',
        data: `order.confirm:none`,
        from: { id: 444444, first_name: 'Hacker' },
        message: { chat: { id: 444444 }, message_id: 205, text: '...' },
      },
    });
    expect(status).toBe(200);
    // The 200 is best-effort; prove the unlinked chat was NOT processed/linked — no target
    // for chat 444444 may exist on our location. (Finding #3.)
    const res = await request.get(
      `${BASE}/api/owner/locations/${locationId}/notifications/targets`,
      { headers: await authHeaders() },
    );
    expect(res.status()).toBe(200);
    const body = await res.json();
    const hacker = body.targets.find((t: any) => t.address === '444444');
    expect(hacker).toBeUndefined();
  });

  // ════════════════════════════════════════════════════════════════
  // PHASE 5: DISCONNECT
  // ════════════════════════════════════════════════════════════════

  test('P5-STOP: /stop disconnects Telegram', async () => {
    const status = await sendWebhook({
      message: {
        text: '/stop',
        chat: { id: CHAT_ID, type: 'private' },
        from: { id: CHAT_ID, first_name: 'E2E' },
      },
    });
    expect(status).toBe(200);
  });

  test('P5-VERIFY-DISABLED: target is disabled after /stop', async ({ request }) => {
    const res = await request.get(
      `${BASE}/api/owner/locations/${locationId}/notifications/targets`,
      { headers: await authHeaders() },
    );
    expect(res.status()).toBe(200);
    const body = await res.json();
    const ourTarget = body.targets.find((t: any) => t.address === String(CHAT_ID));
    expect(ourTarget).toBeTruthy();
    expect(ourTarget.status).toBe('disabled');
  });

  test('P5-RE-LINK: can re-connect after /stop', async () => {
    // Generate new connect token
    const res = await fetch(
      `${BASE}/api/owner/locations/${locationId}/notifications/telegram/connect-init`,
      { method: 'POST', headers: await authHeaders() },
    );
    expect(res.status).toBe(200);
    const body = await res.json();
    const newToken = body.token;

    // /start with new token
    const status = await sendWebhook({
      message: {
        text: `/start ${newToken}`,
        chat: { id: CHAT_ID, type: 'private' },
        from: { id: CHAT_ID, first_name: 'E2E' },
      },
    });
    expect(status).toBe(200);

    // Prove the re-link actually flipped the target back to 'active' (it was 'disabled'
    // after /stop) — the 200 alone is best-effort. (Finding #6.)
    const verify = await fetch(
      `${BASE}/api/owner/locations/${locationId}/notifications/targets`,
      { headers: await authHeaders() },
    );
    expect(verify.status).toBe(200);
    const verifyBody = await verify.json();
    const ourTarget = verifyBody.targets.find((t: any) => t.address === String(CHAT_ID));
    expect(ourTarget).toBeTruthy();
    expect(ourTarget.status).toBe('active');
  });

  // ════════════════════════════════════════════════════════════════
  // PHASE 6: WEBHOOK SECURITY
  // ════════════════════════════════════════════════════════════════

  test('P6-NO-SECRET: missing x-telegram-bot-api-secret-token returns 401', async () => {
    const resp = await fetch(WEBHOOK_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ message: { text: '/stop', chat: { id: CHAT_ID }, from: { id: CHAT_ID } } }),
    });
    expect(resp.status).toBe(401);
  });

  test('P6-WRONG-SECRET: wrong secret token returns 401', async () => {
    const resp = await fetch(WEBHOOK_URL, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-telegram-bot-api-secret-token': 'wrong-secret-here',
      },
      body: JSON.stringify({ message: { text: '/stop', chat: { id: CHAT_ID }, from: { id: CHAT_ID } } }),
    });
    expect(resp.status).toBe(401);
  });

  test('P6-MALFORMED: malformed body returns 200 (best-effort)', async () => {
    const resp = await fetch(WEBHOOK_URL, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-telegram-bot-api-secret-token': BOT_SECRET,
      },
      body: JSON.stringify({ invalid: 'payload' }),
    });
    expect(resp.status).toBe(200);
  });

  test('P6-EMPTY-BODY: empty body handled gracefully', async ({ request }) => {
    const resp = await request.post(WEBHOOK_URL, {
      headers: {
        'Content-Type': 'application/json',
        'x-telegram-bot-api-secret-token': BOT_SECRET,
      },
      data: '',
    });
    expect(resp.status()).toBe(200);
  });

  test('P6-NO-COOKIE: webhook endpoint sets no cookies', async ({ request }) => {
    const resp = await request.post(WEBHOOK_URL, {
      headers: {
        'Content-Type': 'application/json',
        'x-telegram-bot-api-secret-token': BOT_SECRET,
      },
      data: { message: { text: '/stop', chat: { id: CHAT_ID }, from: { id: CHAT_ID } } },
    });
    const cookies = resp.headers()['set-cookie'];
    expect(cookies).toBeUndefined();
  });

  test('P6-BOT-INFO: Bot API getMe returns valid bot info', async ({ request }) => {
    const resp = await request.get(`https://api.telegram.org/bot${BOT_TOKEN}/getMe`);
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.ok).toBe(true);
    expect(body.result.username).toBe('dowizbot_bot');
    expect(body.result.is_bot).toBe(true);
    expect(body.result.can_join_groups).toBe(true);
  });

  test('P6-WEBHOOK-INFO: Telegram webhook is configured correctly', async ({ request }) => {
    const resp = await request.get(`https://api.telegram.org/bot${BOT_TOKEN}/getWebhookInfo`);
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.ok).toBe(true);
    expect(body.result.url).toBe(WEBHOOK_URL);
    expect(body.result.pending_update_count).toBe(0);
    expect(body.result.allowed_updates).toContain('message');
    expect(body.result.allowed_updates).toContain('callback_query');
  });

  // TODO(needs_staging): cross-tenant IDOR on GET .../notifications/targets — requireLocationAccess
  // returns 404 for an owner without an active membership (apps/api/src/plugins/auth.ts:153).
  // Cannot be tested here: /dev/mock-auth always mints the SAME owner (fixed email
  // dev@deliveryos.com, ON CONFLICT DO UPDATE), so a "second" token has legitimate access.
  // A real second seeded tenant is required to assert the 404. (Finding #4.)

  // ════════════════════════════════════════════════════════════════
  // PHASE 7: NOTIFICATIONS SETTINGS UI
  // ════════════════════════════════════════════════════════════════

  test('P7-NOTIF-LIST: notification target list includes Telegram', async ({ request }) => {
    const res = await request.get(
      `${BASE}/api/owner/locations/${locationId}/notifications/targets`,
      { headers: await authHeaders() },
    );
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.targets).toBeDefined();
    const tg = body.targets.find((t: any) => t.channel === 'telegram');
    expect(tg).toBeTruthy();
    expect(tg.address).toBe(String(CHAT_ID));
    notificationTargetId = tg.id;
  });

  test('P7-NOTIF-UPDATE: can toggle notification preference', async ({ request }) => {
    const res = await request.put(
      `${BASE}/api/owner/locations/${locationId}/notifications/targets/${notificationTargetId}`,
      {
        headers: await authHeaders(),
        data: { status: 'active', prefs: { order_created: true, order_substitution_needs_human: true } },
      },
    );
    expect(res.status()).toBe(200);

    // Verify the PUT actually persisted (status 200 alone is not proof) — read the target back
    // and assert the pref landed in the prefs column. (Finding #7 / Test Integrity #9.)
    const after = await request.get(
      `${BASE}/api/owner/locations/${locationId}/notifications/targets`,
      { headers: await authHeaders() },
    );
    expect(after.status()).toBe(200);
    const afterBody = await after.json();
    const updated = afterBody.targets.find((t: any) => t.id === notificationTargetId);
    expect(updated).toBeTruthy();
    expect(updated.prefs.order_created).toBe(true);
    expect(updated.status).toBe('active');
  });

  // ════════════════════════════════════════════════════════════════
  // PHASE 8: HEALTH + BOT STATE
  // ════════════════════════════════════════════════════════════════

  test('P8-HEALTH: health check returns 200 with Telegram degraded', async ({ request }) => {
    const res = await request.get(`${BASE}/health`);
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.status).toBeDefined();
    expect(body.checks.postgres.status).toBe('ok');
    expect(['ok', 'degraded']).toContain(body.checks.telegram.status);
  });

  test('P8-MENU-PAGE: public menu page loads for test location', async ({ request }) => {
    const res = await request.get(`${BASE}/s/${TEST_SLUG}`);
    expect(res.status()).toBe(200);
    const html = await res.text();
    expect(html).toContain('lang="sq"');
  });

  // ════════════════════════════════════════════════════════════════
  // PHASE 9: CLEANUP — Delete test product
  // ════════════════════════════════════════════════════════════════

  test('P9-CLEANUP: cleanup test resources', async ({ request }) => {
    const res = await request.delete(
      `${BASE}/api/owner/locations/${locationId}/products/${productId}`,
      { headers: await authHeaders() },
    );
    expect(res.status()).toBe(204);
  });
});
