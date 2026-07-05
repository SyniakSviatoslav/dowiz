import { test } from 'node:test';
import assert from 'node:assert/strict';

// Site #2 / B6 (audit-fix-authz resolution.md §2, proposal.md §3.3.2): the anonymizer sink used
// to self-derive its tenant scope from the row being mutated (`options.subject?.locationId ||
// row.location_id`) — a caller that omitted the scope silently "proved" it from the very row it
// was about to touch, which is not a proof at all. The fix makes the scope REQUIRED: omitting it
// now throws (fail-closed) instead of silently substituting the row's own location_id. No DB
// needed — the throw must fire before any pool access, which this test also asserts by making
// pool/messageBus/storage throw loudly if ever touched.

function poisonedPool() {
  return new Proxy({}, {
    get(_t, prop) {
      throw new Error(`pool.${String(prop)} must never be called before the fail-closed scope check`);
    },
  });
}
function poisonedBus() {
  return new Proxy({}, {
    get(_t, prop) {
      throw new Error(`messageBus.${String(prop)} must never be called before the fail-closed scope check`);
    },
  });
}

test('anonymize() customerId path without subject.locationId → throws (fail-closed), never touches the pool', async () => {
  const { AnonymizerService } = await import('../src/lib/anonymizer/index.js');
  const service = new AnonymizerService(poisonedPool() as any, poisonedBus() as any);
  await assert.rejects(
    () => service.anonymize({ scope: 'gdpr', subject: { customerId: 'cust-1' } } as any),
    /explicit subject\.locationId scope/i,
  );
});

test('anonymize() orderId path without subject.locationId → throws (fail-closed), never touches the pool', async () => {
  const { AnonymizerService } = await import('../src/lib/anonymizer/index.js');
  const service = new AnonymizerService(poisonedPool() as any, poisonedBus() as any);
  await assert.rejects(
    () => service.anonymize({ scope: 'gdpr', subject: { orderId: 'order-1' } } as any),
    /explicit subject\.locationId scope/i,
  );
});

test('anonymize() dryRun path without subject.locationId → throws (fail-closed), never touches the pool', async () => {
  const { AnonymizerService } = await import('../src/lib/anonymizer/index.js');
  const service = new AnonymizerService(poisonedPool() as any, poisonedBus() as any);
  await assert.rejects(
    () => service.anonymize({ scope: 'gdpr', dryRun: true, subject: { customerId: 'cust-1' } } as any),
    /explicit subject\.locationId scope/i,
  );
});

test('anonymize() customerId path WITH subject.locationId does not throw at the scope-check (proceeds to the pool)', async () => {
  const { AnonymizerService } = await import('../src/lib/anonymizer/index.js');
  // A pool that returns "not found" for the lock read — proves the call proceeded PAST the
  // fail-closed check (the previous tests prove it throws before ever reaching this point).
  const pool = {
    connect: async () => ({
      query: async (sql: string) => {
        if (/^\s*(BEGIN|COMMIT|ROLLBACK)/i.test(sql)) return { rowCount: 0, rows: [] };
        if (/FROM\s+customers\s+WHERE/i.test(sql)) return { rowCount: 0, rows: [] };
        return { rowCount: 0, rows: [] };
      },
      release() {},
    }),
    // REV-S9-1 GDPR customer fan-out (resolution.md) reads the subject's orders via
    // pool.query() directly (mirrors findExpiredCustomers/findExpiredOrders) — 0 rows here so
    // this fail-closed harness (which never models orders) proceeds with no fan-out.
    query: async () => ({ rowCount: 0, rows: [] }),
  };
  const service = new AnonymizerService(pool as any, { publish: async () => {} } as any);
  const result = await service.anonymize({
    scope: 'gdpr',
    subject: { customerId: 'cust-1', locationId: 'loc-1' },
  } as any);
  assert.equal(result.customersAnonymized, 0);
  assert.equal(result.skipped, 1, 'row not found under the scoped predicate → skipped, not thrown');
});
