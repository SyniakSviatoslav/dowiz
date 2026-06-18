import test from 'node:test';
import assert from 'node:assert/strict';
import { PgMessageBus } from '../src/message-bus.js';

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
    await new Promise((r) => setTimeout(r, 50));
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
});
