import { test } from 'node:test';
import assert from 'node:assert/strict';
import { AnonymizerService } from '../src/lib/anonymizer/index.js';

// Guardrail for the S4 media-council gap (REV-S4-7, counsel-opinion.md + resolution.md):
// `anonymizeOrder` nulled every text PII field (address/instructions/handles) on GDPR/retention
// erasure but left `delivery_photo_key` set and purged no R2 object — the only object-delete in
// the whole anonymizer service was the `avatar_key` purge in `anonymizeCustomer`. A customer's
// doorway/entry photo survived erasure of its own order, public-by-key, indefinitely.
//
// Fix (apps/api/src/lib/anonymizer/index.ts, anonymizeOrder): the FOR UPDATE lock query now also
// selects delivery_photo_key; the SAME UPDATE that nulls the other order PII fields also nulls
// delivery_photo_key; the R2 object is purged via the storage provider, mirroring the avatar_key
// purge's error semantics EXACTLY — tolerated-and-reported (try/catch + console.error), never
// rethrown, never rolls back the anonymization transaction (customers.ts precedent, lines ~168-176).

const ORDER_ID = 'order-1';
const LOCATION_ID = 'loc-1';
const PHOTO_KEY = 'delivery-photos/order-1.jpg';

interface HarnessOpts {
  deliveryPhotoKey: string | null;
  /** storage.delete() throws this error instead of resolving, when set. */
  storageDeleteError?: Error;
}

function makeHarness(opts: HarnessOpts) {
  const queries: { sql: string; params: any[] }[] = [];
  const audits: any[][] = [];
  const publishes: { channel: string; payload: any }[] = [];
  const storageDeletes: string[] = [];

  const client = {
    query: async (sql: string, params: any[] = []) => {
      queries.push({ sql, params });
      if (/^\s*(BEGIN|COMMIT|ROLLBACK)/i.test(sql)) return { rowCount: 0, rows: [] };
      if (/SELECT\s+anonymized_at,\s*location_id.*FROM\s+orders/is.test(sql)) {
        return {
          rowCount: 1,
          rows: [{ anonymized_at: null, location_id: LOCATION_ID, delivery_photo_key: opts.deliveryPhotoKey }],
        };
      }
      if (/UPDATE\s+orders\s+SET/i.test(sql)) {
        return { rowCount: 1, rows: [] };
      }
      if (/INSERT\s+INTO\s+anonymization_audit_log/i.test(sql)) {
        audits.push(params);
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
  const messageBus = {
    publish: async (channel: string, payload: any) => { publishes.push({ channel, payload }); },
  };
  const storage = {
    put: async () => {},
    get: async () => null,
    delete: async (key: string) => {
      storageDeletes.push(key);
      if (opts.storageDeleteError) throw opts.storageDeleteError;
    },
  };

  const service = new AnonymizerService(pool as any, messageBus as any, storage as any);
  return { service, queries, audits, publishes, storageDeletes };
}

test('order photo purge: a non-null delivery_photo_key is deleted from storage with the exact key', async () => {
  const { service, storageDeletes } = makeHarness({ deliveryPhotoKey: PHOTO_KEY });

  await service.anonymize({ scope: 'gdpr', subject: { orderId: ORDER_ID, locationId: LOCATION_ID } });

  assert.deepEqual(storageDeletes, [PHOTO_KEY], "storage.delete must be called with exactly the order's delivery_photo_key");
});

test('order photo purge: delivery_photo_key is nulled in the SAME UPDATE that nulls the other order PII fields', async () => {
  const { service, queries } = makeHarness({ deliveryPhotoKey: PHOTO_KEY });

  await service.anonymize({ scope: 'gdpr', subject: { orderId: ORDER_ID, locationId: LOCATION_ID } });

  const updateOrders = queries.filter((q) => /UPDATE\s+orders\s+SET/i.test(q.sql));
  assert.equal(updateOrders.length, 1, 'must be exactly one UPDATE orders (no second cleanup statement)');
  assert.match(updateOrders[0].sql, /delivery_photo_key\s*=\s*NULL/i, 'delivery_photo_key must be nulled');
  assert.match(updateOrders[0].sql, /delivery_address\s*=\s*NULL/i, 'must be the SAME statement that nulls the other PII fields');
});

test('order photo purge: no delivery_photo_key -> storage.delete is never called, no spurious purge', async () => {
  const { service, storageDeletes } = makeHarness({ deliveryPhotoKey: null });

  const result = await service.anonymize({ scope: 'gdpr', subject: { orderId: ORDER_ID, locationId: LOCATION_ID } });

  assert.equal(storageDeletes.length, 0, 'storage.delete must not be called when there is no photo to purge');
  assert.equal(result.storagePurged, 0);
});

test('order photo purge: storage delete failure is tolerated-and-reported (mirrors avatar_key purge) — anonymization still commits, storagePurged stays 0, no throw', async () => {
  const { service, audits, publishes, queries } = makeHarness({
    deliveryPhotoKey: PHOTO_KEY,
    storageDeleteError: new Error('R2 delete failed'),
  });

  const result = await service.anonymize({ scope: 'gdpr', subject: { orderId: ORDER_ID, locationId: LOCATION_ID } });

  assert.equal(result.ordersAnonymized, 1, 'the order anonymization itself must still succeed (tolerated, not fail-loud)');
  assert.equal(result.storagePurged, 0, 'a failed purge must not be counted as purged');
  assert.equal(audits.length, 1, 'the audit row must still be inserted — a storage failure does not roll back the transaction');
  assert.ok(!queries.some((q) => /^\s*ROLLBACK/i.test(q.sql)), 'must COMMIT, never ROLLBACK, on a tolerated storage failure');
  assert.ok(publishes.some((p) => p.channel === 'order.anonymized'), 'the order.anonymized event must still publish');
});

test('order photo purge: top-level anonymize() result.storagePurged reflects the order purge (aggregation was previously dropped for the orderId branch)', async () => {
  const { service } = makeHarness({ deliveryPhotoKey: PHOTO_KEY });

  const result = await service.anonymize({ scope: 'gdpr', subject: { orderId: ORDER_ID, locationId: LOCATION_ID } });

  assert.equal(result.storagePurged, 1, 'the top-level result must count the order-level storage purge');
});
