import test from 'node:test';
import assert from 'node:assert/strict';

// Self-contained dummy env so loadEnv() inside the service succeeds without the
// real (prod) .env. Set BEFORE importing the service (dynamic import below).
Object.assign(process.env, {
  NODE_ENV: 'test',
  APP_BASE_URL: 'https://x.test',
  DATABASE_URL_OPERATIONAL: 'postgres://u:p@localhost:5432/d',
  DATABASE_URL_SESSION: 'postgres://u:p@localhost:5432/d',
  DATABASE_URL_MIGRATIONS: 'postgres://u:p@localhost:5432/d',
  REDIS_URL: 'redis://localhost:6379',
  JWT_PRIVATE_KEY: 'x', JWT_PUBLIC_KEY: 'x', JWT_KID: 'x',
  GOOGLE_CLIENT_ID: 'x', GOOGLE_CLIENT_SECRET: 'x',
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
// `ownerRow` overrides fields of the returned row (status / assigned_at) so terminal-state
// and expired-window branches can be exercised.
function makeClient(ownerRow: Record<string, any> = {}) {
  const queries: { sql: string; params: any[] }[] = [];
  const client: any = {
    queries,
    query: async (sql: string, params: any[] = []) => {
      queries.push({ sql, params });
      if (/SELECT[\s\S]*FOR UPDATE/i.test(sql)) {
        const courierId = params[1];
        if (courierId === OWNER_COURIER) {
          return { rowCount: 1, rows: [{ order_id: ORDER, assigned_at: new Date().toISOString(), status: 'assigned', courier_id: OWNER_COURIER, ...ownerRow }] };
        }
        return { rowCount: 0, rows: [] };
      }
      return { rowCount: 1, rows: [] };
    },
  };
  return client;
}
// Spy bus so we can assert publish was called exactly once with the correct payload.
function makeBus() {
  const calls: any[][] = [];
  return { calls, publish: async (...a: any[]) => { calls.push(a); } } as any;
}

test('cross-courier IDOR on accept (ADR courier-assignment-idor)', async (t) => {
  const { acceptCourierAssignment } = await import('../src/lib/courierAssignmentService.js');
  const { BUS_CHANNELS } = await import('../src/lib/registry.js');

  await t.test('a DIFFERENT courier accepting the assignment is rejected 404', async () => {
    const client = makeClient();
    await assert.rejects(
      () => acceptCourierAssignment(client, ASSIGN, 'loc', ATTACKER_COURIER, { messageBus: makeBus() }),
      (err: any) => err.statusCode === 404,
      'attacker courier must get 404, not hijack the assignment',
    );
  });

  await t.test('a courier from a DIFFERENT location/tenant is also rejected 404', async () => {
    // The courier_id predicate blocks any courier_id != the owner, regardless of the
    // location passed in the request. (True DB-level cross-tenant RLS isolation — RLS
    // isolates by location only — needs a live staging run with a real 2nd tenant.)
    // TODO(needs_staging): assert cross-tenant RLS isolation against real 2nd tenant.
    const client = makeClient();
    await assert.rejects(
      () => acceptCourierAssignment(client, ASSIGN, 'other-location', ATTACKER_COURIER, { messageBus: makeBus() }),
      (err: any) => err.statusCode === 404,
      'a foreign-tenant courier must not hijack via a different location',
    );
  });

  await t.test('the lookup is scoped by courier_id (the IDOR predicate)', async () => {
    const client = makeClient();
    const bus = makeBus();
    await acceptCourierAssignment(client, ASSIGN, 'loc', OWNER_COURIER, { messageBus: bus });
    const select = client.queries.find((q: any) => /SELECT[\s\S]*FOR UPDATE/i.test(q.sql));
    assert.ok(select, 'a FOR UPDATE select must run');
    assert.match(select.sql, /courier_id\s*=\s*\$2/, 'SELECT must filter by courier_id = $2');
    assert.deepEqual(select.params, [ASSIGN, OWNER_COURIER], 'SELECT must be parameterized with [assignmentId, courierId]');
    const update = client.queries.find((q: any) => /UPDATE courier_assignments/i.test(q.sql));
    assert.match(update.sql, /courier_id\s*=\s*\$2/, 'UPDATE must also be scoped by courier_id (defense in depth)');
    assert.deepEqual(update.params, [ASSIGN, OWNER_COURIER], 'UPDATE must be parameterized with [assignmentId, courierId]');
  });

  await t.test('a non-"assigned" (terminal/in-progress) assignment is rejected 400', async () => {
    const client = makeClient({ status: 'accepted' });
    await assert.rejects(
      () => acceptCourierAssignment(client, ASSIGN, 'loc', OWNER_COURIER, { messageBus: makeBus() }),
      (err: any) => err.statusCode === 400,
      'accepting an already-accepted assignment must 400, not re-accept',
    );
  });

  await t.test('an expired acceptance window is rejected 410', async () => {
    const client = makeClient({ assigned_at: new Date(Date.now() - 999_999).toISOString() });
    await assert.rejects(
      () => acceptCourierAssignment(client, ASSIGN, 'loc', OWNER_COURIER, { messageBus: makeBus() }),
      (err: any) => err.statusCode === 410,
      'accepting past the window must 410',
    );
  });

  await t.test('the legitimate owning courier still succeeds AND publishes', async () => {
    const client = makeClient();
    const bus = makeBus();
    const res = await acceptCourierAssignment(client, ASSIGN, 'loc', OWNER_COURIER, { messageBus: bus });
    assert.equal(res.orderId, ORDER);
    assert.equal(bus.calls.length, 1, 'messageBus.publish must be called exactly once');
    assert.equal(bus.calls[0][0], BUS_CHANNELS.ORDER_COURIER_ACCEPTED, 'must publish on the ORDER_COURIER_ACCEPTED channel');
    assert.deepEqual(bus.calls[0][1], { orderId: ORDER, locationId: 'loc', courierId: OWNER_COURIER }, 'publish payload must carry the order/location/courier');
  });
});
