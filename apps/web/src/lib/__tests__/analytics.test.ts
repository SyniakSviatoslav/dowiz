import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { FUNNEL_EVENTS, FUNNEL_FIRING_SURFACE, buildEvent } from '../analytics.js';

describe('funnel analytics taxonomy', () => {
  it('defines exactly the 7 funnel events, in funnel order', () => {
    assert.deepEqual(
      [...FUNNEL_EVENTS],
      ['menu_view', 'item_add', 'cart_open', 'checkout_start', 'order_placed', 'courier_assigned', 'delivered'],
    );
  });

  it('has no duplicate event names', () => {
    assert.equal(new Set(FUNNEL_EVENTS).size, FUNNEL_EVENTS.length);
  });

  it('documents a firing surface for every event and nothing extra', () => {
    for (const e of FUNNEL_EVENTS) {
      assert.ok(FUNNEL_FIRING_SURFACE[e]?.length > 0, `missing firing surface for ${e}`);
    }
    assert.equal(Object.keys(FUNNEL_FIRING_SURFACE).length, FUNNEL_EVENTS.length);
  });

  it('buildEvent returns a typed { event, properties } envelope', () => {
    const ev = buildEvent('order_placed', {
      slug: 'pizza-place',
      orderId: 'o-1',
      locationId: 'loc-1',
      total: 1500,
      itemCount: 2,
    });
    assert.equal(ev.event, 'order_placed');
    assert.equal(ev.properties.orderId, 'o-1');
    assert.equal(ev.properties.total, 1500);
  });
});
