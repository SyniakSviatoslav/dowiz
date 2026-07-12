import test from 'node:test';
import assert from 'node:assert/strict';
import { signAuthToken, verifyAuthToken } from '@deliveryos/platform';
import { refreshedOwnerClaims } from '../src/routes/auth.js';

// H5 regression: /auth/refresh must never mint anything but an owner token.
// Previously the role was inferred from a nullable users.google_sub column,
// which could escalate/flip a dual-identity user's role on refresh.

test('refresh always issues an owner token', async (t) => {
  const userId = '11111111-1111-1111-1111-111111111111';

  await t.test('claims builder is invariant to user identity', () => {
    assert.deepEqual(refreshedOwnerClaims(userId), { role: 'owner', userId });
    // no branch can produce 'courier'
    assert.equal(refreshedOwnerClaims('any').role, 'owner');
  });

  await t.test('the signed+verified token carries the owner role', async () => {
    const jwt = await signAuthToken(refreshedOwnerClaims(userId) as any, '7d');
    const decoded = await verifyAuthToken(jwt);
    assert.equal(decoded.role, 'owner');
    assert.equal((decoded as any).userId, userId);
  });
});
