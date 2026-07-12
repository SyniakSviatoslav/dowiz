import test from 'node:test';
import assert from 'node:assert/strict';
import {
  haversineMeters,
  lerp,
  lerpLatLng,
  bearingDeg,
  emaNext,
  progressAlongRoute,
  polylineLengthMeters,
  etaSeconds,
  isOutOfOrder,
  shouldSnap,
  isArriving,
  ARRIVE_THRESHOLD_M,
  type LatLng,
} from '../../../packages/ui/src/lib/geo-anim.js';

const near = (a: number, b: number, tol: number, msg?: string) =>
  assert.ok(Math.abs(a - b) <= tol, `${msg ?? ''} expected ${a} ≈ ${b} (±${tol})`);

test('G2 geo-anim pure math', async (t) => {
  await t.test('haversineMeters: ~1 deg lat ≈ 111 km', () => {
    near(haversineMeters({ lat: 41, lng: 19 }, { lat: 42, lng: 19 }), 111_195, 500);
    assert.equal(haversineMeters({ lat: 41, lng: 19 }, { lat: 41, lng: 19 }), 0);
  });

  await t.test('lerp + lerpLatLng: midpoint, clamped', () => {
    assert.equal(lerp(0, 10, 0.5), 5);
    assert.deepEqual(lerpLatLng({ lat: 0, lng: 0 }, { lat: 10, lng: 20 }, 0.5), { lat: 5, lng: 10 });
    // clamps out-of-range t (no extrapolation)
    assert.deepEqual(lerpLatLng({ lat: 0, lng: 0 }, { lat: 10, lng: 10 }, 2), { lat: 10, lng: 10 });
  });

  await t.test('bearingDeg: cardinal directions', () => {
    near(bearingDeg({ lat: 0, lng: 0 }, { lat: 1, lng: 0 }), 0, 0.5, 'north');
    near(bearingDeg({ lat: 0, lng: 0 }, { lat: 0, lng: 1 }), 90, 0.5, 'east');
    near(bearingDeg({ lat: 0, lng: 0 }, { lat: -1, lng: 0 }), 180, 0.5, 'south');
    near(bearingDeg({ lat: 0, lng: 0 }, { lat: 0, lng: -1 }), 270, 0.5, 'west');
  });

  await t.test('emaNext: seeds on null, smooths thereafter', () => {
    assert.equal(emaNext(null, 100, 0.3), 100);
    assert.equal(emaNext(100, 200, 0.5), 150);
    // a single spike moves the average only partway (anti-jitter)
    assert.equal(emaNext(100, 200, 0.2), 120);
  });

  await t.test('progressAlongRoute: straight line — start/mid/end remaining', () => {
    const line: LatLng[] = [{ lat: 41.30, lng: 19.80 }, { lat: 41.40, lng: 19.80 }];
    const total = polylineLengthMeters(line);
    near(progressAlongRoute(line, line[0]!).remainingMeters, total, 1, 'at start → full length');
    near(progressAlongRoute(line, { lat: 41.35, lng: 19.80 }).remainingMeters, total / 2, total * 0.02, 'midpoint → half');
    near(progressAlongRoute(line, line[1]!).remainingMeters, 0, 1, 'at end → ~0');
  });

  await t.test('progressAlongRoute: off-route point snaps onto the line', () => {
    const line: LatLng[] = [{ lat: 41.30, lng: 19.80 }, { lat: 41.40, lng: 19.80 }];
    // ~200m east of the midpoint
    const off = { lat: 41.35, lng: 19.8024 };
    const p = progressAlongRoute(line, off);
    near(p.snapped.lng, 19.80, 0.0005, 'snapped back onto the corridor');
    near(p.snapped.lat, 41.35, 0.001);
  });

  await t.test('progressAlongRoute: multi-segment remaining decreases monotonically', () => {
    const line: LatLng[] = [
      { lat: 41.30, lng: 19.80 },
      { lat: 41.34, lng: 19.82 },
      { lat: 41.38, lng: 19.80 },
    ];
    const atStart = progressAlongRoute(line, line[0]!).remainingMeters;
    const atKnee = progressAlongRoute(line, line[1]!).remainingMeters;
    const atEnd = progressAlongRoute(line, line[2]!).remainingMeters;
    assert.ok(atStart > atKnee && atKnee > atEnd, 'remaining shrinks along the route');
    near(atEnd, 0, 1);
  });

  await t.test('etaSeconds: paces by route average speed; fallback when degenerate', () => {
    // 1000 m remaining of a 2000 m / 400 s route → 5 m/s → 200 s
    assert.equal(etaSeconds(1000, 2000, 400), 200);
    // degenerate baseline → 18 km/h fallback (5 m/s) → 1000 m → 200 s
    assert.equal(etaSeconds(1000, 0, 0), 200);
  });

  await t.test('isOutOfOrder: only a strictly-older ts is rejected', () => {
    assert.equal(isOutOfOrder(null, 100), false);
    assert.equal(isOutOfOrder(100, 90), true);
    assert.equal(isOutOfOrder(100, 100), false);
    assert.equal(isOutOfOrder(100, 110), false);
  });

  await t.test('shouldSnap: null prev or big jump → snap; small move → tween', () => {
    assert.equal(shouldSnap(null, { lat: 41.3, lng: 19.8 }), true);
    assert.equal(shouldSnap({ lat: 41.3000, lng: 19.8000 }, { lat: 41.3003, lng: 19.8000 }), false); // ~33m
    assert.equal(shouldSnap({ lat: 41.3, lng: 19.8 }, { lat: 41.31, lng: 19.8 }), true); // ~1.1km
  });

  await t.test('isArriving: flips at the 150m proximity threshold', () => {
    assert.equal(ARRIVE_THRESHOLD_M, 150);
    assert.equal(isArriving(151), false);
    assert.equal(isArriving(150), true);
    assert.equal(isArriving(10), true);
  });
});
