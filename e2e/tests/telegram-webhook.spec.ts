import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
const BOT_SECRET = 'Ihatenuclearwar';
const WEBHOOK_URL = `${BASE}/webhook/telegram/${BOT_SECRET}`;
const BOT_TOKEN = '8996764379:AAHkuc5mgYQdkWG5rLZEjHc8a8k5MQsHDIk';

function uuid() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => {
    const r = Math.random() * 16 | 0;
    return (c === 'x' ? r : (r & 0x3 | 0x8)).toString(16);
  });
}

test.describe('Telegram Webhook — Live https://dowiz.fly.dev', () => {

  test('HEALTH-1: health check returns 200 with Telegram degraded', async ({ request }) => {
    const resp = await request.get(`${BASE}/health`);
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.status).toBeDefined();
    expect(body.checks.postgres.status).toBe('ok');
    // Telegram is non-critical, so even with an invalid token it should be degraded not down
    expect(['ok', 'degraded']).toContain(body.checks.telegram.status);
  });

  test('WEBHOOK-1: correct secret returns 200', async ({ request }) => {
    const resp = await request.post(WEBHOOK_URL, {
      headers: {
        'Content-Type': 'application/json',
        'x-telegram-bot-api-secret-token': BOT_SECRET,
      },
      data: { message: { text: '/stop', chat: { id: 999001 }, from: { id: 999001 } } },
    });
    expect(resp.status()).toBe(200);
  });

  test('WEBHOOK-2: missing secret returns 401', async ({ request }) => {
    const resp = await request.post(WEBHOOK_URL, {
      headers: { 'Content-Type': 'application/json' },
      data: { message: { text: '/stop', chat: { id: 999001 }, from: { id: 999001 } } },
    });
    expect(resp.status()).toBe(401);
  });

  test('WEBHOOK-3: wrong secret returns 401', async ({ request }) => {
    const resp = await request.post(WEBHOOK_URL, {
      headers: {
        'Content-Type': 'application/json',
        'x-telegram-bot-api-secret-token': 'wrong-secret-here',
      },
      data: { message: { text: '/stop', chat: { id: 999001 }, from: { id: 999001 } } },
    });
    expect(resp.status()).toBe(401);
  });

  test('WEBHOOK-4: malformed body returns 200 (best-effort)', async ({ request }) => {
    const resp = await request.post(WEBHOOK_URL, {
      headers: {
        'Content-Type': 'application/json',
        'x-telegram-bot-api-secret-token': BOT_SECRET,
      },
      data: { invalid: 'body' },
    });
    expect(resp.status()).toBe(200);
  });

  test('WEBHOOK-5: empty body handled gracefully', async ({ request }) => {
    const resp = await request.post(WEBHOOK_URL, {
      headers: {
        'Content-Type': 'application/json',
        'x-telegram-bot-api-secret-token': BOT_SECRET,
      },
      data: '',
    });
    expect([200, 400]).toContain(resp.status());
  });

  test('WEBHOOK-6: /start with invalid token returns 200 (best-effort)', async ({ request }) => {
    const resp = await request.post(WEBHOOK_URL, {
      headers: {
        'Content-Type': 'application/json',
        'x-telegram-bot-api-secret-token': BOT_SECRET,
      },
      data: {
        message: {
          text: `/start 00000000-0000-0000-0000-000000000000`,
          chat: { id: 999002, type: 'private' },
          from: { id: 999002, first_name: 'Test' },
        },
      },
    });
    expect(resp.status()).toBe(200);
  });

  test('WEBHOOK-7: callback with unknown action returns 200 (best-effort)', async ({ request }) => {
    const resp = await request.post(WEBHOOK_URL, {
      headers: {
        'Content-Type': 'application/json',
        'x-telegram-bot-api-secret-token': BOT_SECRET,
      },
      data: {
        callback_query: {
          id: 'cb_unknown',
          data: `unknown_action:${uuid()}`,
          from: { id: 999001, first_name: 'Test' },
          message: { chat: { id: 999001 }, message_id: 100, text: '...' },
        },
      },
    });
    expect(resp.status()).toBe(200);
  });

  test('WEBHOOK-8: unlinked user callback returns 200 (best-effort)', async ({ request }) => {
    const resp = await request.post(WEBHOOK_URL, {
      headers: {
        'Content-Type': 'application/json',
        'x-telegram-bot-api-secret-token': BOT_SECRET,
      },
      data: {
        callback_query: {
          id: 'cb_unlinked',
          data: `order.confirm:${uuid()}`,
          from: { id: 555555, first_name: 'Unlinked' },
          message: { chat: { id: 555555 }, message_id: 101, text: '...' },
        },
      },
    });
    expect(resp.status()).toBe(200);
  });

  test('WEBHOOK-9: Bot API getMe returns valid bot info', async ({ request }) => {
    const resp = await request.get(`https://api.telegram.org/bot${BOT_TOKEN}/getMe`);
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.ok).toBe(true);
    expect(body.result.username).toBe('dowizbot_bot');
    expect(body.result.is_bot).toBe(true);
  });

  test('WEBHOOK-10: Bot webhook is configured', async ({ request }) => {
    const resp = await request.get(`https://api.telegram.org/bot${BOT_TOKEN}/getWebhookInfo`);
    expect(resp.status()).toBe(200);
    const body = await resp.json();
    expect(body.ok).toBe(true);
    expect(body.result.url).toBe(WEBHOOK_URL);
    expect(body.result.pending_update_count).toBe(0);
  });

  test('NO-COOKIE-1: webhook endpoint sets no cookies', async ({ request }) => {
    const resp = await request.post(WEBHOOK_URL, {
      headers: {
        'Content-Type': 'application/json',
        'x-telegram-bot-api-secret-token': BOT_SECRET,
      },
      data: { message: { text: '/stop', chat: { id: 999001 }, from: { id: 999001 } } },
    });
    const cookies = resp.headers()['set-cookie'];
    expect(cookies).toBeUndefined();
  });
});
