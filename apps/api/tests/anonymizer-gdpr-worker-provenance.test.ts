import { test } from 'node:test';
import assert from 'node:assert/strict';
import { GdprErasureWorker } from '../src/workers/anonymizer-gdpr.js';

// R2-5 (breaker-findings-r2.md): there are TWO audit-log inserts per GDPR erasure — the
// anonymizer's own (lib/anonymizer/index.ts) and this worker's SECOND insert (:74-78 pre-fix,
// now the block right after the `UPDATE ... SET status='completed'`). The worker's insert used
// to stamp `row.location_id` — the erasure REQUEST's (actor's) tenant — for BOTH the location_id
// column and (implicitly) as the only tenant reference in metadata. If a request's location_id
// ever diverges from the subject customer's true location_id (the exact drift the STOP-1 forensic
// queries are designed to catch), the two audit rows for one erasure would disagree on tenant.
// The fix reads the customer's true location_id back from the DB and stamps THAT — independent
// of what the request row claims — plus actor_location_id/subject_location_id/request_id
// provenance, matching the field names the lib's own insert uses.

const REQUEST_ID = 'req-1';
const CUSTOMER_ID = 'cust-1';
const REQUEST_LOCATION = 'loc-actor'; // the erasure request's (actor's) tenant
const CUSTOMER_TRUE_LOCATION = 'loc-subject-true'; // the customer's ACTUAL tenant (may drift)

function makeHarness(customerTrueLocation: string) {
  const inserted: { params: any[] }[] = [];
  const client = {
    query: async (sql: string, params: any[] = []) => {
      if (/^\s*(BEGIN|COMMIT|ROLLBACK)/i.test(sql)) return { rowCount: 0, rows: [] };
      if (/SELECT\s+id,\s*location_id,\s*customer_id/i.test(sql)) {
        return { rowCount: 1, rows: [{ id: REQUEST_ID, location_id: REQUEST_LOCATION, customer_id: CUSTOMER_ID, subject_phone: null, metadata: {} }] };
      }
      if (/UPDATE\s+gdpr_erasure_requests\s+SET\s+status\s*=\s*'in_progress'/i.test(sql)) {
        return { rowCount: 1, rows: [] };
      }
      if (/UPDATE\s+gdpr_erasure_requests[\s\S]*status\s*=\s*'completed'/i.test(sql)) {
        return { rowCount: 1, rows: [] };
      }
      // N1 / REV-S9-3 fail-loud backstop re-read (resolution-r2.md §1 N1.2; subject-graph
      // extension resolution.md REV-S9-3): the worker confirms the erasure took effect before
      // writing `completed`. Model the confirmed (erased) state — no orders/ratings in this
      // harness — so the provenance assertions below exercise the completion path.
      if (/customer_anonymized_at/i.test(sql)) {
        return { rowCount: 1, rows: [{ customer_anonymized_at: '2026-07-03T00:00:00Z', orders_remaining: 0, ratings_remaining: 0 }] };
      }
      // The NEW true-tenant lookup (R2-5 fix).
      if (/SELECT\s+location_id\s+FROM\s+customers\s+WHERE\s+id\s*=\s*\$1/i.test(sql)) {
        return { rowCount: 1, rows: [{ location_id: customerTrueLocation }] };
      }
      if (/INSERT\s+INTO\s+anonymization_audit_log/i.test(sql)) {
        inserted.push({ params });
        return { rowCount: 1, rows: [] };
      }
      return { rowCount: 0, rows: [] };
    },
    release() {},
  };
  const pool = { connect: async () => client };
  const boss = { work: async () => {}, send: async () => {} };
  const messageBus = { publish: async () => {} };
  const anonymizerService = { anonymize: async () => ({ customersAnonymized: 1, ordersAnonymized: 0, storagePurged: 0, r2Marked: 0, skipped: 0, durationMs: 1, dryRun: false }) };
  const worker = new GdprErasureWorker(pool as any, boss as any, messageBus as any, anonymizerService as any);
  return { worker, inserted };
}

test('R2-5: worker audit insert stamps the CUSTOMER\'S TRUE location_id, not the request row\'s, when they diverge', async () => {
  const { worker, inserted } = makeHarness(CUSTOMER_TRUE_LOCATION);
  await (worker as any).run();

  assert.equal(inserted.length, 1, 'exactly one audit row is inserted by the worker');
  const [scope, subjectKind, subjectId, locationIdCol, actorKind, actorId, metadataJson] = inserted[0].params;
  assert.equal(scope, 'gdpr');
  assert.equal(subjectKind, 'customer');
  assert.equal(subjectId, CUSTOMER_ID);
  assert.equal(locationIdCol, CUSTOMER_TRUE_LOCATION,
    'the location_id COLUMN must be the subject\'s true tenant, not the request\'s');
  assert.equal(actorKind, 'system');
  assert.equal(actorId, null);

  const metadata = JSON.parse(metadataJson);
  assert.equal(metadata.subject_location_id, CUSTOMER_TRUE_LOCATION);
  assert.equal(metadata.actor_location_id, REQUEST_LOCATION);
  assert.equal(metadata.request_id, REQUEST_ID);
});

test('R2-5: worker audit insert matches request tenant in the (expected) non-divergent case (no regression)', async () => {
  const { worker, inserted } = makeHarness(REQUEST_LOCATION);
  await (worker as any).run();

  const [, , , locationIdCol] = inserted[0].params;
  assert.equal(locationIdCol, REQUEST_LOCATION);
  const metadata = JSON.parse(inserted[0].params[6]);
  assert.equal(metadata.subject_location_id, REQUEST_LOCATION);
  assert.equal(metadata.actor_location_id, REQUEST_LOCATION);
});
