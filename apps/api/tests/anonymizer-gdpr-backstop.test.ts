import { test } from 'node:test';
import assert from 'node:assert/strict';
import { GdprErasureWorker } from '../src/workers/anonymizer-gdpr.js';

// Guardrail for two legal-red-line (GDPR Art.17) defects the RESOLVE-R2 round
// (docs/design/audit-fix-rls-reliability/resolution-r2.md §2) prescribed as required Lane-A rework:
//
//  * LC4 (reliability C4) — a transient failure flipped the request to `in_progress`; the retry
//    scan reads only `status='pending'`, so the row is NEVER re-selected → the erasure is silently
//    stranded and never reaches `completed` or `failed`. Fix: retryable failure resets to `pending`.
//
//  * N1 (rls-reliability breaker-r2, CRITICAL) — the worker wrote `status='completed'` + audit +
//    event UNCONDITIONALLY, regardless of whether the anonymizer actually erased anything. Under
//    NOBYPASSRLS+MIG-2 a context-free erasure silently no-ops, producing a false Art.17 completion.
//    Fix (fail-loud backstop): the terminal `completed`/audit/event fire ONLY when a data-level
//    re-read confirms `customers.anonymized_at IS NOT NULL`; otherwise `failed` + FAILED signal,
//    NEVER `completed`.
//
// These are worker-local (anonymizer-gdpr.ts) — no migration dependency. The structural post-flip
// *success* fix (DEFINER gdpr_erase_customer) rides LC4-MIG and is DEFER-FLAG/GATE-FLIP-E2E; this
// backstop is what makes the interim safe (loud failure, never a silent false completion).

const REQUEST_ID = 'req-1';
const CUSTOMER_ID = 'cust-1';
const LOCATION = 'loc-1';

interface HarnessOpts {
  /** What anonymizerService.anonymize resolves to, or an Error to throw. */
  anon: any | Error;
  /** Value returned by the N1 data-level re-read `SELECT anonymized_at FROM customers`. */
  reReadAnonymizedAt: string | null;
  /** metadata on the pending request row (drives the retry counter in the catch path). */
  rowMetadata?: any;
}

function makeHarness(opts: HarnessOpts) {
  const statusWrites: string[] = [];
  const audits: any[][] = [];
  const publishes: { channel: string; payload: any }[] = [];
  const bossSends: { queue: string; payload: any }[] = [];

  const client = {
    query: async (sql: string, params: any[] = []) => {
      if (/^\s*(BEGIN|COMMIT|ROLLBACK)/i.test(sql)) return { rowCount: 0, rows: [] };
      if (/SELECT\s+id,\s*location_id,\s*customer_id/i.test(sql)) {
        return {
          rowCount: 1,
          rows: [{ id: REQUEST_ID, location_id: LOCATION, customer_id: CUSTOMER_ID, subject_phone: null, metadata: opts.rowMetadata ?? {} }],
        };
      }
      const statusMatch = sql.match(/UPDATE\s+gdpr_erasure_requests\s+SET\s+status\s*=\s*'(\w+)'/i);
      if (statusMatch) {
        statusWrites.push(statusMatch[1]);
        return { rowCount: 1, rows: [] };
      }
      // N1 data-level backstop re-read.
      if (/SELECT\s+anonymized_at\s+FROM\s+customers\s+WHERE\s+id\s*=\s*\$1/i.test(sql)) {
        return { rowCount: opts.reReadAnonymizedAt ? 1 : 0, rows: opts.reReadAnonymizedAt ? [{ anonymized_at: opts.reReadAnonymizedAt }] : [] };
      }
      // Provenance true-tenant lookup (R2-5).
      if (/SELECT\s+location_id\s+FROM\s+customers\s+WHERE\s+id\s*=\s*\$1/i.test(sql)) {
        return { rowCount: 1, rows: [{ location_id: LOCATION }] };
      }
      if (/INSERT\s+INTO\s+anonymization_audit_log/i.test(sql)) {
        audits.push(params);
        return { rowCount: 1, rows: [] };
      }
      // metadata-only update (current retry path) — record but do not treat as a status write.
      if (/UPDATE\s+gdpr_erasure_requests\s+SET\s+metadata/i.test(sql)) {
        return { rowCount: 1, rows: [] };
      }
      return { rowCount: 0, rows: [] };
    },
    release() {},
  };

  const pool = {
    connect: async () => client,
    query: async () => ({ rowCount: 0, rows: [] }),
  };
  const boss = { work: async () => {}, send: async (queue: string, payload: any) => { bossSends.push({ queue, payload }); } };
  const messageBus = { publish: async (channel: string, payload: any) => { publishes.push({ channel, payload }); } };
  const anonymizerService = {
    anonymize: async () => {
      if (opts.anon instanceof Error) throw opts.anon;
      return opts.anon;
    },
  };
  const worker = new GdprErasureWorker(pool as any, boss as any, messageBus as any, anonymizerService as any);
  return { worker, statusWrites, audits, publishes, bossSends };
}

