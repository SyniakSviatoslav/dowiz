import './_env-stub.js';
import test from 'node:test';
import assert from 'node:assert/strict';
import {
  OrsRoutingProvider,
  createRoutingProvider,
  haversineRoute,
  haversineMeters,
  type LatLng,
  type RouteResult,
  type RoutingProvider,
} from '@deliveryos/platform';
import { RoutingService, deviationMeters, shouldReroute } from '../src/lib/routing.js';

const FROM: LatLng = { lat: 41.3275, lng: 19.8187 };
const TO: LatLng = { lat: 41.3375, lng: 19.8287 };

// A realistic ORS directions-geojson body (3-vertex road geometry).
const okGeo = {
  features: [
    {
      geometry: { coordinates: [[19.8187, 41.3275], [19.823, 41.332], [19.8287, 41.3375]] },
      properties: { summary: { distance: 1500, duration: 300 } },
    },
  ],
};

function fakeResponse(opts: { ok: boolean; status: number; json?: unknown; remaining?: string }): Response {
  return {
    ok: opts.ok,
    status: opts.status,
    headers: { get: (h: string) => (h.toLowerCase() === 'x-ratelimit-remaining' ? opts.remaining ?? null : null) },
    json: async () => opts.json,
  } as unknown as Response;
}

const silent = () => {};

