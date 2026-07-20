import test from 'node:test';
import assert from 'node:assert/strict';
import { QUEUE_NAMES } from '@deliveryos/shared-types';
import { registerHandlers } from '../src/handlers.js';

// L2 regression: when an order times out, the handler must auto-cancel it AND
// broadcast the CANCELLED status to the customer status page (order:{id}) and
// the owner dashboard (location:{id}:dashboard). Previously it was silent, so a
// timed-out order only flipped on the next page refresh.

function setup(cancelRowCount: number) {
  const handlers: Record<string, (p: any) => Promise<void>> = {};
  const queue = { work: async (name: string, fn: any) => { handlers[name] = fn; } } as any;
  const published: Array<{ ch: string; msg: any }> = [];
  const messageBus = { publish: async (ch: string, msg: any) => { published.push({ ch, msg }); } } as any;
  const queries: string[] = [];
  const pool = {
    query: async (sql: string, params: any[]) => {
      queries.push(sql);
      if (sql.includes('UPDATE orders')) {
        return cancelRowCount
          ? { rowCount: 1, rows: [{ id: params[0], status: 'CANCELLED', location_id: 'loc1' }] }
          : { rowCount: 0, rows: [] };
      }
      return { rowCount: 1, rows: [] };
    },
  } as any;
  registerHandlers(queue, pool, messageBus);
  return { handlers, published, queries };
}

test('ORDER_TIMEOUT handler', async (t) => {
  await t.test('auto-cancel → history + CANCELLED published to order + dashboard', async () => {
    const { handlers, published, queries } = setup(1);
    await handlers[QUEUE_NAMES.ORDER_TIMEOUT]({ orderId: 'o1' });

    assert.ok(queries.some((q) => q.includes('order_status_history')), 'history insert missing');
    const order = published.find((p) => p.ch === 'order:o1');
    assert.ok(order, 'order:{id} publish missing');
    assert.equal(order!.msg.status, 'CANCELLED');
    assert.equal(order!.msg.type, 'order.status');
    const dash = published.find((p) => p.ch === 'location:loc1:dashboard');
    assert.ok(dash, 'dashboard publish missing');
    assert.equal(dash!.msg.data.status, 'CANCELLED');
  });

  await t.test('no-op when already transitioned (rowCount 0) → no publish', async () => {
    const { handlers, published } = setup(0);
    await handlers[QUEUE_NAMES.ORDER_TIMEOUT]({ orderId: 'o2' });
    assert.equal(published.length, 0, 'must not publish when nothing was cancelled');
  });

  await t.test('missing orderId → no throw, no publish', async () => {
    const { handlers, published } = setup(1);
    await handlers[QUEUE_NAMES.ORDER_TIMEOUT]({});
    assert.equal(published.length, 0);
  });
});
