import { test } from 'node:test';
import assert from 'node:assert/strict';
import { AnonymizerService } from '../src/lib/anonymizer/index.js';
import { GdprErasureWorker } from '../src/workers/anonymizer-gdpr.js';

// Guardrail for the S9 council's live Art.17 under-erasure (ALL 3 seats confirmed —
// docs/design/rebuild-gdpr-s9-council/{resolution.md,breaker-findings.md,counsel-opinion.md}):
//
// A GDPR customer-erasure used to call ONLY anonymizeCustomer (anonymizer-gdpr.ts:62-65 ->
// index.ts:83-88) and NEVER fan out to the subject's orders. anonymizeOrder — which carries the
// #74 delivery_photo_key/address/receiver purge — was UNREACHABLE from Art.17, and
// orders.delivery_lat/lng (GAP-B, precise home GPS) + order_ratings.feedback (GAP-C) were nulled
// by NO path. The #61 backstop's completion gate re-read ONLY customers.anonymized_at, so the
// worker wrote `completed` + fired `gdpr.erasure_completed` while all of that survived
// (REV-S9-3). gdpr_erasure_requests.subject_phone was itself plaintext PII erased by no path
// (REV-S9-5).
//
// Fix (apps/api/src/lib/anonymizer/index.ts `anonymize()`): a GDPR customerId erasure now
// enumerates the subject's orders and runs anonymizeOrder for each (tolerated-and-reported per
// order, mirroring the avatar/photo-purge semantics — one order's failure never aborts the
// rest). anonymizeOrder now also nulls delivery_lat/delivery_lng and order_ratings.feedback in
// the SAME transaction. The worker's completion gate (anonymizer-gdpr.ts) now re-reads the WHOLE
// subject-graph (customer + orders + ratings) before writing `completed`, and nulls
// gdpr_erasure_requests.subject_phone in that same completing UPDATE.
//
// This test wires the REAL AnonymizerService + REAL GdprErasureWorker together against a shared
// in-memory fake Postgres, so it exercises the actual fan-out / completion-gate logic end-to-end
// (not a stubbed anonymizerService like the other worker-level guardrails in this directory).

interface FakeOrder {
  id: string;
  customer_id: string;
  location_id: string;
  anonymized_at: string | null;
  delivery_photo_key: string | null;
  delivery_lat: number | null;
  delivery_lng: number | null;
  delivery_address: string | null;
  _simulateNotFound: boolean;
}

interface FakeDb {
  gdprRequests: Map<string, any>;
  customers: Map<string, any>;
  orders: Map<string, FakeOrder>;
  ratings: Map<string, any>;
  auditLog: any[][];
}

function makeFakeDb(): FakeDb {
  return { gdprRequests: new Map(), customers: new Map(), orders: new Map(), ratings: new Map(), auditLog: [] };
}

