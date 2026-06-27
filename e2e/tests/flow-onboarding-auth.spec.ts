import { test, expect } from '@playwright/test';
import { requireStaging } from '../helpers/staging-guard';

// Onboarding + auth contract on the deployed app. Covers the Telegram-login handshake,
// the Google/exchange routing (the /api-prefix fix), owner-gating of the activation
// endpoints, and the OAuth callback page. Owner-authed flows (gate→publish→Z7) are
// proven separately by the forged-owner runtime harness + the reliability gate.

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

test.describe('Flow: Onboarding + auth contract', () => {
  // /auth/telegram/start INSERTs a login token — mutating, so refuse to run against prod.
  test.beforeAll(() => requireStaging(BASE));

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
    expect(start.token).toMatch(UUID);
    const pending = await request.get(`${BASE}/api/auth/telegram/poll?token=${start.token}`);
    expect(pending.status()).toBe(200);
    expect((await pending.json()).status).toBe('pending');

    // Polling does not consume a still-pending token: a second poll with the SAME token
    // must resolve identically (200/pending), not 404/410 — a mere read can't invalidate it.
    // (auth.ts: only authenticated→consumed flips state; pending is idempotent.)
    const pending2 = await request.get(`${BASE}/api/auth/telegram/poll?token=${start.token}`);
    expect(pending2.status()).toBe(200);
    expect((await pending2.json()).status).toBe('pending');
    // TODO(needs_staging): prove true single-use REDEMPTION — bind the token via the bot
    // (authenticated), poll once (200+access_token, flips to consumed), then re-poll the same
    // token → expect 410/consumed. Requires a live Telegram bot bind on staging.

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
    // Negative control only: no token → 401 fires at verifyAuth, BEFORE any tenant lookup,
    // so the all-zeros id is fine here (it proves the auth gate, not isolation).
    const id = '00000000-0000-4000-8000-000000000000';
    expect((await request.get(`${BASE}/api/owner/activation/${id}/status`)).status()).toBe(401);
    expect((await request.post(`${BASE}/api/owner/activation/${id}/publish`)).status()).toBe(401);
  });

  // TODO(needs_staging): cross-tenant IDOR + positive control for /owner/activation.
  // The 401 test above is a NEGATIVE control only; the gate could be silently rejecting
  // everyone. requireLocationAccess (activation.ts preHandler) must be proven with REAL
  // tokens (no all-zeros / nil-UUID — that 404s by absence, proving nothing):
  //   1. mint two distinct owner tokens for two real staging tenants A and B;
  //   2. POSITIVE: token A → GET /activation/<A.locationId>/status → 200, body.slug === A.slug;
  //   3. IDOR:     token A → GET /activation/<B.locationId>/status → 403 (requireLocationAccess).
  // Needs a live staging run with two seeded tenants + real owner JWTs.

  // ── OAuth callback page renders (the previously-missing handler) ──────────────
  test('/auth/callback page renders without crashing', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
    await page.goto(`${BASE}/auth/callback#code=00000000-0000-4000-8000-000000000000`, { waitUntil: 'networkidle' });
    await expect(page.locator('#root')).toBeVisible();
    // The code IS present (not missing) but invalid → /auth/exchange 400 → catch → the
    // EXACT 'Login failed.' branch must fire and be visible (AuthCallback.tsx:25-28).
    // An OR over all three strings would green even if the spinner stuck or the wrong branch ran.
    await expect(page.getByText('Login failed.', { exact: true })).toBeVisible();
    expect(errors.filter((e) => /Minified React error|Cannot read|is not a function/.test(e))).toHaveLength(0);
  });
});
