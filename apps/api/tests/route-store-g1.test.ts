import test from 'node:test';
import assert from 'node:assert/strict';
import Redis from 'ioredis';
import { loadEnv } from '@deliveryos/config';
import { haversineRoute } from '@deliveryos/platform';
import { saveRoute, loadRoute, claimOnce, closeRouteRedis } from '../src/lib/routing.js';

// Same EX value saveRoute writes (routing.ts:135). A round 2h; the TTL probe below
// only needs an upper bound to prove the EX argument is present (key not persistent).
const ROUTE_TTL_S = 2 * 60 * 60;

// Integration test for the N-safe route state layer (real Redis from .env).
// Proves: RouteResult survives a round-trip out of memory, unknown orders read as
// null, and the NX claim grants exactly one winner (the cross-instance dedup that
// keeps the route push "exactly once").

test('G1b — Redis route store (N-safe state)', async (t) => {
  // Unique per run; no Math.random needed — timestamp + pid suffice.
  const orderId = `g1test-${Date.now()}-${process.pid}`;
  // Independent probe connection to inspect the key's TTL (saveRoute owns no getter).
  const probe = new Redis(loadEnv().REDIS_URL, { maxRetriesPerRequest: 2 });
  t.after(async () => {
    // Best-effort teardown: a quit on an already-dropped connection is irrelevant to the test outcome.
    try { await probe.quit(); } catch { /* connection already closed — nothing to clean up */ }
    await closeRouteRedis();
  });

  await t.test('save → load round-trips the RouteResult', async () => {
    const r = haversineRoute({ lat: 41.0, lng: 19.0 }, { lat: 41.1, lng: 19.1 });
    await saveRoute(orderId, r);
    const back = await loadRoute(orderId);
    assert.ok(back, 'route reloads from Redis');
    assert.equal(back!.provider, 'haversine');
    assert.equal(back!.polyline.length, 2);
    // Coordinate fidelity: length alone hides a lat/lng-corrupting serialization bug.
    assert.deepEqual(back!.polyline, r.polyline);
    assert.equal(back!.distance_m, r.distance_m);
    assert.equal(back!.duration_s, r.duration_s);

    // TTL must be SET (EX argument present): -1 = persistent (leaks forever),
    // -2 = missing. Must be a positive value no greater than the configured window.
    const ttl = await probe.ttl(`route:${orderId}`);
    assert.ok(ttl > 0 && ttl <= ROUTE_TTL_S, `route TTL set within window (got ${ttl})`);
  });

  await t.test('unknown order → null (no throw)', async () => {
    assert.equal(await loadRoute(`absent-${orderId}`), null);
  });

  await t.test('claimOnce: first instance wins, second loses within the window', async () => {
    const key = `g1claim-${orderId}`;
    assert.equal(await claimOnce(key, 60), true, 'first claim wins');
    assert.equal(await claimOnce(key, 60), false, 'second claim (other instance) loses');
  });

  await t.test('claimOnce: exactly one winner under concurrent contention', async () => {
    // The production contract is "exactly one winner across N instances". A serial
    // call pair cannot catch a mis-keyed/non-atomic SET NX that lets two racing
    // callers both win. Fire N claims concurrently on a fresh key and assert the
    // count of winners is precisely one (SET NX is atomic in Redis).
    const key = `g1race-${orderId}`;
    const N = 16;
    const results = await Promise.all(
      Array.from({ length: N }, () => claimOnce(key, 60)),
    );
    const winners = results.filter((won) => won === true).length;
    assert.equal(winners, 1, `exactly one concurrent winner (got ${winners})`);
  });
});
