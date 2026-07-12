import test from 'node:test';
import assert from 'node:assert/strict';
import { signAuthToken, verifyAuthToken } from '@deliveryos/platform';
import { refreshedOwnerClaims } from '../src/routes/auth.js';

// H5 regression: /auth/refresh must never mint anything but an owner token.
// Previously the role was inferred from a nullable users.google_sub column,
// which could escalate/flip a dual-identity user's role on refresh.

const JWT_RE = /^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/;
const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

test('refresh always issues an owner token', async (t) => {
  const userId = '11111111-1111-1111-1111-111111111111';
  const locId = '22222222-2222-2222-2222-222222222222';

  await t.test('claims builder is invariant to user identity', () => {
    assert.deepEqual(refreshedOwnerClaims(userId), { role: 'owner', userId });
    // no branch can produce 'courier'
    assert.equal(refreshedOwnerClaims('any').role, 'owner');
  });

  await t.test('the signed+verified token carries the owner role', async () => {
    const jwt = await signAuthToken(refreshedOwnerClaims(userId) as any, '7d');
    assert.match(jwt, JWT_RE, 'signed token must be a 3-segment JWT');
    const decoded = await verifyAuthToken(jwt);
    assert.equal(decoded.role, 'owner');
    assert.equal((decoded as any).userId, userId);
  });

  // Finding 1 [HIGH]: activeLocationId param — R2-2 (ADR-0004) requires the working
  // tenant to survive a refresh; an unscoped refreshed token loses tenant scoping.
  await t.test('activeLocationId is carried into claims and the signed+verified JWT', async () => {
    assert.match(locId, UUID_RE);
    const claims = refreshedOwnerClaims(userId, locId);
    assert.deepEqual(claims, { role: 'owner', userId, activeLocationId: locId });

    const jwt = await signAuthToken(claims as any, '7d');
    assert.match(jwt, JWT_RE);
    const decoded = await verifyAuthToken(jwt);
    assert.equal(decoded.role, 'owner');
    assert.equal((decoded as any).userId, userId);
    assert.equal((decoded as any).activeLocationId, locId);
    assert.match((decoded as any).activeLocationId, UUID_RE);
  });

  await t.test('activeLocationId is OMITTED (not null/undefined) when absent or falsy', () => {
    // No location, explicit undefined, explicit null, and empty string must all
    // produce the bare owner claim with no activeLocationId key (route falls back
    // to a deterministic owner membership rather than carrying a falsy scope).
    for (const arg of [undefined, null, ''] as const) {
      const claims = refreshedOwnerClaims(userId, arg);
      assert.deepEqual(claims, { role: 'owner', userId });
      assert.ok(!('activeLocationId' in claims), `activeLocationId must be absent for ${String(arg)}`);
    }
  });
});

// Finding 2 [HIGH]: privilege-escalation negative control — a valid courier-session
// token POSTed to /auth/refresh must NOT yield an owner access token. The courier
// token is absent from auth_refresh_tokens, so the route returns 401 UNAUTHORIZED
// (apps/api/src/routes/auth.ts:251). This needs the real Fastify route + a live DB
// (courier session + memberships), which this pure-unit harness (_env-stub, no server)
// cannot exercise without faking the DB — covering it would be a false-green here.
// TODO(needs_staging): add an E2E that POSTs a real courier session token to
//   /auth/refresh against dowiz-staging and asserts response.status() === 401
//   (UNAUTHORIZED) and that no owner access_token is returned. requireStaging(BASE)
//   in beforeAll; never run against prod.
