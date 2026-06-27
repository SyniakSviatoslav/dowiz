import test from 'node:test';
import assert from 'node:assert/strict';
import { QUEUE_NAMES } from '@deliveryos/shared-types';
import { registerHandlers } from '../src/handlers.js';

// L2 regression: when an order times out, the handler must auto-cancel it AND
// broadcast the CANCELLED status to the customer status page (order:{id}) and
// the owner dashboard (location:{id}:dashboard). Previously it was silent, so a
// timed-out order only flipped on the next page refresh.

function setup(cancelRowCount: number, opts: { dbThrows?: boolean } = {}) {
  const handlers: Record<string, (p: any) => Promise<void>> = {};
  const enqueued: Array<{ name: string; payload: any; opts: any }> = [];
  // Mirror the real QueueProvider surface: the handler calls BOTH work() (to
  // register) and enqueue() (to emit the owner notification). A mock missing
  // enqueue would let a `TypeError: queue.enqueue is not a function` get
  // swallowed by the handler's try-catch and pass silently.
  const queue = {
    work: async (name: string, fn: any) => { handlers[name] = fn; },
    enqueue: async (name: string, payload: any, options?: any) => {
      enqueued.push({ name, payload, opts: options });
    },
  } as any;
  const published: Array<{ ch: string; msg: any }> = [];
  const messageBus = { publish: async (ch: string, msg: any) => { published.push({ ch, msg }); } } as any;
  const queries: Array<{ sql: string; params: any[] }> = [];
  const pool = {
    query: async (sql: string, params: any[]) => {
      queries.push({ sql, params });
      if (sql.includes('UPDATE orders')) {
        if (opts.dbThrows) throw new Error('db unavailable');
        return cancelRowCount
          ? { rowCount: 1, rows: [{ id: params[0], status: 'CANCELLED', location_id: 'loc1' }] }
          : { rowCount: 0, rows: [] };
      }
      return { rowCount: 1, rows: [] };
    },
  } as any;
  registerHandlers(queue, pool, messageBus);
  return { handlers, published, queries, enqueued };
}

test('ORDER_TIMEOUT handler', async (t) => {
  await t.test('auto-cancel → history + CANCELLED published to order + dashboard + notify enqueued', async () => {
    const { handlers, published, queries, enqueued } = setup(1);
    await handlers[QUEUE_NAMES.ORDER_TIMEOUT]({ orderId: 'o1' });

    // History row must be an INSERT into order_status_history, attributed to the
    // system timeout actor, with exactly (orderId, locationId) params — not just
    // any query that mentions the table name.
    const hist = queries.find((q) => q.sql.includes('order_status_history'));
    assert.ok(hist, 'history insert missing');
    assert.match(hist!.sql, /INSERT INTO order_status_history/, 'history must be an INSERT');
    assert.match(hist!.sql, /'system:timeout'/, 'history actor must be system:timeout');
    assert.deepEqual(hist!.params, ['o1', 'loc1'], 'history params must be (orderId, locationId)');

    const order = published.find((p) => p.ch === 'order:o1');
    assert.ok(order, 'order:{id} publish missing');
    assert.equal(order!.msg.status, 'CANCELLED');
    assert.equal(order!.msg.type, 'order.status');
    const dash = published.find((p) => p.ch === 'location:loc1:dashboard');
    assert.ok(dash, 'dashboard publish missing');
    assert.equal(dash!.msg.data.status, 'CANCELLED');

    // Owner notification: exactly one enqueue to NOTIFY_TELEGRAM_SEND with the
    // timeout-cancelled event, order/location ids, and a singletonKey-keyed
    // dedup so the per-order handler and the reconciliation sweep collapse to one.
    const notify = enqueued.filter((e) => e.name === QUEUE_NAMES.NOTIFY_TELEGRAM_SEND);
    assert.equal(notify.length, 1, 'must enqueue exactly one timeout-cancelled notification');
    assert.equal(notify[0].payload.event, 'order.timeout_cancelled');
    assert.equal(notify[0].payload.entity_id, 'o1');
    assert.equal(notify[0].payload.location_id, 'loc1');
    assert.equal(notify[0].payload.dedupKey, 'order.timeout_cancelled:o1:loc1');
    assert.equal(notify[0].opts.singletonKey, 'order.timeout_cancelled:o1:loc1');
  });

  await t.test('no-op when already transitioned (rowCount 0) → no publish, no enqueue', async () => {
    const { handlers, published, enqueued } = setup(0);
    await handlers[QUEUE_NAMES.ORDER_TIMEOUT]({ orderId: 'o2' });
    assert.equal(published.length, 0, 'must not publish when nothing was cancelled');
    assert.equal(enqueued.length, 0, 'must not enqueue notification when nothing was cancelled');
  });

  await t.test('missing orderId → no throw, no publish, no enqueue', async () => {
    const { handlers, published, enqueued } = setup(1);
    await handlers[QUEUE_NAMES.ORDER_TIMEOUT]({});
    assert.equal(published.length, 0);
    assert.equal(enqueued.length, 0);
  });

  // Pins the current swallow-and-log behaviour of the outer try-catch: a DB
  // failure on the UPDATE must NOT reject (it would crash the worker / poison the
  // job) and must produce no side effects. NOTE: because the handler resolves on
  // DB error, a retry system that keys off rejected promises will never retry an
  // auto-cancel that failed mid-write — escalate as a product decision, do not
  // change behaviour from this test.
  await t.test('UPDATE throws → handler swallows (resolves), no publish, no enqueue', async () => {
    const { handlers, published, queries, enqueued } = setup(1, { dbThrows: true });
    await assert.doesNotReject(handlers[QUEUE_NAMES.ORDER_TIMEOUT]({ orderId: 'o3' }));
    assert.ok(queries.some((q) => q.sql.includes('UPDATE orders')), 'UPDATE must have been attempted');
    assert.equal(published.length, 0, 'must not publish when the cancel write failed');
    assert.equal(enqueued.length, 0, 'must not enqueue when the cancel write failed');
  });
});
