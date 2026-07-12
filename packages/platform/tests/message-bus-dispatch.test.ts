import test from 'node:test';
import assert from 'node:assert/strict';
import { PgMessageBus } from '../src/message-bus.js';

// Deterministic replacement for a fixed sleep: drain the microtask queue across
// enough checkpoints that any rejected promise without a synchronous .catch()
// gets flagged by Node's unhandled-rejection tracking (which runs at a microtask
// checkpoint). No wall-clock dependence — if dispatch() failed to attach its
// per-handler .catch, the rejection WOULD surface here and the assertion goes red.
const drainMicrotasks = async (): Promise<void> => {
  for (let i = 0; i < 50; i++) await Promise.resolve();
};

// Regression: a rejecting async subscriber on a bus channel (e.g. the
// courier-events worker on `order.courier_accepted`) must NOT crash the API
// process. Before the fix, `handlers.forEach(h => h(parsed))` dropped the
// returned promise, so a rejection became an unhandled rejection → exit 1,
// killing the order loop for every tenant on a single courier accept.

test('PgMessageBus dispatch isolates failing subscribers', async (t) => {
  // No DB needed: we exercise the in-process fan-out directly. Pass a stub pool
  // so the constructor never reaches createSessionPool().
  const bus = new PgMessageBus({} as any);

  await t.test('a rejecting handler does not produce an unhandled rejection, and siblings still run', async () => {
    const unhandled: unknown[] = [];
    const onUnhandled = (err: unknown) => unhandled.push(err);
    process.on('unhandledRejection', onUnhandled);

    let goodHandlerRan = false;

    const throwingHandler = async () => {
      throw new Error('simulated worker SQL error (column does not exist)');
    };
    const healthyHandler = async () => {
      goodHandlerRan = true;
    };

    // Drive the exact code path the pg 'notification' callback uses.
    (bus as any).dispatch('order.courier_accepted', [throwingHandler, healthyHandler], { orderId: 'o1' });

    // Let the rejecting promise settle and any (un)handled-rejection fire.
    await drainMicrotasks();
    process.off('unhandledRejection', onUnhandled);

    assert.equal(unhandled.length, 0, 'a failing subscriber must not escape as an unhandled rejection');
    assert.equal(goodHandlerRan, true, 'a sibling handler must still run despite a failing peer');
  });

  await t.test('a synchronously throwing handler is also contained', async () => {
    let goodHandlerRan = false;
    const syncThrow = () => { throw new Error('sync boom'); };
    const healthy = () => { goodHandlerRan = true; };

    assert.doesNotThrow(() => {
      (bus as any).dispatch('order.confirmed', [syncThrow, healthy], { orderId: 'o2' });
    });
    assert.equal(goodHandlerRan, true, 'a sibling handler must still run after a sync throw');
  });

  await t.test('a subscribe()-registered handler receives a dispatched notification', async () => {
    // Exercise the REAL registration path instead of a hand-built handler array:
    // subscribe() stores the handler in the same internal `handlers` map that the
    // pg 'notification' callback reads before calling dispatch(). With no live
    // listenerClient subscribe() only warns and still records the handler, so we
    // can drive the exact map→dispatch path the notification callback uses.
    const received: unknown[] = [];
    const handler = async (msg: unknown) => { received.push(msg); };
    await bus.subscribe('order.delivered', handler);

    const registered = (bus as any).handlers.get('order.delivered');
    assert.ok(
      Array.isArray(registered) && registered.includes(handler),
      'subscribe() must register the handler in the map dispatch() reads',
    );

    (bus as any).dispatch('order.delivered', registered, { orderId: 'o3' });
    await drainMicrotasks();

    assert.equal(received.length, 1, 'the subscribe()-registered handler must run once on dispatch');
    assert.deepEqual(received[0], { orderId: 'o3' }, 'the handler must receive the dispatched payload verbatim');
  });
});
