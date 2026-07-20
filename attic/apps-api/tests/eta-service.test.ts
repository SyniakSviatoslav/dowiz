import { test, describe } from 'node:test';
import assert from 'node:assert';
import { computeEtaRange, deliveryLegMinutes, ETA_DEFAULTS, type EtaInput } from '../src/lib/etaService.js';

// Base happy-path input; individual tests override fields.
function base(over: Partial<EtaInput> = {}): EtaInput {
  return {
    phase: 'pre_assign',
    status: 'PREPARING',
    prepRemainingMinutes: 15,
    kitchenQueueAheadMinutes: 0,
    courierQueueAheadMinutes: 0,
    deliveryLegMinutes: 10,
    ...over,
  };
}

describe('computeEtaRange — core invariants', () => {
  test('always a range, never a single number (low < high)', () => {
    const r = computeEtaRange(base());
    assert.ok(r.lowMin < r.highMin, `expected low<high, got ${r.lowMin}-${r.highMin}`);
    assert.ok(r.highMin - r.lowMin >= ETA_DEFAULTS.minBandMin);
  });

  test('never 0 / never below the low floor, even with zero everything', () => {
    const r = computeEtaRange(base({ prepRemainingMinutes: 0, deliveryLegMinutes: 0, status: 'READY' }));
    assert.ok(r.lowMin >= ETA_DEFAULTS.minLowFloor, `low ${r.lowMin} must be ≥ ${ETA_DEFAULTS.minLowFloor}`);
    assert.ok(r.lowMin > 0 && r.highMin > 0);
  });

  test('Phase 1 (pre_assign) is WIDER than Phase 2 (assigned) for the same core', () => {
    const p1 = computeEtaRange(base({ phase: 'pre_assign' }));
    const p2 = computeEtaRange(base({ phase: 'assigned' }));
    assert.ok((p1.highMin - p1.lowMin) > (p2.highMin - p2.lowMin), 'pre_assign band should exceed assigned band');
  });

  test('deeper courier queue → higher estimate (width reflects queue depth)', () => {
    const free = computeEtaRange(base({ phase: 'assigned', courierQueueAheadMinutes: 0 }));
    const busy = computeEtaRange(base({ phase: 'assigned', courierQueueAheadMinutes: 40 }));
    assert.ok(busy.highMin > free.highMin, 'busy courier must push the estimate up');
    assert.ok(busy.lowMin > free.lowMin);
  });

  test('near-end: IN_DELIVERY with a tiny core → calm floor band, never 0', () => {
    const r = computeEtaRange(base({ phase: 'assigned', status: 'IN_DELIVERY', prepRemainingMinutes: 0, courierQueueAheadMinutes: 0, deliveryLegMinutes: 1 }));
    assert.equal(r.lowMin, ETA_DEFAULTS.nearEndLow);
    assert.equal(r.highMin, ETA_DEFAULTS.nearEndHigh);
  });

  test('monotonic: a modest Phase-2 re-estimate is absorbed (top does not jitter up)', () => {
    const prev = computeEtaRange(base({ phase: 'assigned', courierQueueAheadMinutes: 10 }));
    // A small upward nudge whose new low still sits under the old high → clamp keeps the top steady.
    const next = computeEtaRange(base({ phase: 'assigned', courierQueueAheadMinutes: 13, previousHighMin: prev.highMin }));
    assert.ok(next.highMin <= prev.highMin, `top should not exceed previous ${prev.highMin}, got ${next.highMin}`);
    assert.ok(next.highMin >= next.lowMin + ETA_DEFAULTS.minBandMin, 'band invariant still holds');
  });

  test('genuine large degradation (new low exceeds old high) is allowed to grow — honesty over a frozen lie', () => {
    const prev = computeEtaRange(base({ phase: 'assigned', courierQueueAheadMinutes: 0 }));
    const next = computeEtaRange(base({ phase: 'assigned', courierQueueAheadMinutes: 60, previousHighMin: prev.highMin }));
    assert.ok(next.lowMin < next.highMin, 'band invariant holds even when growth is real');
    assert.ok(next.highMin > prev.highMin, 'a real +60min queue must surface, not be clamped to a lie');
  });
});

describe('computeEtaRange — degenerate inputs never leak NaN/0/single/low>high', () => {
  for (const bad of [NaN, -5, Infinity, undefined as unknown as number, null as unknown as number]) {
    test(`prep=${String(bad)} stays valid`, () => {
      const r = computeEtaRange(base({ prepRemainingMinutes: bad }));
      assert.ok(Number.isFinite(r.lowMin) && Number.isFinite(r.highMin));
      assert.ok(r.lowMin >= ETA_DEFAULTS.minLowFloor && r.lowMin < r.highMin);
    });
  }

  test('null delivery leg (no customer pin) → fallback used, not NaN', () => {
    const r = computeEtaRange(base({ deliveryLegMinutes: null }));
    assert.ok(Number.isFinite(r.lowMin) && Number.isFinite(r.highMin));
    assert.ok(r.highMin > r.lowMin);
  });

  test('NaN delivery leg → fallback used', () => {
    const r = computeEtaRange(base({ deliveryLegMinutes: NaN }));
    assert.ok(Number.isFinite(r.highMin) && r.highMin > r.lowMin);
  });
});

describe('computeEtaRange — overdue (D3)', () => {
  test('elapsed beyond the high bound, not terminal → overdue', () => {
    const r = computeEtaRange(base({ phase: 'assigned', status: 'IN_DELIVERY', elapsedSincePlacedMinutes: 999 }));
    assert.equal(r.overdue, true);
  });
  test('terminal status is never overdue', () => {
    const r = computeEtaRange(base({ status: 'DELIVERED', elapsedSincePlacedMinutes: 999 }));
    assert.equal(r.overdue, false);
  });
  test('within the estimate → not overdue', () => {
    const r = computeEtaRange(base({ elapsedSincePlacedMinutes: 1 }));
    assert.equal(r.overdue, false);
  });
});

describe('deliveryLegMinutes', () => {
  test('returns null when any coordinate is missing/non-finite', () => {
    assert.equal(deliveryLegMinutes(null, 19.4, 41.3, 19.5), null);
    assert.equal(deliveryLegMinutes(41.3, 19.4, NaN, 19.5), null);
    assert.equal(deliveryLegMinutes(undefined, undefined, undefined, undefined), null);
  });
  test('returns a finite positive minute estimate for real coords (road_factor applied)', () => {
    const min = deliveryLegMinutes(41.31, 19.44, 41.33, 19.46);
    assert.ok(typeof min === 'number' && Number.isFinite(min) && min >= 0);
  });
  test('same point → ~0 minutes (not negative, not NaN)', () => {
    const min = deliveryLegMinutes(41.31, 19.44, 41.31, 19.44);
    assert.ok(min === 0 || (typeof min === 'number' && min < 0.001));
  });
});
