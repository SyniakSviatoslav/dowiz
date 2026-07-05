import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'http://localhost:3000';
// WS host tracks BASE — a token minted on one env can't auth against another's WS.
const WS_BASE = BASE.replace(/^http/, 'ws');

// ── Helper: get real auth token ──
async function getOwnerToken(request: any): Promise<{ token: string; activeLocationId: string }> {
  const res = await request.post(`${BASE}/api/dev/mock-auth`);
  expect(res.status()).toBe(200);
  const body = await res.json();
  expectJwt(body.access_token, 'access_token');
  expectUuid(body.activeLocationId, 'activeLocationId');
  return { token: body.access_token as string, activeLocationId: body.activeLocationId as string };
}

test.describe('Hot-path sanity (Rule 14.4)', () => {

  test('SSR menu renders ≥1 product card', async ({ page }) => {
    const response = await page.goto(`${BASE}/s/demo`);
    expect(response?.status()).toBe(200);
    const cards = page.locator('[data-testid="menu-item"]');
    await expect(cards.first()).toBeVisible({ timeout: 15000 });
    const count = await cards.count();
    expect(count).toBeGreaterThanOrEqual(1);
    console.log(`SSR: ${count} product cards`);
  });

  test('Public menu API returns 200 with valid JSON', async ({ page }) => {
    const response = await page.request.get(`${BASE}/public/locations/demo/menu`);
    expect(response.status()).toBe(200);
    const body = await response.json();
    expect(body).toHaveProperty('categories');
    expect(Array.isArray(body.categories)).toBe(true);
    // A bare empty array proves nothing about menu rendering — the demo has products.
    expect(body.categories.length).toBeGreaterThan(0);
    expect(Array.isArray(body.categories[0].products)).toBe(true);
    const totalProducts = body.categories.reduce(
      (n: number, c: any) => n + (Array.isArray(c.products) ? c.products.length : 0), 0);
    expect(totalProducts).toBeGreaterThan(0);
  });

  test('Subdomain routing serves SPA at /admin', async ({ page }) => {
    const response = await page.goto(`https://demo-location.dowiz.org/admin`);
    expect(response?.status()).toBe(200);
    await expect(page.locator('#root')).toBeVisible({ timeout: 15000 });
  });

  test('Subdomain still serves public menu at /s/:slug', async ({ page }) => {
    const response = await page.goto(`${BASE}/s/demo`);
    expect(response?.status()).toBe(200);
    // The shell now injects a per-tenant <title> for SEO/link-unfurls (spa-shell.ts),
    // e.g. "Dubin & Sushi — Order Online | Dowiz" — the bare "Dowiz" default is only
    // the un-resolved fallback. Assert the branded storefront title served.
    const title = await page.title();
    expect(title).toContain('Dowiz');
    expect(title.length).toBeGreaterThan('Dowiz'.length);
  });
});

