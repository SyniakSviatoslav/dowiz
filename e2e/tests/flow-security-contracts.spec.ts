import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
let authToken: string;

test.describe.configure({ mode: 'serial' });

test.describe('Flow: Security & Contracts — Auth, CSP, Cookies, Rate Limits, Viewports', () => {
  test.beforeAll(async ({ request }) => {
    // mock-auth UPSERTs the dev user (a write) — never let this run against prod (#7).
    requireStaging(BASE);
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    authToken = (await authRes.json()).access_token;
    expectJwt(authToken, 'mock-auth access_token');
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 1: All protected endpoints return 401 without auth
  // ──────────────────────────────────────────────────────────────
  test('Flow 1: Security — protected endpoints return 401 without Authorization header', async ({ request }) => {
    const protectedRoutes = [
      { method: 'GET', url: `${BASE}/api/owner/settings` },
      { method: 'GET', url: `${BASE}/api/owner/menu/categories` },
      { method: 'GET', url: `${BASE}/api/owner/menu/products` },
      { method: 'GET', url: `${BASE}/api/courier/me` },
      { method: 'GET', url: `${BASE}/api/courier/me/assignments` },
      { method: 'GET', url: `${BASE}/api/courier/me/shift` },
      { method: 'GET', url: `${BASE}/api/courier/me/earnings` },
      { method: 'GET', url: `${BASE}/api/courier/me/history` },
    ];

    for (const route of protectedRoutes) {
      let res;
      if (route.method === 'GET') {
        res = await request.get(route.url);
      } else if (route.method === 'POST') {
        res = await request.post(route.url, { data: {} });
      }
      expect(res!.status(), `${route.method} ${route.url} should return 401`).toBe(401);
      const body = await res!.json();
      expect(body.error || body.message).toBeTruthy();
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 2: All protected POST/PATCH endpoints reject invalid input (400)
  // ──────────────────────────────────────────────────────────────
  test('Flow 2: Security — protected endpoints return 400 on invalid input', async ({ request }) => {
    const invalidRoutes = [
      { method: 'POST', url: `${BASE}/api/owner/menu/categories`, data: { invalid: true } },
      { method: 'POST', url: `${BASE}/api/owner/menu/products`, data: { invalid: true } },
    ];

    for (const route of invalidRoutes) {
      const res = await request.post(route.url, {
        data: route.data,
        headers: { Authorization: `Bearer ${authToken}` },
      });
      // Both routes Zod-validate the body (categories .strict(), products required name/price/
      // prep_time_minutes) → setErrorHandler maps the ZodError to EXACTLY 400 VALIDATION_FAILED
      // (server.ts:435-457). Assert the exact status + machine code (#2 — not the old not.toBe(401)).
      expect(res.status(), `${route.method} ${route.url} must reject invalid input with 400`).toBe(400);
      const body = await res.json();
      expect(body.code, `${route.method} ${route.url} VALIDATION_FAILED envelope`).toBe('VALIDATION_FAILED');
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 3: AuthToken JWT decodes with correct claims
  // ──────────────────────────────────────────────────────────────
  test('Flow 3: Security — JWT from mock-auth has correct claims', async ({ request }) => {
    // Decode JWT (without verifying signature — just inspect claims)
    expectJwt(authToken, 'mock-auth token');
    const parts = authToken.split('.');
    expect(parts.length).toBe(3);
    const payload = JSON.parse(Buffer.from(parts[1], 'base64url').toString('utf-8'));
    expect(payload.role).toBe('owner');
    expectUuid(payload.userId ?? payload.sub, 'jwt subject');
    expect(payload.iat).toBeTruthy();
    expect(payload.exp).toBeTruthy();
    expect(payload.exp).toBeGreaterThan(payload.iat);
    // #9: exp > iat alone passes for an already-expired token — assert it is still valid NOW.
    expect(payload.exp, 'token must not already be expired').toBeGreaterThan(Math.floor(Date.now() / 1000));
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 4: Rate limiting headers present on public endpoints
  // ──────────────────────────────────────────────────────────────
  test('Flow 4: Security — rate limit headers on public POST endpoints', async ({ request }) => {
    const res = await request.post(`${BASE}/api/orders`, {
      data: {
        locationId: '00000000-0000-0000-0000-000000000000',
        type: 'delivery',
        items: [{ product_id: '00000000-0000-0000-0000-000000000001', quantity: 1 }],
        delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Test' },
        payment: { method: 'cash' },
        idempotency_key: crypto.randomUUID(),
      },
    });
    // @fastify/rate-limit is registered globally (max:100/1min, server.ts:345) → it emits the
    // x-ratelimit-* headers on every response by default. Assert the header is actually present
    // (#1 — hasRateLimit was computed but never asserted; the rate-limit check was dead code).
    const headers = res.headers();
    const hasRateLimit = headers['x-ratelimit-limit'] !== undefined ||
                         headers['ratelimit-limit'] !== undefined ||
                         headers['x-ratelimit-remaining'] !== undefined;
    expect(hasRateLimit, 'rate-limit header must be present on public POST /api/orders').toBe(true);
    expect(res.status()).not.toBe(500);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 5: CSP headers on every page
  // ──────────────────────────────────────────────────────────────
  test('Flow 5: Security — CSP headers present on all page types', async ({ page }) => {
    const pages = [
      { url: `${BASE}/s/test-slug`, label: 'client menu' },
      { url: `${BASE}/admin`, label: 'admin dashboard' },
      { url: `${BASE}/courier/login`, label: 'courier login' },
      { url: `${BASE}/login`, label: 'login' },
    ];

    for (const { url, label } of pages) {
      const resp = await page.goto(url, { waitUntil: 'networkidle' });
      if (resp) {
        // #4: the CSP block was inside `if (csp)` — an entirely absent header silently passed.
        // The SSR shell (spa-shell.ts) + global security hook (security/headers.ts) set CSP on
        // every page response, so assert presence UNCONDITIONALLY before inspecting it.
        const csp = resp.headers()['content-security-policy'];
        expect(csp, `${label}: CSP header must be present`).toBeTruthy();
        expect(csp).toContain('default-src');
        expect(csp).toContain('script-src');
        expect(resp.status(), `${label}: status 200`).toBe(200);
      }
      const cookies = await page.context().cookies();
      expect(cookies, `${label}: no cookies`).toEqual([]);
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 6: 0 cookies on ALL pages
  // ──────────────────────────────────────────────────────────────
  test('Flow 6: Security — zero cookies on all page types', async ({ page }) => {
    const pages = [
      `${BASE}/s/test-slug`,
      `${BASE}/signup`,
      `${BASE}/login`,
      `${BASE}/courier/login`,
      `${BASE}/courier`,
      `${BASE}/admin`,
      `${BASE}/admin/menu`,
      `${BASE}/admin/branding`,
    ];

    for (const url of pages) {
      await page.goto(url, { waitUntil: 'networkidle' });
      await expect(page.locator('body')).toBeAttached({ timeout: 5000 });
      const cookies = await page.context().cookies();
      expect(cookies, `${url}: should have 0 cookies`).toEqual([]);
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 7: Embed mode — embed-mode class, no fixed positioning
  // ──────────────────────────────────────────────────────────────
  test('Flow 7: Embed — embed-mode class applied, no fixed elements, no cookies', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto(`${BASE}/s/test-slug?dev=true&embed=true`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);

    const hasEmbedClass = await page.evaluate(() => {
      return document.documentElement.classList.contains('embed-mode') ||
             document.body.classList.contains('embed-mode') ||
             document.getElementById('root')?.classList.contains('embed-mode') ||
             document.documentElement.getAttribute('data-embed') === 'true' ||
             document.documentElement.hasAttribute('data-embed');
    });
    // #5: ClientLayout.tsx adds `embed-mode` to document.body when ?embed=true (L41-43), so the
    // marker MUST be present once the SPA has mounted — assert it (was computed but never checked).
    expect(hasEmbedClass, 'embed-mode marker must be present in embed mode').toBe(true);

    const hasFixed = await page.evaluate(() => {
      const all = document.querySelectorAll('*');
      for (const el of all) {
        if (getComputedStyle(el).position === 'fixed') return true;
      }
      return false;
    });
    expect(hasFixed).toBe(false);

    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 8: Allergens format — public menu returns strings, admin returns arrays
  // ──────────────────────────────────────────────────────────────
  test('Flow 8: Contract — allergens as strings in public menu, arrays in admin API', async ({ request }) => {
    // First resolve slug
    const settingsRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(settingsRes.status()).toBe(200);
    const slug = (await settingsRes.json()).slug;

    // Get products from admin API
    const adminRes = await request.get(`${BASE}/api/owner/menu/products`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(adminRes.status()).toBe(200);
    const adminProds = await adminRes.json();
    // Structural floor (always fires): the admin products endpoint returns an array.
    expect(Array.isArray(adminProds), 'admin /menu/products returns an array').toBe(true);
    // TODO(needs-allergen-seed): #8 — the allergen-array contract is only exercised when a product
    // actually carries allergens. With the current demo fixture this may be empty → 0 assertions.
    // Seed a product with allergens in the demo fixture, then assert this unconditionally.
    const withAllergens = adminProds.find((p: any) => p.allergens && p.allergens.length > 0);
    if (withAllergens) {
      expect(Array.isArray(withAllergens.allergens), 'admin allergens must be an array').toBe(true);
    }

    // Get public menu
    const menuRes = await request.get(`${BASE}/public/locations/${slug}/menu`);
    expect(menuRes.status()).toBe(200);
    const menu = await menuRes.json();
    expect(Array.isArray(menu.categories), 'public menu has a categories array').toBe(true);
    const allProds = menu.categories.flatMap((c: any) => c.products || []);
    for (const p of allProds) {
      if (p.attributes?.bom) {
        for (const bom of p.attributes.bom) {
          if (bom.allergens) {
            // Public menu allergens can be strings (comma-separated) or arrays
            const isValid = typeof bom.allergens === 'string' || Array.isArray(bom.allergens);
            expect(isValid).toBe(true);
          }
        }
      }
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 9: Cross-tenant query returns 404
  // ──────────────────────────────────────────────────────────────
  test('Flow 9: Security — cross-tenant query returns 404', async ({ request }) => {
    // TODO(needs-2nd-tenant): #3 — a nil/all-zero UUID 404s by ABSENCE (no row), not by an
    // ownership check, so a real cross-tenant LEAK (owner A reading owner B's REAL location id)
    // is NOT exercised here. Provision a second owner fixture, capture its real locationId, and
    // assert owner A gets 404 for that id. Until then this only proves "unknown id → 404".
    const fakeLocationId = '00000000-0000-0000-0000-000000000000';
    const crossRes = await request.get(
      `${BASE}/api/owner/locations/${fakeLocationId}/dashboard/snapshot`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(crossRes.status()).toBe(404);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 10: All 3 viewports — client menu renders without horizontal scroll
  // ──────────────────────────────────────────────────────────────
  test('Flow 10: Responsive — menu page renders on mobile, tablet, desktop without overflow', async ({ page }) => {
    const viewports = [
      { width: 390, height: 844, label: 'mobile' },
      { width: 768, height: 1024, label: 'tablet' },
      { width: 1280, height: 800, label: 'desktop' },
    ];

    for (const vp of viewports) {
      await page.setViewportSize(vp);
      const errors: string[] = [];
      page.on('pageerror', err => errors.push(err.message));

      await page.goto(`${BASE}/s/test-slug?dev=true`, { waitUntil: 'networkidle' });
      await expect(page.locator('body')).toBeAttached({ timeout: 10000 });

      const scrollWidth = await page.evaluate(() => document.documentElement.scrollWidth);
      expect(scrollWidth, `${vp.label}: no horizontal overflow`).toBeLessThanOrEqual(vp.width + 5);

      const cards = page.locator('[data-testid="menu-item"]');
      const cardCount = await cards.count().catch(() => 0);
      if (cardCount > 0) {
        await expect(cards.first()).toBeVisible({ timeout: 5000 });
      }

      const criticalErrors = errors.filter(e =>
        !e.includes('favicon') && !e.includes('404') && !e.includes('manifest')
      );
      expect(criticalErrors, `${vp.label}: no JS errors`).toEqual([]);

      const cookies = await page.context().cookies();
      expect(cookies, `${vp.label}: no cookies`).toEqual([]);
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 11: Public endpoint shapes are valid
  // ──────────────────────────────────────────────────────────────
  test('Flow 11: Contract — public endpoints return valid response shapes', async ({ request }) => {
    const endpoints = [
      { method: 'GET', url: `${BASE}/public/locations/demo/info` },
      { method: 'GET', url: `${BASE}/public/locations/demo/menu` },
    ];

    for (const ep of endpoints) {
      const res = await request.get(ep.url);
      expect(res.status(), `${ep.url} should return 200`).toBe(200);
      const body = await res.json();
      expect(body).toBeTruthy();
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 12: Order creates with 400 on completely invalid body
  // ──────────────────────────────────────────────────────────────
  test('Flow 12: Contract — POST /api/orders returns 400 on completely invalid body', async ({ request }) => {
    const res = await request.post(`${BASE}/api/orders`, {
      data: { completely: 'invalid' },
    });
    expect(res.status()).toBe(400);
    const body = await res.json();
    expect(body.error || body.message).toBeTruthy();
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 13: Corrupted localStorage — app recovers gracefully
  // ──────────────────────────────────────────────────────────────
  test('Flow 13: Resilience — corrupted localStorage does not crash client menu', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.goto(`${BASE}/s/test-slug?dev=true`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 10000 });

    await page.evaluate(() => {
      const keys = Object.keys(localStorage);
      for (const key of keys) {
        localStorage.setItem(key, '{{{corrupted-json');
      }
    });

    await page.reload({ waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 10000 });

    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('JSON')
    );
    expect(criticalErrors, `JS errors after corrupted localStorage: ${criticalErrors.join('; ')}`).toEqual([]);

    const body = await page.textContent('body');
    expect(body).toBeTruthy();
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 14: Privilege escalation — a courier-role token is FORBIDDEN on owner endpoints
  // ──────────────────────────────────────────────────────────────
  test('Flow 14: Security — courier-role token cannot access owner endpoints (403)', async ({ request }) => {
    // #6: Flow 1 only proves "no token → 401". Role confusion (a VALID courier token reaching an
    // owner-only route) was untested. requireRole(['owner']) (auth.ts:110-112) returns EXACTLY 403
    // for a non-owner role — assert that, the negative auth control.
    const courierAuth = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'courier' } });
    expect(courierAuth.status()).toBe(200);
    const courierToken = (await courierAuth.json()).access_token;
    expectJwt(courierToken, 'courier mock-auth token');

    const ownerOnlyRoutes = [
      `${BASE}/api/owner/menu/categories`,
      `${BASE}/api/owner/menu/products`,
    ];
    for (const url of ownerOnlyRoutes) {
      const res = await request.get(url, { headers: { Authorization: `Bearer ${courierToken}` } });
      expect(res.status(), `courier role must be forbidden (403) on ${url}`).toBe(403);
    }
  });
});
