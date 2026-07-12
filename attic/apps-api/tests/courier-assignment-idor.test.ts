import test from 'node:test';
import assert from 'node:assert/strict';

// Self-contained dummy env so loadEnv() inside the service succeeds without the
// real (prod) .env. Set BEFORE importing the service (dynamic import below).
Object.assign(process.env, {
  NODE_ENV: 'test',
  APP_BASE_URL: 'https://x.test',
  ***REDACTED***: 'postgres://u:p@localhost:5432/d',
  ***REDACTED***: 'postgres://u:p@localhost:5432/d',
  ***REDACTED***: 'postgres://u:p@localhost:5432/d',
  REDIS_URL: 'redis://localhost:6379',
  ***REDACTED***: 'x', ***REDACTED***: 'x', JWT_KID: 'x',
  ***REDACTED***: 'x', ***REDACTED***: 'x',
  VAPID_PUBLIC_KEY: 'x', VAPID_PRIVATE_KEY: 'x',
  IP_HASH_SALT: 'x',
});

const OWNER_COURIER = 'courier-b-owns-it';
const ATTACKER_COURIER = 'courier-a-attacker';
const ASSIGN = 'assignment-uuid';
const ORDER = 'order-uuid';

// Fake PoolClient: the SELECT ... FOR UPDATE returns a row ONLY when the courier_id
// param matches the assignment's real owner — i.e. it models the DB's behaviour AFTER
// the `AND courier_id = $2` predicate is applied. An attacker courier sees rowCount 0.
function makeClient() {
  const queries: { sql: string; params: any[] }[] = [];
  const client: any = {
    queries,
    query: async (sql: string, params: any[] = []) => {
      queries.push({ sql, params });
      if (/SELECT[\s\S]*FOR UPDATE/i.test(sql)) {
        const courierId = params[1];
        if (courierId === OWNER_COURIER) {
          return { rowCount: 1, rows: [{ order_id: ORDER, assigned_at: new Date().toISOString(), status: 'assigned', courier_id: OWNER_COURIER }] };
        }
        return { rowCount: 0, rows: [] };
      }
      return { rowCount: 1, rows: [] };
    },
  };
  return client;
}
const fakeBus: any = { publish: async () => {} };

test('cross-courier IDOR on accept (ADR courier-assignment-idor)', async (t) => {
  const { acceptCourierAssignment } = await import('../src/lib/courierAssignmentService.js');

  await t.test('a DIFFERENT courier accepting the assignment is rejected 404', async () => {
    const client = makeClient();
    await assert.rejects(
      () => acceptCourierAssignment(client, ASSIGN, 'loc', ATTACKER_COURIER, { messageBus: fakeBus }),
      (err: any) => err.statusCode === 404,
      'attacker courier must get 404, not hijack the assignment',
    );
  });

  await t.test('the lookup is scoped by courier_id (the IDOR predicate)', async () => {
    const client = makeClient();
    await acceptCourierAssignment(client, ASSIGN, 'loc', OWNER_COURIER, { messageBus: fakeBus });
    const select = client.queries.find((q: any) => /SELECT[\s\S]*FOR UPDATE/i.test(q.sql));
    assert.ok(select, 'a FOR UPDATE select must run');
    assert.match(select.sql, /courier_id\s*=\s*\$2/, 'SELECT must filter by courier_id = $2');
    assert.deepEqual(select.params, [ASSIGN, OWNER_COURIER], 'SELECT must be parameterized with [assignmentId, courierId]');
    const update = client.queries.find((q: any) => /UPDATE courier_assignments/i.test(q.sql));
    assert.match(update.sql, /courier_id\s*=\s*\$2/, 'UPDATE must also be scoped by courier_id (defense in depth)');
  });

  await t.test('the legitimate owning courier still succeeds', async () => {
    const client = makeClient();
    const res = await acceptCourierAssignment(client, ASSIGN, 'loc', OWNER_COURIER, { messageBus: fakeBus });
    assert.equal(res.orderId, ORDER);
  });
});
