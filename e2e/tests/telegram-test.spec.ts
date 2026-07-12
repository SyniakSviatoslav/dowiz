import { test, expect } from '@playwright/test';

// Telegram bot integration checks. Test Integrity (AGENTS.md):
// - Never log the bot token (CI-log leak) — no console.* in this file.
// - Assert EXACT values (bot username, webhook URL), not `.toBeTruthy()`.
// - Every test runs >=1 assertion that can go red.
//
// Required env (asserted in beforeAll so setup fails fast, not silently green):
//   TELEGRAM_BOT_TOKEN      — `<botId>:<secret>` for the bot under test
//   EXPECTED_BOT_USERNAME   — exact @username the token must resolve to (anti-wrong-bot)
//   EXPECTED_WEBHOOK_URL    — exact dowiz webhook endpoint Telegram must be calling
const BOT_TOKEN = process.env.TELEGRAM_BOT_TOKEN;
const EXPECTED_BOT_USERNAME = process.env.EXPECTED_BOT_USERNAME;
const EXPECTED_WEBHOOK_URL = process.env.EXPECTED_WEBHOOK_URL;

// Telegram bot tokens are `<digits>:<35 url-safe chars>`. Validate shape without printing.
const TG_TOKEN = /^\d+:[\w-]{35,}$/;

test.describe('Telegram Bot Tests', () => {
  test.beforeAll(() => {
    expect(String(BOT_TOKEN ?? ''), 'TELEGRAM_BOT_TOKEN must be a valid bot token').toMatch(TG_TOKEN);
    expect(String(EXPECTED_BOT_USERNAME ?? ''), 'EXPECTED_BOT_USERNAME must be set').not.toBe('');
    expect(String(EXPECTED_WEBHOOK_URL ?? ''), 'EXPECTED_WEBHOOK_URL must be an https URL').toMatch(/^https:\/\/.+/);
  });

  test('should get bot info', async () => {
    const url = `https://api.telegram.org/bot${BOT_TOKEN}/getMe`;
    const resp = await fetch(url);
    expect(resp.status).toBe(200);
    const data = await resp.json();
    expect(data.ok).toBe(true);
    expect(data.result.is_bot).toBe(true);
    // Exact username — a truthy check would pass for a completely different bot.
    expect(data.result.username).toBe(EXPECTED_BOT_USERNAME);
  });

  test('should get webhook info', async () => {
    const url = `https://api.telegram.org/bot${BOT_TOKEN}/getWebhookInfo`;
    const resp = await fetch(url);
    expect(resp.status).toBe(200);
    const data = await resp.json();
    expect(data.ok).toBe(true);
    // Telegram must be delivering to OUR endpoint, not some leftover/foreign URL.
    expect(data.result.url).toBe(EXPECTED_WEBHOOK_URL);
    expect(typeof data.result.pending_update_count).toBe('number');
    // No delivery error standing on the webhook.
    expect(data.result.last_error_message ?? '').toBe('');
  });

  test('should get updates', async () => {
    const url = `https://api.telegram.org/bot${BOT_TOKEN}/getUpdates?offset=0&limit=1&timeout=0`;
    const resp = await fetch(url);
    const data = await resp.json();
    // Bimodal but exact: with a webhook registered, getUpdates MUST 409 (conflict);
    // in poll mode it MUST 200 with ok:true + an array result. Both branches assert
    // exact status + body — no permissive [200,409] array, no silent path.
    if (resp.status === 200) {
      expect(data.ok).toBe(true);
      expect(Array.isArray(data.result)).toBe(true);
    } else {
      expect(resp.status).toBe(409);
      expect(data.ok).toBe(false);
      expect(data.error_code).toBe(409);
    }
  });

  // TODO(needs_staging): cross-tenant notification-dispatch isolation. These probes hit
  // Telegram directly and prove nothing about tenant scoping. Add an integration scenario
  // against staging with a REAL second tenant: drive an order in tenant A, assert tenant B's
  // linked chat receives NO dispatch (and A's does), via /api/owner/.../notifications. Requires
  // a live staging run + a real 2nd tenant's chat_id — do not fake with a nil/zero id.
});