// One dispatcher recognizes every SQL shape issued by both AnonymizerService and
// GdprErasureWorker (both PRE-FIX and POST-FIX shapes for the confirm query, so this same test
// file proves red against the pre-fix code and green against the fix — see the report).
function execQuery(db: FakeDb, sql: string, params: any[] = []) {
  const s = sql;

  if (/^\s*(BEGIN|COMMIT|ROLLBACK)/i.test(s)) return { rowCount: 0, rows: [] };

  // Worker: claim pending requests.
  if (/FOR UPDATE SKIP LOCKED/i.test(s)) {
    const rows = [...db.gdprRequests.values()]
      .filter((r) => r.status === 'pending')
      .slice(0, 10)
      .map((r) => ({ id: r.id, location_id: r.location_id, customer_id: r.customer_id, subject_phone: r.subject_phone, metadata: r.metadata }));
    return { rowCount: rows.length, rows };
  }

  // Worker: claim -> in_progress.
  if (/UPDATE\s+gdpr_erasure_requests\s+SET\s+status\s*=\s*'in_progress'/i.test(s)) {
    const [id] = params;
    const r = db.gdprRequests.get(id);
    if (r) r.status = 'in_progress';
    return { rowCount: r ? 1 : 0, rows: [] };
  }

  // Worker: POST-FIX subject-graph confirm re-read (REV-S9-3).
  if (/customer_anonymized_at/i.test(s)) {
    const [customerId, locationId] = params;
    const cust = db.customers.get(customerId);
    if (!cust || cust.location_id !== locationId) return { rowCount: 0, rows: [] };
    const ordersRemaining = [...db.orders.values()].filter(
      (o) => o.customer_id === customerId && o.location_id === locationId && o.anonymized_at == null,
    ).length;
    const ratingsRemaining = [...db.ratings.values()].filter((r) => {
      const o = db.orders.get(r.order_id);
      return o && o.customer_id === customerId && o.location_id === locationId && r.feedback != null;
    }).length;
    return {
      rowCount: 1,
      rows: [{ customer_anonymized_at: cust.anonymized_at, orders_remaining: ordersRemaining, ratings_remaining: ratingsRemaining }],
    };
  }

  // Worker: PRE-FIX confirm re-read shape (customers-only) — recognized so a red run against the
  // unfixed production code exercises the REAL defect (false-early completion) rather than an
  // unrelated "unrecognized query" failure.
  if (/SELECT\s+anonymized_at\s+FROM\s+customers\s+WHERE\s+id\s*=\s*\$1\s+AND\s+location_id\s*=\s*\$2/i.test(s)) {
    const [id, locationId] = params;
    const cust = db.customers.get(id);
    if (!cust || cust.location_id !== locationId) return { rowCount: 0, rows: [] };
    return { rowCount: 1, rows: [{ anonymized_at: cust.anonymized_at }] };
  }

  // Worker: terminal completed / failed writes.
  if (/UPDATE\s+gdpr_erasure_requests[\s\S]*status\s*=\s*'completed'/i.test(s)) {
    const [, id] = params;
    const r = db.gdprRequests.get(id);
    if (r) {
      r.status = 'completed';
      r.metadata = params[0];
      r.completed_at = new Date().toISOString();
      if (/subject_phone\s*=\s*NULL/i.test(s)) r.subject_phone = null;
    }
    return { rowCount: r ? 1 : 0, rows: [] };
  }
  if (/UPDATE\s+gdpr_erasure_requests[\s\S]*status\s*=\s*'failed'/i.test(s)) {
    const [id] = params;
    const r = db.gdprRequests.get(id);
    if (r) { r.status = 'failed'; r.error_message = 'erasure incomplete'; }
    return { rowCount: r ? 1 : 0, rows: [] };
  }

  // Worker: R2-5 true-tenant lookup (SELECT location_id FROM customers WHERE id = $1, no AND/FOR UPDATE).
  if (/FROM\s+customers\s+WHERE\s+id\s*=\s*\$1/i.test(s) && !/FOR\s+UPDATE/i.test(s) && !/AND\s+location_id/i.test(s)) {
    const [id] = params;
    const cust = db.customers.get(id);
    return cust ? { rowCount: 1, rows: [{ location_id: cust.location_id }] } : { rowCount: 0, rows: [] };
  }

  // AnonymizerService.anonymizeCustomer: lock read.
  if (/FROM\s+customers\s+WHERE\s+id\s*=\s*\$1\s+AND\s+location_id\s*=\s*\$2/i.test(s) && /FOR\s+UPDATE/i.test(s)) {
    const [id, locationId] = params;
    const cust = db.customers.get(id);
    if (!cust || cust.location_id !== locationId) return { rowCount: 0, rows: [] };
    return { rowCount: 1, rows: [{ anonymized_at: cust.anonymized_at, location_id: cust.location_id }] };
  }

  // AnonymizerService.anonymizeCustomer: the erasure UPDATE.
  if (/UPDATE\s+customers\s+SET/i.test(s)) {
    const [id] = params;
    const cust = db.customers.get(id);
    if (cust) cust.anonymized_at = new Date().toISOString();
    return { rowCount: cust ? 1 : 0, rows: [] };
  }

  // AnonymizerService.columnExists (avatar_key probe) — simulate "column does not exist" so the
  // avatar-purge branch (irrelevant to this guardrail) is skipped.
  if (/pg_attribute/i.test(s)) return { rowCount: 0, rows: [] };

  // AnonymizerService.anonymizeOrder: lock read.
  if (/FROM\s+orders\s+WHERE\s+id\s*=\s*\$1\s+AND\s+location_id\s*=\s*\$2/i.test(s) && /FOR\s+UPDATE/i.test(s)) {
    const [id, locationId] = params;
    const order = db.orders.get(id);
    if (!order || order._simulateNotFound || order.location_id !== locationId) return { rowCount: 0, rows: [] };
    return { rowCount: 1, rows: [{ anonymized_at: order.anonymized_at, location_id: order.location_id, delivery_photo_key: order.delivery_photo_key }] };
  }

  // AnonymizerService.anonymizeOrder: the erasure UPDATE (now includes delivery_lat/lng, GAP-B).
  if (/UPDATE\s+orders\s+SET/i.test(s)) {
    const [id] = params;
    const order = db.orders.get(id);
    if (order) {
      order.delivery_address = null;
      order.delivery_photo_key = null;
      order.delivery_lat = null;
      order.delivery_lng = null;
      order.anonymized_at = new Date().toISOString();
    }
    return { rowCount: order ? 1 : 0, rows: [] };
  }

  // AnonymizerService.anonymizeOrder: the new order_ratings.feedback purge (GAP-C).
  if (/UPDATE\s+order_ratings\s+SET/i.test(s)) {
    const [orderId, locationId] = params;
    const rating = db.ratings.get(orderId);
    if (rating && rating.location_id === locationId) rating.feedback = null;
    return { rowCount: rating ? 1 : 0, rows: [] };
  }

  // The new GDPR fan-out: enumerate the subject's orders.
  if (/FROM\s+orders\s+WHERE\s+customer_id/i.test(s)) {
    const [customerId, locationId] = params;
    const rows = [...db.orders.values()]
      .filter((o) => o.customer_id === customerId && o.location_id === locationId)
      .map((o) => ({ id: o.id }));
    return { rowCount: rows.length, rows };
  }

  if (/INSERT\s+INTO\s+anonymization_audit_log/i.test(s)) {
    db.auditLog.push(params);
    return { rowCount: 1, rows: [] };
  }

  return { rowCount: 0, rows: [] };
}

