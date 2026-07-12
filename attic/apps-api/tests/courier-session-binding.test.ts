import test from 'node:test';
import assert from 'node:assert/strict';
import { courierSessionValid, type CourierSessionRow } from '../src/plugins/auth.js';

// H2+H4 regression: a signed courier JWT must additionally map to a live,
// non-revoked session whose courier still holds membership in the token's
// location. Revocation (logout / password-change / refresh-rotation) and
// location removal must take effect immediately, not at JWT expiry.

const NOW = Date.parse('2026-06-18T12:00:00Z');
const future = '2026-06-19T12:00:00Z';
const past = '2026-06-17T12:00:00Z';

const ok: CourierSessionRow = { courier_id: 'c1', revoked_at: null, expires_at: future, has_location: true };

test('courierSessionValid', async (t) => {
  await t.test('accepts a live, non-revoked, in-location session', () => {
    assert.equal(courierSessionValid(ok, NOW), true);
  });

  await t.test('rejects when no session row exists (revoked+deleted / unknown jti)', () => {
    assert.equal(courierSessionValid(null, NOW), false);
    assert.equal(courierSessionValid(undefined, NOW), false);
  });

  await t.test('rejects a revoked session (logout / rotation)', () => {
    assert.equal(courierSessionValid({ ...ok, revoked_at: past }, NOW), false);
  });

  await t.test('rejects an expired session', () => {
    assert.equal(courierSessionValid({ ...ok, expires_at: past }, NOW), false);
  });

  await t.test('rejects when courier lost membership in the token location (H4)', () => {
    assert.equal(courierSessionValid({ ...ok, has_location: false }, NOW), false);
  });
});
