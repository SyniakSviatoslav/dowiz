import test from 'node:test';
import assert from 'node:assert/strict';
import { isDevPath, isDevRequestAuthorized, devLoginAllowed, type DevAuthEnv } from '../src/plugins/dev-guard.js';

// ADR-0003 regression: /dev and /api/dev endpoints mint real JWTs and the /auth/local
// dev bypass mints owner tokens. Both must fail CLOSED in production. The gate now
// requires BOTH the explicit ALLOW_DEV_LOGIN flag AND the shared DEV_AUTH_SECRET — the
// secret alone is no longer sufficient (that was the live-incident root cause: a leaked
// secret on a prod box minted owner JWTs). Production sets neither and boot-guard D
// rejects either, so the bypass can never activate there.

const SECRET = 's3cret-dev-token';
// Fully-enabled dev env (staging/CI/local): flag ON + secret present.
const ON: DevAuthEnv = { ALLOW_DEV_LOGIN: 'true', DEV_AUTH_SECRET: SECRET };
// Production-shaped env: flag off, no secret.
const PROD: DevAuthEnv = { ALLOW_DEV_LOGIN: 'false' };
// The dangerous misconfig the old code honored: secret leaked but flag still off.
const SECRET_ONLY: DevAuthEnv = { ALLOW_DEV_LOGIN: 'false', DEV_AUTH_SECRET: SECRET };
// Flag on but no secret — still closed (both are required).
const FLAG_ONLY: DevAuthEnv = { ALLOW_DEV_LOGIN: 'true' };

test('dev endpoint guard', async (t) => {
  await t.test('classifies dev paths (incl. query strings) and leaves others alone', () => {
    assert.equal(isDevPath('/api/dev/mock-auth'), true);
    assert.equal(isDevPath('/dev/mock-auth?role=owner'), true);
    assert.equal(isDevPath('/api/dev/create-assignment'), true);
    assert.equal(isDevPath('/api/dev/seed-data'), true);
    assert.equal(isDevPath('/api/owner/orders'), false);
    assert.equal(isDevPath('/api/developers'), false); // not a /dev/ segment
  });

  await t.test('fails CLOSED in production (flag off, no secret)', () => {
    // even with a "correct-looking" header, a prod-shaped env rejects everything
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', SECRET, PROD), false);
  });

  await t.test('fails CLOSED when the secret leaks but the flag is still off (incident root cause)', () => {
    // The OLD guard returned true here — minting owner JWTs on prod. The flag closes it.
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', SECRET, SECRET_ONLY), false);
    assert.equal(devLoginAllowed(SECRET_ONLY), false);
  });

  await t.test('fails CLOSED when the flag is on but no secret is configured', () => {
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', SECRET, FLAG_ONLY), false);
    assert.equal(devLoginAllowed(FLAG_ONLY), false);
  });

  await t.test('rejects missing / wrong secret when the gate is fully enabled', () => {
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', undefined, ON), false);
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', '', ON), false);
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', 'wrong', ON), false);
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', SECRET + 'x', ON), false); // length mismatch
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', { evil: 1 } as any, ON), false); // non-string
  });

  await t.test('admits the correct secret only when fully enabled (flag + secret)', () => {
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', SECRET, ON), true);
    assert.equal(isDevRequestAuthorized('/dev/create-assignment', SECRET, ON), true);
  });

  await t.test('never blocks non-dev paths regardless of gate state', () => {
    assert.equal(isDevRequestAuthorized('/api/owner/orders', undefined, PROD), true);
    assert.equal(isDevRequestAuthorized('/api/owner/orders', undefined, ON), true);
  });

  await t.test('devLoginAllowed requires BOTH the flag AND the secret', () => {
    assert.equal(devLoginAllowed(PROD), false);
    assert.equal(devLoginAllowed(SECRET_ONLY), false);
    assert.equal(devLoginAllowed(FLAG_ONLY), false);
    assert.equal(devLoginAllowed(ON), true);
  });
});
