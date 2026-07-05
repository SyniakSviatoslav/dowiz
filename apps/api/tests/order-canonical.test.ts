import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildRequestHash, buildSignalState } from '../src/lib/order-canonical.js';

// Guardrail for the canonicalization helpers extracted from POST /orders.
// buildRequestHash is the idempotency fingerprint: the golden hash below pins it
// byte-for-byte so any drift (key order, pin rounding, cash_pay_with falsy→null)
// that would break in-flight idempotency retries fails RED.

const GOLDEN_HASH = 'e18727ed9730e9618e18a9897b43c2d1f18ccf1fbacb26f21272f5b461a598a7';

test('buildRequestHash — golden fingerprint (idempotency continuity anchor)', () => {
  const hash = buildRequestHash({
    locationId: 'loc-1',
    type: 'delivery',
    items: [
      { product_id: 'p2', quantity: 1, modifier_ids: ['mb', 'ma'] },
      { product_id: 'p1', quantity: 2 },
    ],
    pin: { lat: 41.123456789, lng: 19.987654321 },
    addressText: 'Main St',
    cashPayWith: 0, // falsy → serialized as null (matches `cashPayWith || null`)
    currencyCode: 'ALL',
    menuVersion: '7',
    customerId: 'anonymous',
  });
  assert.equal(hash, GOLDEN_HASH);
});

test('buildRequestHash — modifier order does not change the hash (sorted)', () => {
  const base = {
    locationId: 'l',
    type: 'pickup',
    pin: null,
    addressText: null,
    cashPayWith: undefined,
    currencyCode: 'ALL',
    menuVersion: '1',
    customerId: 'anonymous',
  };
  const a = buildRequestHash({ ...base, items: [{ product_id: 'p', quantity: 1, modifier_ids: ['x', 'y'] }] });
  const b = buildRequestHash({ ...base, items: [{ product_id: 'p', quantity: 1, modifier_ids: ['y', 'x'] }] });
  assert.equal(a, b);
});

test('buildRequestHash — pin difference past 5dp does not change hash; before it does', () => {
  const base = {
    locationId: 'l',
    type: 'delivery',
    items: [{ product_id: 'p', quantity: 1 }],
    addressText: null,
    cashPayWith: undefined,
    currencyCode: 'ALL',
    menuVersion: '1',
    customerId: 'anonymous',
  };
  const h1 = buildRequestHash({ ...base, pin: { lat: 41.1234561, lng: 19.0 } });
  const h2 = buildRequestHash({ ...base, pin: { lat: 41.1234569, lng: 19.0 } }); // same to 5dp
  const h3 = buildRequestHash({ ...base, pin: { lat: 41.12350, lng: 19.0 } }); // differs at 5dp
  assert.equal(h1, h2);
  assert.notEqual(h1, h3);
});

test('buildSignalState — defaults when no signals', () => {
  const s = buildSignalState({ signals: [], otpRequired: true, otpVerified: false });
  assert.deepEqual(s, {
    velocityPhoneCount: 0,
    velocityIpCount: 0,
    noShowCount: 0,
    noShowAgeDays: null,
    completedCount: 0,
    otpRequired: true,
    otpVerified: false,
  });
});

test('buildSignalState — velocity takes the max across matching windows', () => {
  const s = buildSignalState({
    signals: [
      { kind: 'velocity_rapid', evidence: { count: 3 } } as any,
      { kind: 'velocity_high_volume', evidence: { count: 7 } } as any,
      { kind: 'ip_velocity_rapid', evidence: { count: 2 } } as any,
    ],
    otpRequired: false,
    otpVerified: false,
  });
  assert.equal(s.velocityPhoneCount, 7);
  assert.equal(s.velocityIpCount, 2);
});

test('buildSignalState — no_show pulls count/age/completed from evidence', () => {
  const s = buildSignalState({
    signals: [{ kind: 'no_show_recent', evidence: { count: 2, ageDays: 5, completedCount: 9 } } as any],
    otpRequired: false,
    otpVerified: true,
  });
  assert.equal(s.noShowCount, 2);
  assert.equal(s.noShowAgeDays, 5);
  assert.equal(s.completedCount, 9);
});
