import test from 'node:test';
import assert from 'node:assert/strict';
import { TelegramAdapter } from '../../src/notifications/adapters/telegram.js';
import { RetryPolicy } from '../../src/notifications/retry.js';

test('Telegram Adapter & Retry Policy', async (t) => {

  await t.test('Retry Policy calculates exponential backoff', () => {
    const policy = new RetryPolicy({ maxAttempts: 5, baseMs: 1000, maxMs: 10000, jitter: 0 });
    assert.equal(policy.getDelay(0), 1000);
    assert.equal(policy.getDelay(1), 2000);
    assert.equal(policy.getDelay(2), 4000);
    assert.equal(policy.getDelay(3), 8000);
    assert.equal(policy.getDelay(4), 10000); // capped
    assert.equal(policy.getDelay(5), -1); // max reached
  });

  // Finding #2: exhaustion path — getDelay() returns a positive backoff while retries remain
  // and flips to -1 EXACTLY at maxAttempts (the dispatcher's "stop retrying" signal).
  // (RetryPolicy exposes getDelay() only; there is no shouldRetry() to call.)
  await t.test('Retry Policy exhausts exactly at maxAttempts', () => {
    const policy = new RetryPolicy({ maxAttempts: 3, baseMs: 100, maxMs: 1000, jitter: 0 });
    assert.equal(policy.getDelay(0), 100); // retry permitted (positive backoff)
    assert.equal(policy.getDelay(1), 200);
    assert.equal(policy.getDelay(2), 400);
    assert.equal(policy.getDelay(3), -1); // exhausted at maxAttempts
    assert.equal(policy.getDelay(4), -1); // stays exhausted beyond
  });

  await t.test('Telegram Adapter fails without token', async () => {
    const adapter = new TelegramAdapter('');
    const result = await adapter.notify(
      { id: '1', channel: 'telegram', address: '123', locationId: 'loc1' },
      { type: 'test' },
      { locationId: 'loc1' }
    );
    assert.equal(result.delivered, false);
    assert.equal(result.reason, 'TELEGRAM_TOKEN_NOT_CONFIGURED');
  });
  
  // Finding #1: success path — stub fetch (200, {ok:true}) and assert the delivered:true
  // branch + message_id parsing actually run (the real HTTP call/response parsing path).
  await t.test('Telegram Adapter delivers on 200 and parses providerMessageId', async () => {
    const original = globalThis.fetch;
    const calls: any[] = [];
    globalThis.fetch = (async (url: any, init: any) => {
      calls.push({ url, init });
      return {
        ok: true,
        status: 200,
        headers: { get: () => null },
        json: async () => ({ ok: true, result: { message_id: 4242 } }),
      };
    }) as any;
    try {
      const adapter = new TelegramAdapter('TESTTOKEN');
      const result = await adapter.notify(
        { id: '1', channel: 'telegram', address: '555', locationId: 'loc1' },
        { type: 'test' },
        { locationId: 'loc1' }
      );
      assert.equal(result.delivered, true);
      assert.equal(result.providerMessageId, '4242');
      assert.equal(calls.length, 1);
      assert.match(String(calls[0].url), /TESTTOKEN\/sendMessage$/);
      assert.equal(JSON.parse(calls[0].init.body).chat_id, '555');
    } finally {
      globalThis.fetch = original;
    }
  });

  // Finding #3a: 429 → RATE_LIMIT with a positive retryAfter backoff the dispatcher feeds to RetryPolicy.
  await t.test('Telegram Adapter returns RATE_LIMIT with retryAfter on 429', async () => {
    const original = globalThis.fetch;
    globalThis.fetch = (async () => ({
      ok: false,
      status: 429,
      headers: { get: (k: string) => (k.toLowerCase() === 'retry-after' ? '3' : null) },
      json: async () => ({}),
    })) as any;
    try {
      const adapter = new TelegramAdapter('TESTTOKEN');
      const result = await adapter.notify(
        { id: '1', channel: 'telegram', address: '555', locationId: 'loc1' },
        { type: 'test' },
        { locationId: 'loc1' }
      );
      assert.equal(result.delivered, false);
      assert.equal(result.reason, 'RATE_LIMIT');
      assert.equal((result as any).retryAfter, 3000); // 3s header → ms, a real positive backoff
    } finally {
      globalThis.fetch = original;
    }
  });

  // Finding #3b: 500 → HTTP_500 (server error surfaced verbatim, not swallowed as delivered).
  await t.test('Telegram Adapter surfaces HTTP_500 on server error', async () => {
    const original = globalThis.fetch;
    globalThis.fetch = (async () => ({
      ok: false,
      status: 500,
      headers: { get: () => null },
      json: async () => ({}),
    })) as any;
    try {
      const adapter = new TelegramAdapter('TESTTOKEN');
      const result = await adapter.notify(
        { id: '1', channel: 'telegram', address: '555', locationId: 'loc1' },
        { type: 'test' },
        { locationId: 'loc1' }
      );
      assert.equal(result.delivered, false);
      assert.equal(result.reason, 'HTTP_500');
    } finally {
      globalThis.fetch = original;
    }
  });

  // Finding #3c: network throw → caught and reported as a non-delivery (not an unhandled rejection).
  await t.test('Telegram Adapter reports network failure as non-delivery', async () => {
    const original = globalThis.fetch;
    globalThis.fetch = (async () => {
      throw new Error('ECONNRESET');
    }) as any;
    try {
      const adapter = new TelegramAdapter('TESTTOKEN');
      const result = await adapter.notify(
        { id: '1', channel: 'telegram', address: '555', locationId: 'loc1' },
        { type: 'test' },
        { locationId: 'loc1' }
      );
      assert.equal(result.delivered, false);
      assert.equal(result.reason, 'ECONNRESET');
    } finally {
      globalThis.fetch = original;
    }
  });

  // Finding #4: cross-tenant isolation — each notify must route ONLY to its own target's chat_id.
  // Capture outgoing fetch bodies for two distinct tenants and assert no chat_id bleed.
  await t.test('Telegram Adapter routes each tenant to its own chat_id (no cross-tenant leak)', async () => {
    const original = globalThis.fetch;
    const seenChatIds: string[] = [];
    globalThis.fetch = (async (_url: any, init: any) => {
      seenChatIds.push(JSON.parse(init.body).chat_id);
      return {
        ok: true,
        status: 200,
        headers: { get: () => null },
        json: async () => ({ ok: true, result: { message_id: 1 } }),
      };
    }) as any;
    try {
      const adapter = new TelegramAdapter('TESTTOKEN');
      await adapter.notify(
        { id: 'a', channel: 'telegram', address: 'tenantA-chat', locationId: 'locA' },
        { type: 'test' },
        { locationId: 'locA' }
      );
      await adapter.notify(
        { id: 'b', channel: 'telegram', address: 'tenantB-chat', locationId: 'locB' },
        { type: 'test' },
        { locationId: 'locB' }
      );
      assert.deepEqual(seenChatIds, ['tenantA-chat', 'tenantB-chat']);
      assert.notEqual(seenChatIds[0], seenChatIds[1]); // tenant B never received tenant A's chat_id
    } finally {
      globalThis.fetch = original;
    }
  });
});
