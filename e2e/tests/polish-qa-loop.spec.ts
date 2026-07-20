import { test, expect, type APIRequestContext } from '@playwright/test';
import crypto from 'node:crypto';

// Proof for the FE-polish + QA loop fixes.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test polish-qa-loop --project=mobile --reporter=list
const CREDS = { email: 'test@dowiz.com', password: 'test123456' };

test('FE-polish: login form controls meet the 44px tap-target minimum', async ({ page }) => {
  await page.goto('/login');
  const pw = page.locator('input[type="password"]');
  await expect(pw).toBeVisible();
  const pwBox = await pw.boundingBox();
  expect(pwBox!.height, `password input height ${pwBox?.height}`).toBeGreaterThanOrEqual(44);
  // The primary submit button (md Button → min-h-11).
  const submit = page.locator('button[type="submit"]').first();
  if (await submit.count()) {
    const b = await submit.boundingBox();
    expect(b!.height, `submit button height ${b?.height}`).toBeGreaterThanOrEqual(44);
  }
});

async function ownerToken(request: APIRequestContext): Promise<string> {
  const res = await request.post('/api/auth/local/login', { data: CREDS });
  expect(res.ok(), 'owner login').toBeTruthy();
  return (await res.json()).access_token as string;
}

test('QA: PATCH /orders/:id/status with a bad enum returns a typed 400 (was raw 500)', async ({ request }) => {
  const t = await ownerToken(request);
  // The enum is rejected at parse, before any DB work — so any well-formed uuid id reaches the
  // validation. Expect 400, NOT 500 (the bare .parse() ZodError used to fall through to 500).
  const res = await request.patch(`/api/orders/${crypto.randomUUID()}/status`, {
    headers: { authorization: `Bearer ${t}` },
    data: { status: 'FLYING' },
  });
  expect(res.status(), `bad-enum status (${res.status()})`).toBe(400);
  expect(res.status(), 'must not be a raw 500').not.toBe(500);
});