const ANON_ERASED = { customersAnonymized: 1, ordersAnonymized: 0, storagePurged: 0, r2Marked: 0, skipped: 0, durationMs: 1, dryRun: false };
const ANON_SKIPPED = { customersAnonymized: 0, ordersAnonymized: 0, storagePurged: 0, r2Marked: 0, skipped: 1, durationMs: 1, dryRun: false };

test('N1: erasure that produced NO effect (re-read anonymized_at IS NULL) lands `failed`, never `completed`, no audit', async () => {
  const { worker, statusWrites, audits, publishes } = makeHarness({ anon: ANON_SKIPPED, reReadAnonymizedAt: null });
  await (worker as any).run();

  assert.ok(!statusWrites.includes('completed'), 'must NOT write completed when the erasure had no effect');
  assert.ok(statusWrites.includes('failed'), 'must write failed when anonymized_at is still null (fail-loud, not silent false completion)');
  assert.equal(audits.length, 0, 'no completion audit row on a non-erasure');
  assert.ok(
    publishes.some((p) => /fail/i.test(p.channel)),
    'must emit a loud FAILED signal so a non-erasure is owned/paged, never silently completed',
  );
});

test('N1: confirmed erasure (anon=1, re-read anonymized_at NOT NULL) lands `completed` + audit + event (no over-correction)', async () => {
  const { worker, statusWrites, audits, publishes } = makeHarness({ anon: ANON_ERASED, reReadAnonymizedAt: '2026-07-03T00:00:00Z' });
  await (worker as any).run();

  assert.ok(statusWrites.includes('completed'), 'a genuinely-erased request must complete');
  assert.ok(!statusWrites.includes('failed'), 'must not fail a genuine erasure');
  assert.equal(audits.length, 1, 'exactly one completion audit row');
  assert.ok(publishes.some((p) => /erasure_completed|completed/i.test(JSON.stringify(p))), 'completion event fires');
});

test('N1: idempotent already-anonymized (anon skipped BUT re-read anonymized_at NOT NULL) still completes', async () => {
  const { worker, statusWrites } = makeHarness({ anon: ANON_SKIPPED, reReadAnonymizedAt: '2026-07-03T00:00:00Z' });
  await (worker as any).run();

  assert.ok(statusWrites.includes('completed'), 'goal-state already reached → idempotent completion is correct, not a failure');
  assert.ok(!statusWrites.includes('failed'), 'already-anonymized is a success, not a failure');
});

test('LC4: a retryable failure (retryCount<3) resets status to `pending`, so the retry scan re-selects it (not stranded in in_progress)', async () => {
  const { worker, statusWrites } = makeHarness({ anon: new Error('transient'), reReadAnonymizedAt: null, rowMetadata: { retryCount: 0 } });
  await (worker as any).run();

  assert.ok(statusWrites.includes('in_progress'), 'sanity: the row was claimed');
  assert.ok(
    statusWrites.includes('pending'),
    'retryable failure MUST reset to pending — the retry scan reads only status=pending, so leaving it in_progress strands the erasure forever (LC4)',
  );
  assert.ok(!statusWrites.includes('completed'), 'a failed attempt never completes');
});

test('LC4: exhausted retries (retryCount>=3) land terminal `failed`, not a reset loop', async () => {
  const { worker, statusWrites } = makeHarness({ anon: new Error('persistent'), reReadAnonymizedAt: null, rowMetadata: { retryCount: 3 } });
  await (worker as any).run();

  assert.ok(statusWrites.includes('failed'), 'after max retries the request must terminate as failed');
  assert.ok(!statusWrites.filter((s) => s === 'pending').length, 'no further pending reset once retries are exhausted');
});
