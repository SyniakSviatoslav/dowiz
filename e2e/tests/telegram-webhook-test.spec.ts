import { test, expect } from '@playwright/test';
import { requireStaging } from '../helpers/staging-guard';

// Live Telegram tests MUST use a DEDICATED test bot — never the production bot.
// deleteWebhook/setWebhook mutate the bot's webhook registration globally, so
// running them against the prod TELEGRAM_BOT_TOKEN would break real notifications.
// Gate on TEST_TELEGRAM_BOT_TOKEN; skip when the dedicated test bot is absent.
const TEST_BOT_TOKEN = process.env.TEST_TELEGRAM_BOT_TOKEN;
const BASE_URL = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
// App webhook path secret — the registered route is /webhook/telegram/<secret>.
const BOT_SECRET = process.env.TELEGRAM_BOT_SECRET;

test.describe('Telegram Webhook Management', () => {
  // Mutating spec (sets the bot webhook to point at BASE_URL) → fail fast off non-staging.
  test.beforeAll(() => {
    requireStaging(BASE_URL);
  });

  test('should be able to delete webhook', async () => {
    test.skip(!TEST_BOT_TOKEN, 'requires TEST_TELEGRAM_BOT_TOKEN (dedicated test bot)');
    const deleteUrl = `https://api.telegram.org/bot${TEST_BOT_TOKEN}/deleteWebhook`;
    const deleteResp = await fetch(deleteUrl, { method: 'POST' });
    expect(deleteResp.ok).toBe(true);
    const deleteData = await deleteResp.json();
    expect(deleteData.ok).toBe(true);
  });

  test('should be able to get updates after deleting webhook', async () => {
    test.skip(!TEST_BOT_TOKEN, 'requires TEST_TELEGRAM_BOT_TOKEN (dedicated test bot)');

    // Delete webhook first — getUpdates and a webhook are mutually exclusive.
    const deleteResp = await fetch(
      `https://api.telegram.org/bot${TEST_BOT_TOKEN}/deleteWebhook`,
      { method: 'POST' },
    );
    expect(deleteResp.ok).toBe(true);

    const url = `https://api.telegram.org/bot${TEST_BOT_TOKEN}/getUpdates?offset=0&limit=1&timeout=0`;
    const resp = await fetch(url);
    // Unconditional assertion — a missing/invalid token returns HTTP 200 with
    // {ok:false}, so assert the parsed payload, not just resp.ok.
    expect(resp.ok).toBe(true);
    const data = await resp.json();
    expect(data.ok).toBe(true);
    expect(Array.isArray(data.result)).toBe(true);
  });

  test('should be able to set webhook again', async () => {
    test.skip(!TEST_BOT_TOKEN, 'requires TEST_TELEGRAM_BOT_TOKEN (dedicated test bot)');
    test.skip(!BOT_SECRET, 'requires TELEGRAM_BOT_SECRET (webhook path secret)');

    const setUrl = `https://api.telegram.org/bot${TEST_BOT_TOKEN}/setWebhook`;
    const webhookUrl = `${BASE_URL}/webhook/telegram/${BOT_SECRET}`;
    const setResp = await fetch(setUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url: webhookUrl, secret_token: BOT_SECRET, allowed_updates: ['message', 'callback_query'] }),
    });
    expect(setResp.ok).toBe(true);
    const setData = await setResp.json();
    expect(setData.ok).toBe(true);

    // setWebhook returns ok:true even when the URL is unreachable — confirm the
    // registration actually points at THIS environment via getWebhookInfo.
    const infoResp = await fetch(`https://api.telegram.org/bot${TEST_BOT_TOKEN}/getWebhookInfo`);
    expect(infoResp.ok).toBe(true);
    const infoData = await infoResp.json();
    expect(infoData.ok).toBe(true);
    expect(infoData.result.url).toBe(webhookUrl);
  });

  // Error matrix — the app server must reject forged/missing webhook secrets.
  test('app server rejects webhook with a wrong path secret', async () => {
    const resp = await fetch(`${BASE_URL}/webhook/telegram/wrong-secret-not-registered`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ update_id: 1 }),
    });
    // Only the secret-embedded path is registered (telegram-webhook.ts:36); a wrong
    // path secret matches no route → Fastify 404, never reaching the handler/DB.
    expect(resp.status).toBe(404);
  });

  test('app server rejects webhook with a forged secret-token header', async () => {
    test.skip(!BOT_SECRET, 'requires TELEGRAM_BOT_SECRET to reach the registered path');
    const resp = await fetch(`${BASE_URL}/webhook/telegram/${BOT_SECRET}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-telegram-bot-api-secret-token': 'forged-wrong-token',
      },
      body: JSON.stringify({ update_id: 1 }),
    });
    // telegram-webhook.ts:50-56 → sendError(401) on header mismatch, before any
    // update is parsed or processed — a non-mutating negative control.
    expect(resp.status).toBe(401);
  });
});
