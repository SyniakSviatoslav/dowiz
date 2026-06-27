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
// tsc emits to dist/; src/ holds only .ts, so a '../src/*.js' import is
// ERR_MODULE_NOT_FOUND and the whole suite silently never runs. Import the
// compiled artifact (run `pnpm --filter @deliveryos/platform build` first).
const { PgMessageBus } = await import('../dist/message-bus.js');

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
    // Full statement shape — a payload-less or malformed NOTIFY must fail here,
    // not slip past a prefix-only match.
    assert.match(pool.calls[0], /^NOTIFY "order:o1", '\{.*\}'$/);
    const parsed = JSON.parse(notifyPayload(pool.calls[0]));
    assert.equal(parsed.type, 'order.status');
    assert.equal(parsed.orderId, 'o1');
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

  await t.test('M3: a rejecting pool.query is swallowed — publish resolves and logs, never throws', async () => {
    const calls: string[] = [];
    const pool = {
      query: async (sql: string) => { calls.push(sql); throw new Error('connection terminated'); },
    };
    const bus = new PgMessageBus(pool as any);

    // Capture the swallow: publish() catches and console.error's, returning void.
    const origError = console.error;
    const errs: unknown[][] = [];
    console.error = (...args: unknown[]) => { errs.push(args); };
    try {
      // Must RESOLVE (not reject) — this documents the deliberate swallow.
      await assert.doesNotReject(() => bus.publish('order:o1', { type: 'order.status', orderId: 'o1' }));
    } finally {
      console.error = origError;
    }

    assert.equal(calls.length, 1, 'the failing NOTIFY was attempted exactly once on the pool');
    // The swallow MUST be observable on the error log, anchored to the channel —
    // a silent swallow (no log) would be a regression.
    assert.ok(
      errs.some((a) => a.some((x) => typeof x === 'string' && x.includes('Publish error')) && a.includes('order:o1')),
      'a swallowed publish error must be logged with its channel',
    );
  });
});

test('PgMessageBus subscribe/dispatch/unsubscribe surface', async (t) => {
  // A listenerClient stub recording the LISTEN/UNLISTEN commands sent on the wire.
  function stubListener() {
    const cmds: string[] = [];
    return {
      cmds,
      query: async (sql: string) => { cmds.push(sql); return { rowCount: 1, rows: [] }; },
    };
  }

  await t.test('subscribe registers the handler and issues LISTEN on the listener client', async () => {
    const bus = new PgMessageBus(stubPool() as any);
    const listener = stubListener();
    (bus as any).listenerClient = listener;
    const handler = () => {};

    await bus.subscribe('order:o1', handler);

    assert.deepEqual(listener.cmds, ['LISTEN "order:o1"'], 'exactly one LISTEN issued for the new channel');
    assert.deepEqual((bus as any).handlers.get('order:o1'), [handler], 'handler registered under the channel');
  });

  await t.test('subscribe on an existing channel adds the handler but does NOT re-LISTEN', async () => {
    const bus = new PgMessageBus(stubPool() as any);
    const listener = stubListener();
    (bus as any).listenerClient = listener;

    await bus.subscribe('order:o1', () => {});
    await bus.subscribe('order:o1', () => {});

    assert.equal(listener.cmds.length, 1, 'second subscribe to the same channel must not LISTEN again');
    assert.equal((bus as any).handlers.get('order:o1').length, 2, 'both handlers registered');
  });

  await t.test('dispatch calls every handler even when one throws synchronously', () => {
    const bus = new PgMessageBus(stubPool() as any);
    const seen: string[] = [];
    const origError = console.error;
    console.error = () => {};
    try {
      const handlers = [
        () => { seen.push('a'); },
        () => { throw new Error('bad subscriber'); },
        () => { seen.push('c'); },
      ];
      // Must not throw despite the middle handler throwing.
      assert.doesNotThrow(() => (bus as any).dispatch('order:o1', handlers, { type: 'x' }));
    } finally {
      console.error = origError;
    }
    assert.deepEqual(seen, ['a', 'c'], 'a throwing handler must not stop the others');
  });

  await t.test('unsubscribe of the last handler removes the channel and issues UNLISTEN', async () => {
    const bus = new PgMessageBus(stubPool() as any);
    const listener = stubListener();
    (bus as any).listenerClient = listener;
    const handler = () => {};

    await bus.subscribe('order:o1', handler);
    bus.unsubscribe('order:o1', handler);

    assert.equal((bus as any).handlers.has('order:o1'), false, 'channel dropped once empty');
    assert.ok(listener.cmds.includes('UNLISTEN "order:o1"'), 'UNLISTEN issued on the listener client');
  });

  await t.test('unsubscribe of one of several handlers keeps the channel LISTENing', async () => {
    const bus = new PgMessageBus(stubPool() as any);
    const listener = stubListener();
    (bus as any).listenerClient = listener;
    const keep = () => {};
    const drop = () => {};

    await bus.subscribe('order:o1', keep);
    await bus.subscribe('order:o1', drop);
    bus.unsubscribe('order:o1', drop);

    assert.deepEqual((bus as any).handlers.get('order:o1'), [keep], 'only the named handler removed');
    assert.ok(!listener.cmds.includes('UNLISTEN "order:o1"'), 'no UNLISTEN while handlers remain');
  });
});