test.describe('Auth-first admin actions (Rule 14.2)', () => {
  // This block contains a mutating PUT /api/owner/settings — never let it run against prod.
  test.beforeAll(() => requireStaging(BASE));

  // Negative control: the owner API must reject unauthenticated reads/writes (401), not just
  // serve the happy path. verifyAuth (apps/api/src/plugins/auth.ts:47) → 401 on missing token.
  test('GET /api/owner/couriers without token → 401', async ({ request }) => {
    const res = await request.get(`${BASE}/api/owner/couriers`);
    expect(res.status()).toBe(401);
  });

  test('PUT /api/owner/settings without token → 401', async ({ request }) => {
    const res = await request.put(`${BASE}/api/owner/settings`, {
      headers: { 'Content-Type': 'application/json' },
      data: { locationName: 'x' },
    });
    expect(res.status()).toBe(401);
  });

  // TODO(needs-staging): cross-tenant isolation — mint a SECOND owner token for a different
  // location (requires a real 2nd tenant fixture; dev/mock-auth only yields one) and assert
  // token-A cannot read/write tenant-B's couriers/settings. Not faked here — see needs_staging.

  test('GET /api/owner/couriers — no duplicates, offline for inactive', async ({ request }) => {
    const { token } = await getOwnerToken(request);
    const res = await request.get(`${BASE}/api/owner/couriers`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.status()).toBe(200);
    const couriers = await res.json() as any[];
    expect(Array.isArray(couriers)).toBe(true);

    // Rule 14.2: verify data, not just status code
    const ids = couriers.map((c: any) => c.id);
    const uniqueIds = new Set(ids);
    expect(uniqueIds.size).toBe(ids.length); // no duplicates
    console.log(`Couriers: ${couriers.length} total, ${ids.length - uniqueIds.size} duplicates`);

    for (const c of couriers) {
      expect(['online', 'busy', 'offline']).toContain(c.status);
    }
  });

  test('PUT /api/owner/settings → GET preserves data round-trip', async ({ request }) => {
    const { token } = await getOwnerToken(request);

    // Step 1: GET current settings
    const getRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(getRes.status()).toBe(200);
    const before = await getRes.json() as any;

    // Step 2: PUT updated hours
    const testHours = {
      monday: { open: '09:00', close: '22:00', closed: false },
      friday: { open: '09:00', close: '23:00', closed: false },
      sunday: { closed: true },
    };
    try {
      const putRes = await request.put(`${BASE}/api/owner/settings`, {
        headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
        data: {
          locationName: before.locationName,
          phone: before.phone || '',
          address: before.address || 'Test Address',
          hoursJson: testHours,
        },
      });
      expect(putRes.status()).toBe(200);

      // Step 3: GET again and verify
      const getRes2 = await request.get(`${BASE}/api/owner/settings`, {
        headers: { Authorization: `Bearer ${token}` },
      });
      expect(getRes2.status()).toBe(200);
      const after = await getRes2.json() as any;

      // Rule 14.2: verify data persisted, not just HTTP 200
      expect(after.hoursJson).toEqual(testHours);
      expect(after.address).toBeTruthy();
      expect(after.address).not.toMatch(/^[0-9a-f-]{36}$/); // not a corrupted UUID
      console.log(`Settings: hours persisted, address="${after.address}"`);
    } finally {
      // Restore shared staging state so reruns / parallel tests don't observe our mutation.
      await request.put(`${BASE}/api/owner/settings`, {
        headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
        data: {
          locationName: before.locationName,
          phone: before.phone ?? '',
          address: before.address ?? '',
          hoursJson: before.hoursJson ?? {},
        },
      });
    }
  });

  test('WebSocket auth with real token', async ({ page }) => {
    const { token, activeLocationId } = await getOwnerToken(await page.request);
    const room = `location:${activeLocationId}:dashboard`;

    const result = await page.evaluate(async ({ t, wsBase, room }: { t: string; wsBase: string; room: string }) => {
      const ws = new WebSocket(`${wsBase}/ws?token=${t}`);
      return new Promise<string[]>((resolve) => {
        const events: string[] = [];
        const timer = setTimeout(() => { ws.close(); resolve([...events, 'timeout']); }, 8000);
        ws.onopen = () => events.push('open');
        ws.onmessage = (e) => {
          try {
            const data = JSON.parse(e.data);
            events.push(`msg:${data.type}`);
            if (data.type === 'auth_success') {
              ws.send(JSON.stringify({ type: 'subscribe', room }));
            }
            if (data.type === 'subscribed') { clearTimeout(timer); ws.close(); resolve(events); }
          } catch { events.push('parse_error'); }
        };
        ws.onclose = (e) => { events.push(`close:${e.code}`); clearTimeout(timer); resolve(events); };
      });
    }, { t: token, wsBase: WS_BASE, room });

    // Rule 14.2: verify the auth flow actually worked
    expect(result).toContain('msg:auth_success');
    expect(result).toContain('msg:subscribed');
    console.log(`WS auth: [${result.join(', ')}]`);
  });

  test('WebSocket rejects a bogus token (no auth_success, server closes)', async ({ page }) => {
    const result = await page.evaluate(async ({ wsBase }: { wsBase: string }) => {
      const ws = new WebSocket(`${wsBase}/ws?token=invalid.bogus.token`);
      return new Promise<string[]>((resolve) => {
        const events: string[] = [];
        const timer = setTimeout(() => { ws.close(); resolve([...events, 'timeout']); }, 9000);
        ws.onopen = () => events.push('open');
        ws.onmessage = (e) => {
          try { events.push(`msg:${JSON.parse(e.data).type}`); } catch { events.push('parse_error'); }
        };
        ws.onclose = (ev) => { events.push(`close:${ev.code}`); clearTimeout(timer); resolve(events); };
      });
    }, { wsBase: WS_BASE });

    // A bogus token must NEVER be authenticated; the server (websocket.ts:135-140) closes 1008.
    expect(result).not.toContain('msg:auth_success');
    expect(result).toContain('close:1008');
    console.log(`WS reject: [${result.join(', ')}]`);
  });
});

test.describe('Integration audit (Rule 14.3)', () => {

  test('CurrencySwitcher visible in client header', async ({ page }) => {
    // Client layout header has CurrencySwitcher next to LanguageSwitcher
    await page.goto(`${BASE}/branding-preview/demo`);
    await expect(page.locator('#root')).toBeVisible({ timeout: 15000 });
    // CurrencySwitcher renders a button with a stable aria-label (CurrencySwitcher.tsx:15:
    // `Switch currency. Current: …`). Matching on that is far tighter than `span.font-mono`.
    const currencyBtn = page.getByRole('button', { name: /switch currency/i });
    const exists = await currencyBtn.count();
    console.log(`CurrencySwitcher buttons found: ${exists}`);
    // >= 0 was always-green (accepts zero renders). Demand the component actually renders.
    // TODO(needs-staging): confirm /branding-preview/demo mounts the client header that hosts
    // CurrencySwitcher; if it does not, repoint this at the storefront root (ClientLayout) that does.
    expect(exists).toBeGreaterThanOrEqual(1);
  });

  test('PriceDisplay renders formatted money in the product detail flow', async ({ page }) => {
    // Bundle-script existence proved nothing about PriceDisplay USE. Drive the real flow:
    // open a product's detail modal — its confirm CTA (MenuPage.tsx:1218/1228) embeds
    // <PriceDisplay>, which renders formatMoney() output: `<int> ALL` or `<dec> €`.
    await page.goto(`${BASE}/s/demo`);
    const item = page.locator('[data-testid="menu-item"]').first();
    await expect(item).toBeVisible({ timeout: 15000 });
    await item.click();
    const confirm = page.locator('[data-testid="product-detail-confirm"]');
    await expect(confirm).toBeVisible({ timeout: 10000 });
    // PriceDisplay format from packages/shared-types formatMoney: digits + ALL/€.
    await expect(confirm).toContainText(/\d+(\.\d+)?\s*(ALL|€)/);
    // TODO(needs-staging): exercising the cart drawer + /checkout PriceDisplay totals as well
    // requires a live storefront run with an item added — see needs_staging.
  });
});