function makePool(db: FakeDb) {
  const makeClient = () => ({
    query: async (sql: string, params: any[] = []) => execQuery(db, sql, params),
    release() {},
  });
  return {
    connect: async () => makeClient(),
    query: async (sql: string, params: any[] = []) => execQuery(db, sql, params),
  };
}

function makeHarness(opts: {
  requestId: string;
  customerId: string;
  locationId: string;
  subjectPhone: string | null;
  orders: Array<{ id: string; photoKey: string | null; feedback: string | null; simulateNotFound?: boolean }>;
}) {
  const db = makeFakeDb();
  db.gdprRequests.set(opts.requestId, {
    id: opts.requestId,
    location_id: opts.locationId,
    customer_id: opts.customerId,
    subject_phone: opts.subjectPhone,
    status: 'pending',
    metadata: {},
    completed_at: null,
    error_message: null,
  });
  db.customers.set(opts.customerId, { id: opts.customerId, location_id: opts.locationId, anonymized_at: null });
  for (const o of opts.orders) {
    db.orders.set(o.id, {
      id: o.id,
      customer_id: opts.customerId,
      location_id: opts.locationId,
      anonymized_at: null,
      delivery_photo_key: o.photoKey,
      delivery_lat: 50.45,
      delivery_lng: 30.52,
      delivery_address: '221B Baker St',
      _simulateNotFound: !!o.simulateNotFound,
    });
    db.ratings.set(o.id, { order_id: o.id, location_id: opts.locationId, feedback: o.feedback });
  }

  const storageDeletes: string[] = [];
  const publishes: any[] = [];
  const pool = makePool(db);
  const storage = { put: async () => {}, get: async () => null, delete: async (key: string) => { storageDeletes.push(key); } };
  const messageBus = { publish: async (channel: string, payload: any) => { publishes.push({ channel, payload }); } };
  const boss = { work: async () => {}, send: async () => {} };

  const anonymizerService = new AnonymizerService(pool as any, messageBus as any, storage as any);
  const worker = new GdprErasureWorker(pool as any, boss as any, messageBus as any, anonymizerService);

  return { db, worker, storageDeletes, publishes };
}

