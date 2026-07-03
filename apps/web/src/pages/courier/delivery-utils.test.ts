import test from 'node:test';
import assert from 'node:assert/strict';
import { computeDestinationRoute } from './delivery-utils.js';

const COURIER_POS: [number, number] = [19.9, 41.4];

test('computeDestinationRoute (LC9/S3 — no fabricated Tirana/Durrës fallback)', async (t) => {
  await t.test('missing customer coords → no pin, no route (was: hardcoded Durrës mock)', () => {
    const result = computeDestinationRoute(
      { customer: { address: 'x' }, restaurant: { name: 'r', address: 'y', lat: 41.328, lng: 19.812 } } as any,
      COURIER_POS,
    );
    assert.equal(result.hasCustomerCoords, false);
    assert.equal(result.destPin, undefined);
    assert.equal(result.routeLine, undefined);
  });

  await t.test('missing task entirely → no pin, no route', () => {
    const result = computeDestinationRoute(null, COURIER_POS);
    assert.equal(result.destPin, undefined);
    assert.equal(result.routeLine, undefined);
  });

  await t.test('real customer coords → real pin, route includes courier + destination', () => {
    const result = computeDestinationRoute(
      { customer: { address: 'x', lat: 41.337, lng: 19.825 }, restaurant: { name: 'r', address: 'y', lat: 41.328, lng: 19.812 } } as any,
      COURIER_POS,
    );
    assert.equal(result.hasCustomerCoords, true);
    assert.deepEqual(result.destPin, [19.825, 41.337]);
    assert.deepEqual(result.routeLine, [COURIER_POS, [19.812, 41.328], [19.825, 41.337]]);
  });

  await t.test('restaurant coords missing but customer present → route skips restaurant leg, still real', () => {
    const result = computeDestinationRoute(
      { customer: { address: 'x', lat: 41.337, lng: 19.825 }, restaurant: { name: 'r', address: 'y' } } as any,
      COURIER_POS,
    );
    assert.equal(result.hasRestaurantCoords, false);
    assert.deepEqual(result.routeLine, [COURIER_POS, [19.825, 41.337]]);
  });

  await t.test('legitimate 0,0 coords are NOT dropped (the old `||` bug)', () => {
    const result = computeDestinationRoute(
      { customer: { address: 'equator', lat: 0, lng: 0 }, restaurant: { name: 'r', address: 'y', lat: 0, lng: 0 } } as any,
      COURIER_POS,
    );
    assert.equal(result.hasCustomerCoords, true, '0 is a real coordinate, not "missing"');
    assert.deepEqual(result.destPin, [0, 0]);
  });
});
