import './_env-stub.js';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { BUS_CHANNELS } from '../src/lib/registry.js';

// F1 (offer-sweep-cancel addendum): a grace-cancel publishes BUS_CHANNELS.ORDER_CANCELLED post-commit.
// LifecycleHandlers subscribes to it → resolves the order's open dwell alerts (app_resolve_order_alerts)
// AND boss.cancel's the pending notify.dispatch.<alertId> escalation jobs. Without this, a grace-cancelled
// order keeps open dwell alerts and fires a contradictory escalation after the cancel. This test drives the
// exact wiring: register subscribers via start(), then publish ORDER_CANCELLED and assert the fan-out.

function makeRecordingBus() {
  const subs = new Map<string, (msg: any) => Promise<void>>();
  const published: Array<{ channel: string; payload: any }> = [];
  return {
    bus: {
      subscribe: (channel: string, handler: any) => { subs.set(channel, handler); },
      publish: async (channel: string, payload: any) => {
        published.push({ channel, payload });
        const h = subs.get(channel);
        if (h) await h(payload);
      },
    } as any,
    subs,
    published,
  };
}

async function loadLifecycle() {
  const { LifecycleHandlers } = await import('../src/workers/lifecycle-handlers.js');
  return LifecycleHandlers as any;
}

test('F1 — ORDER_CANCELLED drives dwell-alert resolution + escalation-job cancel', async () => {
  const LifecycleHandlers = await loadLifecycle();
  const cancels: string[] = [];
  const boss = { cancel: async (jobKey: string) => { cancels.push(jobKey); } };
  const resolveCalls: any[][] = [];
  const client = {
    query: async (sql: string, params: any[] = []) => {
      if (/app_resolve_order_alerts/.test(sql)) {
        resolveCalls.push(params);
        // Resolve one alert on the first kind only (dwell_preparing for a PREPARING order).
        if (params[2] === 'dwell_preparing') return { rows: [{ id: 'alert-77' }], rowCount: 1 };
        return { rows: [], rowCount: 0 };
      }
      return { rows: [], rowCount: 0 };
    },
    release() {},
  };
  const pool = { connect: async () => client } as any;
  const { bus, subs, published } = makeRecordingBus();

  const handlers = new LifecycleHandlers(pool, boss as any, bus);
  await handlers.start();

  // Wiring proof: the addendum's canonical topic is subscribed.
  assert.ok(subs.has(BUS_CHANNELS.ORDER_CANCELLED), 'LifecycleHandlers subscribes ORDER_CANCELLED');
  assert.equal(BUS_CHANNELS.ORDER_CANCELLED, 'order.cancelled');

  // Simulate the worker's post-commit publish.
  await bus.publish(BUS_CHANNELS.ORDER_CANCELLED, { orderId: 'o9', locationId: 'l1', reason: 'dispatch_exhausted' });

  // The resolve fn was invoked for the CANCELLED kind set (incl. dwell_preparing).
  assert.ok(resolveCalls.some((p) => p[1] === 'o9' && p[2] === 'dwell_preparing'), 'app_resolve_order_alerts called for the order');
  // The pending escalation job for the resolved alert was cancelled.
  assert.ok(cancels.includes('notify.dispatch.alert-77'), 'boss.cancel(notify.dispatch.<alertId>) fired');
  // A DWELL_ALERT_RESOLVED signal was emitted.
  assert.ok(published.some((e) => e.channel === BUS_CHANNELS.DWELL_ALERT_RESOLVED && e.payload.alertId === 'alert-77'), 'DWELL_ALERT_RESOLVED published');
});
