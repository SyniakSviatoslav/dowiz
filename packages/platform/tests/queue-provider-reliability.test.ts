import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createQueueWithDefaults, deadLetterQueueName } from '../src/queue-provider.js';

// H1/H2 (2026-07-03 reliability audit): pg-boss v10 createQueue() with no options
// left every queue on runtime defaults — retryLimit=2, retryDelay=0s, no backoff,
// no deadLetter — so a transient failure was hammered twice within ms then vanished
// into `failed` with no salvage path. This proves createQueueWithDefaults() always
// supplies non-zero retry/backoff config, and that requesting a `deadLetter` creates
// the DLQ queue FIRST (pg-boss v10 has a self-referencing FK: queue.deadLetter must
// name an existing queue) before pointing the main queue at it.

function fakeBoss() {
  const calls: Array<{ name: string; options: any }> = [];
  const existing = new Set<string>();
  return {
    calls,
    existing,
    async createQueue(name: string, options: any = {}) {
      // Mirrors pg-boss v10's real constraint: a deadLetter target must already exist.
      if (options.deadLetter && !existing.has(options.deadLetter)) {
        throw new Error(`deadLetter queue "${options.deadLetter}" does not exist`);
      }
      calls.push({ name, options });
      existing.add(name);
    },
  };
}

test('#H1/H2 default retry/backoff config is always applied, never zero/absent', async () => {
  const boss = fakeBoss();
  await createQueueWithDefaults(boss, 'backup.hourly');
  assert.equal(boss.calls.length, 1);
  const [call] = boss.calls;
  assert.ok(call.options.retryLimit > 0, 'retryLimit must be non-zero (v10 default was only 2 with 0s delay)');
  assert.ok(call.options.retryDelay > 0, 'retryDelay must be non-zero (v10 default is 0s — instant hammer)');
  assert.equal(call.options.retryBackoff, true);
});

test('#H1/H2 deadLetter:true creates the DLQ queue before the main queue references it', async () => {
  const boss = fakeBoss();
  await createQueueWithDefaults(boss, 'backup.hourly', { deadLetter: true });
  assert.equal(boss.calls.length, 2, 'must create the DLQ queue and the main queue');
  assert.equal(boss.calls[0].name, deadLetterQueueName('backup.hourly'));
  assert.equal(boss.calls[1].name, 'backup.hourly');
  assert.equal(boss.calls[1].options.deadLetter, 'backup.hourly.dlq');
  // The DLQ itself is a terminal sink — it should not carry a retry policy of its own.
  assert.equal(boss.calls[0].options.retryLimit, undefined);
});

test('#H1/H2 policy override (short) is passed through for singletonKey-only dedup', async () => {
  const boss = fakeBoss();
  await createQueueWithDefaults(boss, 'backup.daily', { policy: 'short' });
  assert.equal(boss.calls[0].options.policy, 'short');
});

test('#H1/H2 caller overrides win over defaults', async () => {
  const boss = fakeBoss();
  await createQueueWithDefaults(boss, 'backup.weekly', { retryLimit: 7, retryDelay: 99 });
  assert.equal(boss.calls[0].options.retryLimit, 7);
  assert.equal(boss.calls[0].options.retryDelay, 99);
});
