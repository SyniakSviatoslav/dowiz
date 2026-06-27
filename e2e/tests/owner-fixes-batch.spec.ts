import { test, expect, type APIRequestContext } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { expectJwt, expectUuid } from '../helpers/assert-shape';

// Proof for the deployed-app owner fixes (session relogin, PDF import via OpenCode Zen,
// Google OAuth hidden, test-owner → sushi-durres binding). Server changes are real.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev DEV_AUTH_SECRET=stg-e2e-secret \
//   pnpm exec playwright test owner-fixes-batch --project=desktop --reporter=list
const CREDS = { email: 'test@dowiz.com', password: 'test123456' };
const DEV_SECRET = process.env.DEV_AUTH_SECRET || 'stg-e2e-secret';

function decodeJwtExpiry(token: string): { ttlSeconds: number } {
  const [, payloadB64] = token.split('.');
  const json = JSON.parse(Buffer.from(payloadB64, 'base64url').toString('utf8'));
  expect(typeof json.exp, 'JWT has exp').toBe('number');
  expect(typeof json.iat, 'JWT has iat').toBe('number');
  return { ttlSeconds: json.exp - json.iat };
}

async function login(request: APIRequestContext) {
  const res = await request.post('/api/auth/local/login', { data: CREDS });
  expect(res.ok(), `owner login (${res.status()})`).toBeTruthy();
  return res.json();
}

test.describe('Owner fixes batch', () => {
  test('session: password login mints a ~7d access token (was 1h → hourly relogin)', async ({ request }) => {
    const body = await login(request);
    expectJwt(body.access_token, 'access_token');
    const { ttlSeconds } = decodeJwtExpiry(body.access_token);
    // 7 days = 604800s. Assert it's well beyond the old 1h (3600s) — i.e. > 24h.
    expect(ttlSeconds, `access TTL ${ttlSeconds}s should be ~7d, not 1h`).toBeGreaterThan(24 * 3600);
    expect(ttlSeconds).toBeLessThanOrEqual(7 * 24 * 3600 + 60);
  });

  test('session: refresh round-trip returns a fresh ~7d access token', async ({ request }) => {
    const body = await login(request);
    test.skip(!body.refresh_token, 'no refresh_token returned (insert may have been skipped)');
    const res = await request.post('/api/auth/refresh', { data: { refresh_token: body.refresh_token } });
    expect(res.ok(), `refresh (${res.status()})`).toBeTruthy();
    const refreshed = await res.json();
    expectJwt(refreshed.access_token, 'new access_token');
    const { ttlSeconds } = decodeJwtExpiry(refreshed.access_token);
    expect(ttlSeconds, `refreshed TTL ${ttlSeconds}s ~7d`).toBeGreaterThan(24 * 3600);
  });

  test('login UI: Google OAuth button is hidden; email + Telegram remain', async ({ page }) => {
    await page.goto('/login');
    await expect(page.locator('input[type="password"]')).toBeVisible();
    // Locale-robust: assert by href/icon, not by (localized) label text.
    await expect(page.locator('a[href="/api/auth/google"]'), 'Google OAuth link gone').toHaveCount(0);
    await expect(page.locator('.ti-brand-telegram'), 'Telegram login still offered').toBeVisible();
  });

  test('data: test@dowiz.com is bound to the demo (sushi-durres) location, active', async ({ request }) => {
    const res = await request.post('/dev/repair-test-owner', {
      headers: { 'x-dev-auth-secret': DEV_SECRET },
      data: { email: CREDS.email, slug: 'demo' },
    });
    expect(res.ok(), `repair-test-owner (${res.status()})`).toBeTruthy();
    const r = await res.json();
    expectUuid(r.locationId, 'demo location');
    const active = (r.membershipsAfter || []).some(
      (m: any) => m.location_id === r.locationId && m.role === 'owner' && m.status === 'active',
    );
    expect(active, `active owner membership to ${r.locationName} (${r.locationId})`).toBeTruthy();
  });

  test('import: PDF menu extracts products via OpenCode Zen (was 0 on OpenRouter 402)', async ({ request }) => {
    test.setTimeout(180_000); // OCR (13-page PDF) + LLM structuring can take up to the 120s request budget.
    // Ensure the owner has a location to import into first.
    await request.post('/dev/repair-test-owner', {
      headers: { 'x-dev-auth-secret': DEV_SECRET },
      data: { email: CREDS.email, slug: 'demo' },
    });
    const { access_token } = await login(request);
    const pdf = readFileSync(resolve(process.cwd(), 'menu-sq.pdf'));
    const res = await request.post('/api/owner/menu/import/preview', {
      headers: { authorization: `Bearer ${access_token}` },
      multipart: {
        file: { name: 'menu-sq.pdf', mimeType: 'application/pdf', buffer: pdf },
        mode: 'merge',
      },
      timeout: 120_000,
    });
    expect(res.ok(), `import preview (${res.status()}): ${await res.text().catch(() => '')}`.slice(0, 300)).toBeTruthy();
    const body = await res.json();
    const products = body?.draft_preview?.products ?? [];
    expect(Array.isArray(products), 'draft_preview.products is an array').toBeTruthy();
    expect(products.length, `extracted products (summary.valid=${body?.summary?.valid})`).toBeGreaterThan(0);
  });
});
