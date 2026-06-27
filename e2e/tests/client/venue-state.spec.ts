import { test, expect, type APIRequestContext } from '@playwright/test';
import { expectJwt } from '../../helpers/assert-shape';

// Testplan §1b (docs/research/UI-IMPROVEMENTS-TESTPLAN.md) — prove the storefront surfaces
// the CLOSED venue state: when the demo location is paused, /s/demo shows
// [data-testid="venue-closed-banner"] and the chip carries data-state="closed".
//
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test venue-state \
//        --project=desktop --reporter=list
//
// CLOSED lever (verified against source):
//   - PUT /api/owner/settings { deliveryPaused: true }  → locations.delivery_paused = true
//     (apps/api/src/routes/spa-proxy.ts:667 — targets the owner's location from the JWT)
//   - GET /public/locations/demo/info computes isOpen = !delivery_paused, then
//     status = !isOpen ? 'closed' : … (apps/api/src/routes/public/menu.ts:91,119)
//   - MenuPage derives venueStatus from that status and renders the closed banner +
//     chip data-state="closed" (apps/web/src/pages/client/MenuPage.tsx:291,553,658)
// Reversible: the location is restored to OPEN in a finally block; the closed window is
// kept tight because other concurrent tests read /s/demo.
const CREDS = { email: 'test@dowiz.com', password: 'test123456' };

// Login is rate-limited (5/min per IP). Memoize one token for the whole worker; on a 429
// sleep ~75s and retry once.
let cachedToken: string | null = null;
async function ownerToken(request: APIRequestContext): Promise<string> {
  if (cachedToken) return cachedToken;
  let res = await request.post('/api/auth/local/login', { data: CREDS });
  if (res.status() === 429) {
    await new Promise((r) => setTimeout(r, 75_000));
    res = await request.post('/api/auth/local/login', { data: CREDS });
  }
  expect(res.ok(), 'owner login should succeed').toBeTruthy();
  const body = await res.json();
  expectJwt(body.access_token, 'login access token');
  cachedToken = body.access_token as string;
  return cachedToken;
}

// Flip the owner's location open/closed via the settings endpoint. deliveryPaused=true is
// the contract's CLOSED lever (independent of hours_json, so it works at any clock time).
async function setDeliveryPaused(request: APIRequestContext, token: string, paused: boolean) {
  const res = await request.put('/api/owner/settings', {
    headers: { Authorization: `Bearer ${token}` },
    data: { deliveryPaused: paused },
  });
  expect(res.ok(), `PUT /api/owner/settings { deliveryPaused: ${paused} } should succeed`).toBeTruthy();
  const body = await res.json();
  expect(body.deliveryPaused, `settings echoes deliveryPaused=${paused}`).toBe(paused);
}

test('storefront shows the venue-closed state when the location is paused', async ({ page, request }) => {
  const token = await ownerToken(request);

  await setDeliveryPaused(request, token, true);
  try {
    await page.goto('/s/demo');

    const banner = page.locator('[data-testid="venue-closed-banner"]');
    await expect(banner).toBeVisible({ timeout: 25000 });
    await expect(banner).toContainText(/closed|mbyllur/i);

    const chip = page.locator('[data-testid="venue-state-chip"]');
    await expect(chip).toHaveAttribute('data-state', 'closed');

    // closed is not busy — the busy banner must be absent.
    await expect(page.locator('[data-testid="venue-busy-banner"]')).not.toBeVisible();
  } finally {
    await setDeliveryPaused(request, token, false);
  }
});
