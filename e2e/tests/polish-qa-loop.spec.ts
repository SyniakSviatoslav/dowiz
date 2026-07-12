import { test, expect, type APIRequestContext } from '@playwright/test';
import crypto from 'node:crypto';
import { expectJwt } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// Proof for the FE-polish + QA loop fixes.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test polish-qa-loop --project=mobile --reporter=list
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

// Credentials come from the environment — never hardcode a living staging account into a
// checked-in file (secret leak). Fail fast with a clear message at point-of-use.
function ownerCreds(): { email: string; password: string } {
  const email = process.env.E2E_OWNER_EMAIL;
  const password = process.env.E2E_OWNER_PASSWORD;
  if (!email || !password) {
    throw new Error('E2E_OWNER_EMAIL / E2E_OWNER_PASSWORD must be set to run polish-qa-loop owner tests');
  }
  return { email, password };
}

// MUTATING endpoint (PATCH /orders/:id/status) — refuse to run against prod/unknown targets.
test.beforeAll(() => requireStaging(BASE));

test('FE-polish: login form controls meet the 44px tap-target minimum', async ({ page }) => {
  await page.goto('/login');
  const pw = page.locator('input[type="password"]');
  await expect(pw).toBeVisible();
  const pwBox = await pw.boundingBox();
  expect(pwBox!.height, `password input height ${pwBox?.height}`).toBeGreaterThanOrEqual(44);
  // The primary submit button (md Button → min-h-11). Absence must FAIL, not silently skip.
  const submit = page.locator('button[type="submit"]').first();
  await expect(submit).toHaveCount(1);
  const b = await submit.boundingBox();
  expect(b!.height, `submit button height ${b?.height}`).toBeGreaterThanOrEqual(44);
});

async function login(request: APIRequestContext, creds: { email: string; password: string }, label: string): Promise<string> {
  const res = await request.post('/api/auth/local/login', { data: creds });
  expect(res.status(), `${label} login status`).toBe(200);
  const token = (await res.json()).access_token as unknown;
  expectJwt(token, `${label} access_token`);
  return String(token);
}

function ownerToken(request: APIRequestContext): Promise<string> {
  return login(request, ownerCreds(), 'owner');
}

test('QA: PATCH /orders/:id/status with a bad enum returns a typed 400 (was raw 500)', async ({ request }) => {
  const t = await ownerToken(request);
  // The enum is rejected at parse, before any DB work — so any well-formed uuid id reaches the
  // validation. Expect 400, NOT 500 (the bare .parse() ZodError used to fall through to 500).
  // NOTE: this only exercises the Zod parse layer (random UUID, no real order). True cross-tenant
  // IDOR coverage (owner-A PATCHing owner-B's real order → expect 403/404) needs a seeded second
  // tenant on staging. TODO(needs_staging): add the 2nd-tenant IDOR scenario.
  const res = await request.patch(`/api/orders/${crypto.randomUUID()}/status`, {
    headers: { authorization: `Bearer ${t}` },
    data: { status: 'FLYING' },
  });
  expect(res.status(), `bad-enum status (${res.status()})`).toBe(400);
});

test('QA: PATCH /orders/:id/status without a Bearer token is rejected 401', async ({ request }) => {
  // Negative control: verifyAuth runs before requireRole → unauthenticated must 401.
  const res = await request.patch(`/api/orders/${crypto.randomUUID()}/status`, {
    data: { status: 'IN_DELIVERY' },
  });
  expect(res.status(), `no-token status (${res.status()})`).toBe(401);
});

test('QA: PATCH /orders/:id/status with a courier-role token is rejected 403', async ({ request }) => {
  // Negative control: requireRole(['owner']) → an authenticated non-owner must 403.
  // TODO(needs_staging): requires a real courier account on staging.
  const email = process.env.E2E_COURIER_EMAIL;
  const password = process.env.E2E_COURIER_PASSWORD;
  if (!email || !password) {
    throw new Error('E2E_COURIER_EMAIL / E2E_COURIER_PASSWORD must be set to run the courier-role 403 control');
  }
  const t = await login(request, { email, password }, 'courier');
  const res = await request.patch(`/api/orders/${crypto.randomUUID()}/status`, {
    headers: { authorization: `Bearer ${t}` },
    data: { status: 'IN_DELIVERY' },
  });
  expect(res.status(), `wrong-role status (${res.status()})`).toBe(403);
});
