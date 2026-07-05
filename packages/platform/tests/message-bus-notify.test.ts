import test from 'node:test';
import assert from 'node:assert/strict';

// message-bus.js calls loadEnv() at module load; stub the required env BEFORE the
// dynamic import so the pure (pool-stubbed) test runs without real infra.
const ENV_STUB: Record<string, string> = {
  NODE_ENV: 'test', APP_BASE_URL: 'http://localhost:3000',
  DATABASE_URL_OPERATIONAL: 'postgres://u:p@localhost:5432/db',
  DATABASE_URL_SESSION: 'postgres://u:p@localhost:5432/db',
  DATABASE_URL_MIGRATIONS: 'postgres://u:p@localhost:5432/db',
  REDIS_URL: 'redis://localhost:6379', JWT_PRIVATE_KEY: 'test-priv', JWT_PUBLIC_KEY: 'test-pub',
  JWT_KID: 'test', GOOGLE_CLIENT_ID: 'test', GOOGLE_CLIENT_SECRET: 'test',
  VAPID_PUBLIC_KEY: 'test', VAPID_PRIVATE_KEY: 'test', IP_HASH_SALT: 'test',
};
for (const [k, v] of Object.entries(ENV_STUB)) if (!process.env[k]) process.env[k] = v;
const { PgMessageBus } = await import('../src/message-bus.js');

// Extract the single-quoted payload from a `NOTIFY "chan", '...'` statement and
// undo the '' escaping, so we can JSON.parse what actually went on the wire.
function notifyPayload(sql: string): string {
  const first = sql.indexOf("'");
  const last = sql.lastIndexOf("'");
  return sql.slice(first + 1, last).replace(/''/g, "'");
}

function stubPool() {
  const calls: string[] = [];
  return {
    calls,
    query: async (sql: string) => { calls.push(sql); return { rowCount: 0, rows: [] }; },
  };
}

test('PgMessageBus.publish targets the pool and guards the NOTIFY size limit', async (t) => {
  await t.test('M2: NOTIFY goes to the pool, never the dedicated listener client', async () => {
    const pool = stubPool();
    const bus = new PgMessageBus(pool as any);
    let listenerUsed = false;
    // Simulate an active LISTEN connection — publish must NOT borrow it.
    (bus as any).listenerClient = { query: async () => { listenerUsed = true; return {}; } };

    await bus.publish('order:o1', { type: 'order.status', orderId: 'o1' });

    assert.equal(listenerUsed, false, 'NOTIFY must run on the pool, not the listener client');
    assert.equal(pool.calls.length, 1, 'exactly one NOTIFY issued on the pool');
    assert.match(pool.calls[0], /^NOTIFY "order:o1"/);
  });

  await t.test('M1: a normal payload is sent intact (not truncated)', async () => {
    const pool = stubPool();
    const bus = new PgMessageBus(pool as any);

    await bus.publish('order:o1', { type: 'order.status', orderId: 'o1', status: 'READY' });

    const parsed = JSON.parse(notifyPayload(pool.calls[0]));
    assert.equal(parsed.type, 'order.status');
    assert.equal(parsed.status, 'READY');
    assert.equal(parsed._truncated, undefined, 'small payloads must not be truncated');
  });

  await t.test('M1: an oversized payload is truncated but keeps type + data.id', async () => {
    const pool = stubPool();
    const bus = new PgMessageBus(pool as any);

    // A delta whose items_summary blows past Postgres' 8000-byte NOTIFY cap.
    const huge = 'x'.repeat(9000);
    await bus.publish('location:l1:dashboard', {
      type: 'order.status',
      data: { id: 'o1', status: 'READY', items_summary: huge },
    });

    const payload = notifyPayload(pool.calls[0]);
    const parsed = JSON.parse(payload);
    assert.equal(parsed._truncated, true, 'oversized payloads must be flagged _truncated');
    assert.equal(parsed.type, 'order.status', 'event type must survive truncation');
    assert.equal(parsed.data.id, 'o1', 'the order id must survive so the client can refetch');
    assert.ok(
      Buffer.byteLength(payload, 'utf8') <= 7800,
      `truncated payload must fit under the NOTIFY cap (was ${Buffer.byteLength(payload, 'utf8')}B)`,
    );
  });
});
