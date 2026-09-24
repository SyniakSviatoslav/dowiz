// A guest's round in the room app, called for real. `node --test workers/api/public/room/*.test.mjs`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { guestWaiting, sittingHasGuest, guestPath, guestBody, renderGuestBar } from './guest.js';

const r = (placed_by, status) => ({ id: 'o/1', placed_by, status });
const ctx = caps => ({ S: { caps: new Set(caps) }, t: k => k });

test('only a guest round still PENDING waits for the room', () => {
  assert.equal(guestWaiting(r('guest', 'PENDING')), true);
  assert.equal(guestWaiting(r('guest', 'CONFIRMED')), false);
  assert.equal(guestWaiting(r('staff_1', 'PENDING')), false);
  assert.equal(sittingHasGuest({ rounds: [r('staff_1', 'CONFIRMED'), r('guest', 'PENDING')] }), true);
  assert.equal(sittingHasGuest({ rounds: [r('guest', 'CONFIRMED')] }), false);
});

test('the buttons show for a waiter and not for the kitchen', () => {
  assert.match(renderGuestBar(ctx(['take_orders']), r('guest', 'PENDING')), /guestConfirm/);
  assert.equal(renderGuestBar(ctx(['advance']), r('guest', 'PENDING')), '');
  assert.equal(renderGuestBar(ctx(['take_orders']), r('guest', 'CONFIRMED')), '');
});

test('the answer goes to the guest route with the venue and the action', () => {
  assert.equal(guestPath('o/1'), '/staff/orders/o%2F1/guest');
  assert.deepEqual(guestBody('v1', 'confirm'), { location_id: 'v1', action: 'confirm' });
});
