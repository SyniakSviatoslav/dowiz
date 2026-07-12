import { test, expect } from '@playwright/test';
<<<<<<< Updated upstream

test.describe('Simple Auth Test', () => {
  test('should be able to get owner token', async () => {
    const BASE_URL = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
    console.log(`Testing with BASE_URL: ${BASE_URL}`);
    
    const authRes = await fetch(`${BASE_URL}/api/dev/mock-auth`, { method: 'POST' });
    console.log(`Auth response status: ${authRes.status}`);
    
    expect(authRes.status).toBe(200);
    const authBody = await authRes.json();
    expect(authBody.access_token).toBeTruthy();
    console.log('Successfully got token');
=======
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// This spec exercises the /api/dev/mock-auth backdoor (mints a REAL owner JWT). Default to
// staging and hard-guard against prod: if ALLOW_DEV_LOGIN were ever accidentally on in prod,
// a prod-defaulted test would go green and silently prove the backdoor is open. The root
// playwright.config injects `x-dev-auth-secret` into the `request` fixture (from DEV_AUTH_SECRET);
// raw `fetch` deliberately does NOT carry it — that asymmetry powers the fail-closed test below.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

test.describe('Simple Auth Test', () => {
  test.beforeAll(() => {
    requireStaging(BASE);
>>>>>>> Stashed changes
  });

  test('mock-auth mints a usable, owner-scoped token', async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status(), 'mock-auth must be reachable on staging').toBe(200);

    const authBody = await authRes.json();
    // SHAPE, not truthiness: '', 'null', false, or an error string would all pass a truthy check.
    expectJwt(authBody.access_token, 'access_token');
    expectUuid(authBody.activeLocationId, 'activeLocationId');

    // Prove the token is actually USABLE and correctly owner-scoped — send it to a protected
    // owner-only route and assert it grants access (not merely that it is a non-empty string).
    // TODO(needs-staging): depends on the seeded demo owner+location on the live target.
    const authed = await request.get(
      `${BASE}/api/owner/locations/${authBody.activeLocationId}/couriers`,
      { headers: { Authorization: `Bearer ${authBody.access_token}` } },
    );
    expect(authed.status(), 'owner token must grant access to an owner-only route').toBe(200);

    // Negative control (Test Integrity #4): the same route with NO token must be rejected, so
    // the 200 above proves authorization rather than an endpoint that is simply open to everyone.
    const anon = await request.get(
      `${BASE}/api/owner/locations/${authBody.activeLocationId}/couriers`,
    );
    expect(anon.status(), 'owner route must reject an anonymous caller').toBe(401);
  });

  test('mock-auth is fail-closed without the dev secret (backdoor stays shut)', async () => {
    // Raw fetch omits x-dev-auth-secret, so the dev guard must 404 — failing closed and never
    // leaking the endpoint's presence. Goes red if the secret gate is ever weakened/removed.
    const res = await fetch(`${BASE}/api/dev/mock-auth`, { method: 'POST' });
    expect(res.status).toBe(404);
  });
});
