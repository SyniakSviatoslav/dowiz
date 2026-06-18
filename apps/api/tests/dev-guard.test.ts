import test from 'node:test';
import assert from 'node:assert/strict';
import { isDevPath, isDevRequestAuthorized, devLoginAllowed } from '../src/plugins/dev-guard.js';

// C1 regression: /dev and /api/dev endpoints mint real JWTs and must never be
// reachable without the shared DEV_AUTH_SECRET. The guard must fail CLOSED when
// no secret is configured (production), so the endpoints 404 as if absent.

const SECRET = 's3cret-dev-token';

test('dev endpoint guard', async (t) => {
  await t.test('classifies dev paths (incl. query strings) and leaves others alone', () => {
    assert.equal(isDevPath('/api/dev/mock-auth'), true);
    assert.equal(isDevPath('/dev/mock-auth?role=owner'), true);
    assert.equal(isDevPath('/api/dev/create-assignment'), true);
    assert.equal(isDevPath('/api/dev/seed-data'), true);
    assert.equal(isDevPath('/api/owner/orders'), false);
    assert.equal(isDevPath('/api/developers'), false); // not a /dev/ segment
  });

  await t.test('fails CLOSED when no secret is configured (production default)', () => {
    // even with a "correct-looking" header, an unset secret rejects everything
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', SECRET, undefined), false);
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', SECRET, ''), false);
  });

  await t.test('rejects missing / wrong secret when one is configured', () => {
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', undefined, SECRET), false);
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', '', SECRET), false);
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', 'wrong', SECRET), false);
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', SECRET + 'x', SECRET), false); // length mismatch
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', { evil: 1 } as any, SECRET), false); // non-string
  });

  await t.test('admits the correct secret', () => {
    assert.equal(isDevRequestAuthorized('/api/dev/mock-auth', SECRET, SECRET), true);
    assert.equal(isDevRequestAuthorized('/dev/create-assignment', SECRET, SECRET), true);
  });

  await t.test('never blocks non-dev paths regardless of secret state', () => {
    assert.equal(isDevRequestAuthorized('/api/owner/orders', undefined, undefined), true);
    assert.equal(isDevRequestAuthorized('/api/owner/orders', undefined, SECRET), true);
  });

  // H1: hardcoded dev-credential bypass must be disabled in production (no secret).
  await t.test('devLoginAllowed is false without a configured secret (prod)', () => {
    assert.equal(devLoginAllowed(undefined), false);
    assert.equal(devLoginAllowed(''), false);
  });

  await t.test('devLoginAllowed is true only when a secret is configured (dev/e2e)', () => {
    assert.equal(devLoginAllowed(SECRET), true);
  });
});