test('GDPR customer erasure fans out to every one of the subject\'s orders (GAP-A), nulls delivery GPS (GAP-B) and rating feedback (GAP-C), and erases subject_phone on completion (REV-S9-5)', async () => {
  const REQUEST_ID = 'req-A';
  const CUSTOMER_ID = 'cust-A';
  const LOCATION_ID = 'loc-A';
  const { db, worker, storageDeletes } = makeHarness({
    requestId: REQUEST_ID,
    customerId: CUSTOMER_ID,
    locationId: LOCATION_ID,
    subjectPhone: '+15551230000',
    orders: [
      { id: 'order-A1', photoKey: 'delivery-photos/a1.jpg', feedback: 'Great courier!' },
      { id: 'order-A2', photoKey: 'delivery-photos/a2.jpg', feedback: 'Fast delivery' },
    ],
  });

  await (worker as any).run();

  const request = db.gdprRequests.get(REQUEST_ID);
  assert.equal(request.status, 'completed', 'a fully-erased subject-graph must complete');
  assert.equal(request.subject_phone, null, "REV-S9-5: the erasure record's own subject_phone must be nulled on completion");

  for (const orderId of ['order-A1', 'order-A2']) {
    const order = db.orders.get(orderId)!;
    assert.ok(order.anonymized_at, `GAP-A: ${orderId} must be reached by the customer-erasure fan-out`);
    assert.equal(order.delivery_lat, null, `GAP-B: ${orderId}.delivery_lat must be nulled`);
    assert.equal(order.delivery_lng, null, `GAP-B: ${orderId}.delivery_lng must be nulled`);
    assert.equal(order.delivery_address, null, `${orderId}.delivery_address must be nulled (the #74 purge path)`);
    assert.equal(order.delivery_photo_key, null, `${orderId}.delivery_photo_key must be nulled`);
    const rating = db.ratings.get(orderId);
    assert.equal(rating.feedback, null, `GAP-C: ${orderId}'s order_ratings.feedback must be nulled`);
  }

  assert.deepEqual(
    new Set(storageDeletes),
    new Set(['delivery-photos/a1.jpg', 'delivery-photos/a2.jpg']),
    'the #74 R2 doorway-photo purge must fire for every fanned-out order',
  );
});

test('a partial per-order failure keeps the request `failed` (never a false completion) and leaves subject_phone un-erased', async () => {
  const REQUEST_ID = 'req-B';
  const CUSTOMER_ID = 'cust-B';
  const LOCATION_ID = 'loc-B';
  const { db, worker } = makeHarness({
    requestId: REQUEST_ID,
    customerId: CUSTOMER_ID,
    locationId: LOCATION_ID,
    subjectPhone: '+15559998888',
    orders: [
      { id: 'order-B1', photoKey: 'delivery-photos/b1.jpg', feedback: 'ok' },
      { id: 'order-B2', photoKey: null, feedback: null, simulateNotFound: true },
    ],
  });

  await (worker as any).run();

  const request = db.gdprRequests.get(REQUEST_ID);
  assert.equal(request.status, 'failed', 'REV-S9-3: one un-erased order must gate the WHOLE request to failed, never completed');
  assert.equal(request.subject_phone, '+15559998888', 'subject_phone must NOT be erased on a failed (incomplete) erasure');

  assert.ok(
    db.orders.get('order-B1')!.anonymized_at,
    'the order that COULD be locked is still anonymized (tolerated-and-reported, not an all-or-nothing abort)',
  );
  assert.equal(
    db.orders.get('order-B2')!.anonymized_at,
    null,
    'the order that could not be locked remains un-anonymized, which is exactly what the gate must catch',
  );
});

test('a re-run of an already-fully-erased subject (customer+orders+ratings already null) still completes — no over-correction', async () => {
  const REQUEST_ID = 'req-C';
  const CUSTOMER_ID = 'cust-C';
  const LOCATION_ID = 'loc-C';
  const { db, worker } = makeHarness({
    requestId: REQUEST_ID,
    customerId: CUSTOMER_ID,
    locationId: LOCATION_ID,
    subjectPhone: '+15551112222',
    orders: [{ id: 'order-C1', photoKey: null, feedback: null }],
  });
  // Pre-anonymize everything, as if a previous run already completed the erasure.
  db.customers.get(CUSTOMER_ID).anonymized_at = '2026-07-01T00:00:00Z';
  db.orders.get('order-C1')!.anonymized_at = '2026-07-01T00:00:00Z';

  await (worker as any).run();

  assert.equal(
    db.gdprRequests.get(REQUEST_ID).status,
    'completed',
    'goal-state already reached (idempotent) must still complete, not fail',
  );
});
