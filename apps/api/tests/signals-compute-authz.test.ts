import { test } from 'node:test';
import assert from 'node:assert/strict';
import { computeSignals } from '../src/lib/signals/compute.js';

// Site #4 / F5 (audit-fix-authz resolution.md §2): lib/signals/compute.ts's no-show lookup read
// `FROM customers WHERE id = $1` with no location_id predicate — GET /:loc/signals/compute could
// pull ANY tenant's customer reputation (no-show count) by supplying a foreign customer_id. The
// fix adds `AND location_id = $2`, threaded from the caller's requireLocationAccess-verified
// locationId. Pure-function test — no fastify/DB needed, just the exported computeSignals(pool, ...).

const CUSTOMER_B = 'cust-b';
const LOC_A = 'loc-a';
const LOC_B = 'loc-b';

function makePool() {
  const queries: { sql: string; params: any[] }[] = [];
  return {
    queries,
    query: async (sql: string, params: any[] = []) => {
      queries.push({ sql, params });
      if (/FROM\s+customers\s+WHERE/i.test(sql)) {
        const [customerId, locationId] = params;
        // Ground truth: CUSTOMER_B has a strong no-show signal, but belongs to LOC_B.
        if (customerId === CUSTOMER_B && (params.length < 2 || locationId === LOC_B)) {
          return { rowCount: 1, rows: [{ no_show_count: 10, completed_count: 1, last_no_show_at: new Date() }] };
        }
        return { rowCount: 0, rows: [] };
      }
      return { rowCount: 0, rows: [] };
    },
  };
}

test('computeSignals — cross-tenant customer_id (owner-A querying tenant-B customer) → no no_show_recent signal', async () => {
  const pool = makePool();
  const signals = await computeSignals(pool as any, { locationId: LOC_A, customerId: CUSTOMER_B });
  assert.equal(signals.find((s) => s.kind === 'no_show_recent'), undefined,
    'a foreign customer_id must not surface another tenant\'s no-show reputation');
  const call = pool.queries.find((q) => /FROM\s+customers\s+WHERE/i.test(q.sql));
  assert.ok(call, 'the customers lookup is attempted');
  assert.equal(call!.params.length, 2, 'the query must bind BOTH customerId and locationId');
  assert.equal(call!.params[1], LOC_A, 'the location predicate binds the CALLER\'s verified location, not the target row\'s');
});

test('computeSignals — own-tenant customer_id surfaces the no_show_recent signal (no regression)', async () => {
  const pool = makePool();
  const signals = await computeSignals(pool as any, { locationId: LOC_B, customerId: CUSTOMER_B });
  const signal = signals.find((s) => s.kind === 'no_show_recent');
  assert.ok(signal, 'own-tenant lookup must still surface the signal');
});
