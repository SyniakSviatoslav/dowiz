import { test, expect } from '@playwright/test';
<<<<<<< Updated upstream
=======
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// BASE defaults to staging (never prod) — these specs exercise the dev/mock-auth backdoor and
// the owner onboarding flow, which must never run against the prod host.
const BASE_URL = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
>>>>>>> Stashed changes

test.describe('Notification Helper Tests', () => {
  // Hard guard: fail fast if pointed at prod / an unknown target (these tests mint dev tokens
  // and the link-telegram flow mutates state via owner onboarding).
  test.beforeAll(() => {
    requireStaging(BASE_URL);
  });

  test('linkTelegram performs the connect flow and returns a valid token + deep link', async () => {
    const { linkTelegram } = await import('../helpers/notifHelpers');
    expect(typeof linkTelegram).toBe('function');

    // TODO(needs_staging): exercises the full owner mock-auth → onboarding → connect-init flow
    // against a live staging API; cannot be stubbed without a real backend.
    const result = await linkTelegram('owner');
    expectUuid(result.connectToken, 'connectToken');
    expectUuid(result.locationId, 'locationId');
    expectUuid(result.userId, 'userId');
    expect(result.deepLink).toContain('t.me/');
    expect(result.deepLink).toContain(result.connectToken);
  });

  test('mock-auth issues a valid owner JWT (positive control)', async () => {
    const authRes = await fetch(`${BASE_URL}/api/dev/mock-auth`, { method: 'POST' });
    // `fetch` Response exposes `.status` as a number property, not a method.
    expect(authRes.status).toBe(200);
    const authBody = await authRes.json();
<<<<<<< Updated upstream
    expect(authBody.access_token).toBeTruthy();
=======
    expectJwt(authBody.access_token);
    expectUuid(authBody.userId, 'userId');
  });

  test('connect-init without a token is rejected with 401 (negative control)', async () => {
    // verifyAuth (onRequest hook in apps/api/src/routes/owner/notifications.ts) rejects a
    // missing Bearer token with exactly 401 before any param/DB work — assert that gate.
    const locationId = '11111111-1111-4111-8111-111111111111';
    const res = await fetch(
      `${BASE_URL}/api/owner/locations/${locationId}/notifications/telegram/connect-init`,
      { method: 'POST' },
    );
    expect(res.status).toBe(401);
>>>>>>> Stashed changes
  });
});
