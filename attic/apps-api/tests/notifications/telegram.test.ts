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
  
  // Note: Since we are not mocking fetch globally here, we won't do full integration tests for fetch in unit tests.
  // We verified the rendering and logic.
});
