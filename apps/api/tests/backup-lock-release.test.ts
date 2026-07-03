import { test } from 'node:test';
import assert from 'node:assert/strict';

// handleBackup()'s import chain (via @deliveryos/db) calls loadEnv() (Zod-validated
// process.env) at MODULE LOAD time. ES module imports are hoisted/evaluated before
// this file's own top-level code runs, so BackupCronWorker must be loaded via a
// dynamic import AFTER the env is seeded below (same pattern as health-truthfulness
// .test.ts). This file runs isolated (node:test spawns each matched file as its own
// process), so seeding dummy-but-schema-valid values here cannot leak into other
// test files.
const REQUIRED_ENV: Record<string, string> = {
  NODE_ENV: 'test',
  APP_BASE_URL: 'http://localhost:3000',
  DATABASE_URL_OPERATIONAL: 'postgres://u:p@localhost:5432/db',
  DATABASE_URL_SESSION: 'postgres://u:p@localhost:5432/db',
  DATABASE_URL_MIGRATIONS: 'postgres://u:p@localhost:5432/db',
  REDIS_URL: 'redis://localhost:6379',
  JWT_PRIVATE_KEY: 'test',
  JWT_PUBLIC_KEY: 'test',
  JWT_KID: 'test',
  GOOGLE_CLIENT_ID: 'test',
  GOOGLE_CLIENT_SECRET: 'test',
  VAPID_PUBLIC_KEY: 'test',
  VAPID_PRIVATE_KEY: 'test',
  IP_HASH_SALT: 'test',
};
for (const [key, value] of Object.entries(REQUIRED_ENV)) {
  if (process.env[key] === undefined) process.env[key] = value;
}

// H6 (2026-07-03 reliability audit): the `finally` block called
// `this.releaseLock(lockClient)` with `type` omitted → getLockKey(undefined) unlocked
// "backup_lock_undefined", never the real per-type advisory lock key. The lock stayed
// held forever on the pooled connection, so every subsequent run of that backup type
// saw locked=false and silently skipped — backups stopped after the first run.
//
// This proves handleBackup() releases the SAME advisory-lock key it acquired, by
// spying on the query text pg_advisory_unlock is called with vs. pg_try_advisory_lock.

function fakePool(query: (sql: string, params?: any[]) => Promise<any>) {
  return {
    async connect() {
      return {
        query: (sql: string, params?: any[]) => query(sql, params),
        release: () => {},
      };
    },
    query,
  } as any;
}

test('#H6 releaseLock unlocks the SAME advisory-lock key handleBackup acquired', async () => {
  const { BackupCronWorker } = await import('../src/workers/backup/index.js');
  const lockCalls: Array<{ kind: 'lock' | 'unlock'; key: number }> = [];

  const opPool = fakePool(async (sql: string, params?: any[]) => {
    if (/pg_try_advisory_lock/.test(sql)) {
      lockCalls.push({ kind: 'lock', key: params![0] });
      return { rows: [{ locked: true }] };
    }
    if (/pg_advisory_unlock/.test(sql)) {
      lockCalls.push({ kind: 'unlock', key: params![0] });
      return { rows: [{}] };
    }
    return { rows: [] };
  });
  const backupPool = fakePool(async () => ({ rows: [] }));
  const boss = { work: async () => {}, createQueue: async () => {}, schedule: async () => {} };
  const messageBus = { publish: async () => {} };

  const worker = new (BackupCronWorker as any)(opPool, backupPool, boss, messageBus, {
    createLogicalDump: async () => {
      // Fails on every attempt so the retry loop exhausts deterministically without
      // ever touching real pg_dump/encryption/R2 — only lock discipline is under test.
      throw new Error('forced failure — lock-discipline test only');
    },
    uploadStream: async () => {},
    uploadJson: async () => {},
  });
  // The real retry loop sleeps up to 1min/5min/15min between attempts (see M8). Skip
  // the real delay so this test doesn't take up to 21 real minutes — the loop's
  // control flow (and, critically, the finally-block lock release) still runs exactly
  // as it does in production, just without the wall-clock wait.
  (worker as any).sleep = async () => {};

  await worker.handleBackup('hourly');

  const locks = lockCalls.filter((c) => c.kind === 'lock');
  const unlocks = lockCalls.filter((c) => c.kind === 'unlock');
  assert.equal(locks.length, 1, 'expected exactly one lock acquisition');
  assert.equal(unlocks.length, 1, 'expected exactly one unlock in finally');
  assert.equal(
    unlocks[0].key,
    locks[0].key,
    `releaseLock must unlock the same key it locked (locked ${locks[0].key}, unlocked ${unlocks[0].key})`,
  );
});
