import './_env-stub.js';
import test from 'node:test';
import assert from 'node:assert/strict';
import { haversineRoute } from '@deliveryos/platform';
import { saveRoute, loadRoute, claimOnce, closeRouteRedis } from '../src/lib/routing.js';

// Integration test for the N-safe route state layer (real Redis from .env).
// Proves: RouteResult survives a round-trip out of memory, unknown orders read as
// null, and the NX claim grants exactly one winner (the cross-instance dedup that
// keeps the route push "exactly once").

test('G1b — Redis route store (N-safe state)', async (t) => {
  // Unique per run; no Math.random needed — timestamp + pid suffice.
  const orderId = `g1test-${Date.now()}-${process.pid}`;
  t.after(async () => { await closeRouteRedis(); });

  await t.test('save → load round-trips the RouteResult', async () => {
    const r = haversineRoute({ lat: 41.0, lng: 19.0 }, { lat: 41.1, lng: 19.1 });
    await saveRoute(orderId, r);
    const back = await loadRoute(orderId);
    assert.ok(back, 'route reloads from Redis');
    assert.equal(back!.provider, 'haversine');
    assert.equal(back!.polyline.length, 2);
    assert.equal(back!.distance_m, r.distance_m);
    assert.equal(back!.duration_s, r.duration_s);
  });

  await t.test('unknown order → null (no throw)', async () => {
    assert.equal(await loadRoute(`absent-${orderId}`), null);
  });

  await t.test('claimOnce: first instance wins, second loses within the window', async () => {
    const key = `g1claim-${orderId}`;
    assert.equal(await claimOnce(key, 60), true, 'first claim wins');
    assert.equal(await claimOnce(key, 60), false, 'second claim (other instance) loses');
  });
});