test('G1 RoutingProvider + service', async (t) => {
  await t.test('ORS happy path → road polyline parsed, provider=ors', async () => {
    let calls = 0;
    const p = new OrsRoutingProvider({
      provider: 'ors',
      baseUrl: 'https://routing.test',
      apiKey: 'k',
      warn: silent,
      fetchImpl: (async () => { calls++; return fakeResponse({ ok: true, status: 200, json: okGeo, remaining: '900' }); }) as any,
    });
    const r = await p.route(FROM, TO);
    assert.equal(r.provider, 'ors');
    assert.equal(r.polyline.length, 3, 'full road geometry, not a straight line');
    assert.deepEqual(r.polyline[0], { lat: 41.3275, lng: 19.8187 });
    assert.equal(r.distance_m, 1500);
    assert.equal(r.duration_s, 300);
    assert.equal(calls, 1, 'exactly one provider call');
  });

  await t.test('forced failure per code → silent haversine, never throws', async () => {
    for (const mode of ['429', '403', 'timeout', 'malformed'] as const) {
      const p = new OrsRoutingProvider({
        provider: 'ors',
        baseUrl: 'https://routing.test',
        warn: silent,
        fetchImpl: (async () => {
          if (mode === 'timeout') throw new DOMException('timeout', 'TimeoutError');
          if (mode === 'malformed') return fakeResponse({ ok: true, status: 200, json: { features: [] } });
          return fakeResponse({ ok: false, status: Number(mode) });
        }) as any,
      });
      const r = await p.route(FROM, TO);
      assert.equal(r.provider, 'haversine', `${mode} must degrade to haversine`);
      assert.equal(r.polyline.length, 2, `${mode} fallback is a straight line`);
      assert.ok(r.duration_s > 0 && r.distance_m > 0);
    }
  });

  await t.test('circuit breaker opens after threshold, then skips the provider; half-open after cooldown', async () => {
    let calls = 0;
    let clock = 1_000_000;
    const p = new OrsRoutingProvider({
      provider: 'ors',
      baseUrl: 'https://routing.test',
      warn: silent,
      now: () => clock,
      tunables: { breakerFailureThreshold: 3, breakerCooldownMs: 30_000 },
      fetchImpl: (async () => { calls++; throw new Error('down'); }) as any,
    });
    // 3 failures open the breaker (each still hits the provider once).
    for (let i = 0; i < 3; i++) await p.route(FROM, TO);
    assert.equal(calls, 3);
    // Breaker now open → next calls short-circuit to haversine WITHOUT a provider call.
    await p.route(FROM, TO);
    await p.route(FROM, TO);
    assert.equal(calls, 3, 'breaker open: provider not called');
    // Advance past cooldown → half-open trial hits the provider again.
    clock += 30_001;
    await p.route(FROM, TO);
    assert.equal(calls, 4, 'half-open: one trial call after cooldown');
  });

  await t.test('RoutingService cache: identical leg hits provider once; new leg re-fetches', async () => {
    let calls = 0;
    const provider: RoutingProvider = { async route(from, to) { calls++; return haversineRoute(from, to); } };
    const svc = new RoutingService(provider);
    await svc.getLegRoute(FROM, TO);
    await svc.getLegRoute(FROM, TO);
    await svc.getLegRoute({ lat: 41.3275, lng: 19.81871 }, TO); // ~1m apart → same rounded key
    assert.equal(calls, 1, 'identical (rounded) leg served from cache');
    await svc.getLegRoute(FROM, { lat: 41.4, lng: 19.9 }); // genuinely different leg
    assert.equal(calls, 2);
  });

  await t.test('cache is non-authoritative: a fresh service (restart) re-fetches', async () => {
    let calls = 0;
    const provider: RoutingProvider = { async route(from, to) { calls++; return haversineRoute(from, to); } };
    const svc1 = new RoutingService(provider);
    await svc1.getLegRoute(FROM, TO);
    assert.equal(svc1.cacheSize, 1);
    // Simulate process kill → new instance, empty cache, must re-fetch.
    const svc2 = new RoutingService(provider);
    assert.equal(svc2.cacheSize, 0);
    await svc2.getLegRoute(FROM, TO);
    assert.equal(calls, 2, 'no authority in memory: re-fetch after restart');
  });

  await t.test('cache TTL expiry re-fetches', async () => {
    let calls = 0;
    let clock = 0;
    const provider: RoutingProvider = { async route(from, to) { calls++; return haversineRoute(from, to); } };
    const svc = new RoutingService(provider, () => clock, 1000);
    await svc.getLegRoute(FROM, TO);
    clock = 1500; // past TTL
    await svc.getLegRoute(FROM, TO);
    assert.equal(calls, 2);
  });

  await t.test('re-route geometry: on-route → no reroute; strayed > threshold → one reroute', () => {
    const route = await_polyline();
    // A point sitting on the polyline → ~0 deviation.
    assert.ok(deviationMeters(route, route[1]) < 1);
    assert.equal(shouldReroute(route, route[1], 300), false);

    // Walking the whole route (interpolated) never triggers a reroute.
    let reroutes = 0;
    for (let i = 0; i < route.length - 1; i++) {
      const mid = { lat: (route[i].lat + route[i + 1].lat) / 2, lng: (route[i].lng + route[i + 1].lng) / 2 };
      if (shouldReroute(route, mid, 300)) reroutes++;
    }
    assert.equal(reroutes, 0, 'positions along the route cause zero reroutes (per-leg, not per-ping)');

    // A point ~1 km off the corridor → exactly one reroute decision.
    const offRoute = { lat: 41.345, lng: 19.80 };
    assert.ok(deviationMeters(route, offRoute) > 300);
    assert.equal(shouldReroute(route, offRoute, 300), true);
  });

  await t.test('haversineRoute math: sinuosity + urban speed, straight polyline', () => {
    const r = haversineRoute(FROM, TO);
    const straight = haversineMeters(FROM, TO);
    assert.equal(r.provider, 'haversine');
    assert.deepEqual(r.polyline, [FROM, TO]);
    assert.equal(r.distance_m, Math.round(straight * 1.3));
    assert.equal(r.duration_s, Math.round(r.distance_m / ((18 * 1000) / 3600)));
  });

  await t.test('factory: haversine kind never calls a provider; self kind tags self', async () => {
    const hav = createRoutingProvider({ ROUTING_PROVIDER: 'haversine', ROUTING_BASE_URL: 'https://x' });
    assert.equal((await hav.route(FROM, TO)).provider, 'haversine');

    const self = createRoutingProvider(
      { ROUTING_PROVIDER: 'self', ROUTING_BASE_URL: 'https://internal.fra' },
      { warn: silent, fetchImpl: (async () => fakeResponse({ ok: true, status: 200, json: okGeo })) as any },
    );
    assert.equal((await self.route(FROM, TO)).provider, 'self');
  });
});

// helper kept below to avoid await-in-non-async lint noise above
function await_polyline(): LatLng[] {
  return okGeo.features[0].geometry.coordinates.map(([lng, lat]) => ({ lat, lng }));
}
