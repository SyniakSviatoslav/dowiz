import { test, expect } from '@playwright/test';

// Onboarding + auth contract on the deployed app. Covers the Telegram-login handshake,
// the Google/exchange routing (the /api-prefix fix), owner-gating of the activation
// endpoints, and the OAuth callback page. Owner-authed flows (gate→publish→Z7) are
// proven separately by the forged-owner runtime harness + the reliability gate.

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

test.describe('Flow: Onboarding + auth contract', () => {
  // ── Telegram login handshake ────────────────────────────────────────────────
  test('TG /auth/telegram/start mints a single-use token + bot deep-link', async ({ request }) => {
    const res = await request.post(`${BASE}/api/auth/telegram/start`);
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.token).toMatch(UUID);
    expect(typeof body.botUsername).toBe('string');
    expect(body.deepLink).toContain(`t.me/${body.botUsername}`);
    expect(body.deepLink).toContain(`start=login_${body.token}`);
  });

  test('TG poll before authentication = pending; unknown token = 404', async ({ request }) => {
    const start = await (await request.post(`${BASE}/api/auth/telegram/start`)).json();
    const pending = await request.get(`${BASE}/api/auth/telegram/poll?token=${start.token}`);
    expect(pending.status()).toBe(200);
    expect((await pending.json()).status).toBe('pending');

    const unknown = await request.get(`${BASE}/api/auth/telegram/poll?token=00000000-0000-4000-8000-000000000000`);
    expect(unknown.status()).toBe(404);
  });

  // ── Google / exchange routing (the /api-prefix fix) ──────────────────────────
  test('Google OAuth start redirects to accounts.google.com with the /api callback', async ({ request }) => {
    const res = await request.get(`${BASE}/api/auth/google`, { maxRedirects: 0 });
    expect(res.status()).toBe(302);
    const loc = res.headers()['location'] || '';
    expect(loc).toContain('accounts.google.com');
    expect(decodeURIComponent(loc)).toContain('/api/auth/google/callback');
  });

  test('exchange rejects an unknown code with 400', async ({ request }) => {
    const res = await request.post(`${BASE}/api/auth/exchange`, { data: { code: '00000000-0000-4000-8000-000000000000' } });
    expect(res.status()).toBe(400);
  });

  test('regression: pre-fix paths (no /api) are gone', async ({ request }) => {
    // authRoutes used to mount at /auth/* (no /api) — the bug. Must be 404 now.
    expect((await request.post(`${BASE}/auth/telegram/start`)).status()).toBe(404);
    expect((await request.get(`${BASE}/auth/google`, { maxRedirects: 0 })).status()).toBe(404);
  });

  // ── Activation endpoints are owner-gated ─────────────────────────────────────
  test('activation status/publish require auth (401 without a token)', async ({ request }) => {
    const id = '00000000-0000-4000-8000-000000000000';
    expect((await request.get(`${BASE}/api/owner/activation/${id}/status`)).status()).toBe(401);
    expect((await request.post(`${BASE}/api/owner/activation/${id}/publish`)).status()).toBe(401);
  });

  // ── OAuth callback page renders (the previously-missing handler) ──────────────
  test('/auth/callback page renders without crashing', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
    await page.goto(`${BASE}/auth/callback#code=00000000-0000-4000-8000-000000000000`, { waitUntil: 'networkidle' });
    await expect(page.locator('#root')).toBeVisible();
    // A bad code surfaces a friendly message, not a blank/crashed page.
    await expect(page.getByText(/Signing you in|Login failed|Missing login code/i)).toBeVisible();
    expect(errors.filter((e) => /Minified React error|Cannot read|is not a function/.test(e))).toHaveLength(0);
  });
});
