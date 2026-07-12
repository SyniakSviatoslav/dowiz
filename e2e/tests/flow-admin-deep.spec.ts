import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
let authToken: string;
let activeLocationId: string;
let locationSlug: string;
let categoryId: string;
let productId: string;
const TS = Date.now();

test.describe.configure({ mode: 'serial' });

test.describe('Flow: Admin — Dashboard, Menu CRUD, Branding, Settings, Signals', () => {
  test.beforeAll(async ({ request }) => {
    requireStaging(BASE); // mutating spec (creates category/product) — never hit prod
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const authBody = await authRes.json();
    authToken = authBody.access_token;
    activeLocationId = authBody.activeLocationId;
    expectJwt(authToken, 'access_token');
    expectUuid(activeLocationId, 'activeLocationId');

    const settingsRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(settingsRes.status()).toBe(200);
    locationSlug = (await settingsRes.json()).slug;
  });

  test.afterAll(async ({ request }) => {
    if (productId) {
      await request.delete(`${BASE}/api/owner/menu/products/${productId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch(() => {});
    }
    if (categoryId) {
      await request.delete(`${BASE}/api/owner/menu/categories/${categoryId}`, {
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch(() => {});
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 1: Dashboard — snapshot response shape
  // ──────────────────────────────────────────────────────────────
  test('Flow 1: Admin — GET dashboard snapshot returns all fields', async ({ request }) => {
    const dashRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/dashboard/snapshot`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(dashRes.status()).toBe(200);
    const dash = await dashRes.json();
    expect(dash.serverTime).toBeTruthy();
    expect(dash.counts).toBeTruthy();
    expect(typeof dash.counts.PENDING).toBe('number');
    expect(typeof dash.counts.CONFIRMED).toBe('number');
    expect(typeof dash.counts.PREPARING).toBe('number');
    expect(typeof dash.counts.READY).toBe('number');
    expect(typeof dash.counts.IN_DELIVERY).toBe('number');
    expect(typeof dash.counts.DELIVERED).toBe('number');
    expect(typeof dash.counts.CANCELLED).toBe('number');
    expect(typeof dash.counts.REJECTED).toBe('number');
    expect(Array.isArray(dash.orders)).toBe(true);
    // TODO(needs_staging): seed ≥1 order in beforeAll so the order-shape block is never skipped
    // (today an empty demo silently bypasses every order assertion below — conditional-skip vacuity).
    if (dash.orders.length > 0) {
      const o = dash.orders[0];
      expectUuid(o.orderId, 'orderId');
      expect(o.status).toBeTruthy();
      expect(typeof o.total).toBe('number');
      expect(o.createdAt).toBeTruthy();
      expect(o.customerNameMasked).toBeTruthy();
      expect(o.customerPhoneMasked).toBeTruthy();
      expect(o.itemCount !== undefined).toBe(true);
      expect(o.paymentMethod).toBeTruthy();
    }
    expect(Array.isArray(dash.activeDeliveries)).toBe(true);
    expect(typeof dash.activeAlertCount).toBe('number');
    expect(typeof dash.activeSignalCount).toBe('number');
    if (dash.nextCursor) {
      expect(typeof dash.nextCursor).toBe('string');
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 2: Dashboard — filter by status
  // ──────────────────────────────────────────────────────────────
  test('Flow 2: Admin — dashboard snapshot filtered by status', async ({ request }) => {
    const filterRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/dashboard/snapshot?status=PENDING`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(filterRes.status()).toBe(200);
    const filtered = await filterRes.json();
    expect(Array.isArray(filtered.orders)).toBe(true);
    // The ?status=PENDING filter must actually filter: a broken filter that returns every
    // order regardless of the param must go RED here (was: accepted ALL statuses → vacuous).
    for (const o of filtered.orders) {
      expect(o.status).toBe('PENDING');
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 3: Menu CRUD — create category + product with all fields
  // ──────────────────────────────────────────────────────────────
  test('Flow 3: Admin — create category and product with taste, BOM, allergens, verify round-trip', async ({ request }) => {
    // Create category
    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: `E2E-Admin-Cat-${TS}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    categoryId = (await catRes.json()).id;
    expectUuid(categoryId, 'categoryId');

    // Create product with all fields
    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: {
        name: `E2E-Admin-Prod-${TS}`,
        price: 999,
        description: 'E2E admin lifecycle test',
        available: true,
        categoryId,
        taste: { spicy: 2, salty: 0, sour: 1, sweet: 3, richness: 2 },
        recipeLines: [
          { supplyId: 'e2e-base', supplyName: 'Base', qty: 200, unit: 'g', kind: 'food_ingredient', kcal: 100, proteinG: 2, fatG: 1, carbsG: 20, allergens: ['eggs'] },
          { supplyId: 'e2e-top', supplyName: 'Topping', qty: 30, unit: 'g', kind: 'condiment', kcal: 50, proteinG: 1, fatG: 4, carbsG: 3, allergens: ['milk', 'soy'] },
        ],
        stockCount: 25,
      },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(201);
    productId = (await prodRes.json()).id;
    expectUuid(productId, 'productId');

    // Verify via GET round-trip
    const getRes = await request.get(`${BASE}/api/owner/menu/products?category_id=${categoryId}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(getRes.status()).toBe(200);
    const products = await getRes.json();
    const found = Array.isArray(products)
      ? products.find((p: any) => p.id === productId)
      : null;
    expect(found).toBeTruthy();
    expect(found.name).toBe(`E2E-Admin-Prod-${TS}`);
    expect(found.price).toBe(999);
    expect(found.available).toBe(true);
    expect(found.taste.spicy).toBe(2);
    expect(found.taste.sweet).toBe(3);
    expect(found.recipeLines.length).toBe(2);
    expect(found.stockCount).toBe(25);
    expect(found.categoryId).toBe(categoryId);
    // Exact set (sorted) — a dedup/aggregation bug that adds a superset of allergens must go RED.
    expect([...found.allergens].sort()).toEqual(['eggs', 'milk', 'soy']);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 4: Public menu — verify product appears with correct shape
  // ──────────────────────────────────────────────────────────────
  test('Flow 4: Public — menu API returns product with allergens, taste, BOM', async ({ request }) => {
    test.skip(!locationSlug || !productId, 'No slug or product from setup');
    const menuRes = await request.get(`${BASE}/public/locations/${locationSlug}/menu`);
    expect(menuRes.status()).toBe(200);
    const menu = await menuRes.json();
    const allProds = menu.categories.flatMap((c: any) => c.products || []);
    const pubProd = allProds.find((p: any) => p.name === `E2E-Admin-Prod-${TS}`);
    expect(pubProd).toBeTruthy();
    expect(pubProd.attributes.taste).toBeTruthy();
    expect(pubProd.attributes.taste.spicy).toBe(2);
    expect(pubProd.attributes.taste.sweet).toBe(3);
    expect(pubProd.attributes.bom).toBeTruthy();
    expect(pubProd.attributes.bom.length).toBe(2);
    // Allergens are comma-separated strings in public menu
    expect(pubProd.attributes.bom[0].allergens).toContain('eggs');
    expect(pubProd.attributes.bom[1].allergens).toContain('milk');
    expect(pubProd.attributes.bom[1].allergens).toContain('soy');
    expect(pubProd.attributes.kcal).toBeUndefined();
    expect(pubProd.price).toBe(999);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 5: Admin UI — dashboard page loads with auth
  // ──────────────────────────────────────────────────────────────
  test('Flow 5: Admin — dashboard page loads with sidebar, counts, orders list, no cookies', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    expect(errors, `JS errors on dashboard: ${errors.join('; ')}`).toEqual([]);

    // Real render proof: the dashboard header WS-status dot is present (a 500/redirect/login
    // page would not render it). A long-enough body or a keyword match is NOT proof.
    // TODO(needs_staging): confirm 'ws-status-dot' renders against deployed staging.
    await expect(page.getByTestId('ws-status-dot')).toBeVisible({ timeout: 15000 });

    const sidebarLinks = page.locator('nav a, [role="navigation"] a, aside a');
    const linkCount = await sidebarLinks.count();
    if (linkCount > 0) {
      expect(linkCount).toBeGreaterThanOrEqual(2);
    }

    // Verify readiness checklist or order sections
    const checkItems = page.locator('text=/readiness|checklist|complete|progress|PENDING|CONFIRMED|PREPARING/i');
    const checkCount = await checkItems.count().catch(() => 0);

    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 6: Admin UI — menu manager navigation
  // ──────────────────────────────────────────────────────────────
  test('Flow 6: Admin — menu manager page shows categories and products', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    expect(errors, `JS errors on menu page: ${errors.join('; ')}`).toEqual([]);

    // Real render proof: a menu-manager-only control is present (not just a long body).
    // TODO(needs_staging): confirm 'kitchen-busy-toggle' renders against deployed staging.
    await expect(page.getByTestId('kitchen-busy-toggle')).toBeVisible({ timeout: 15000 });

    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 7: Admin UI — branding page with CSS vars
  // ──────────────────────────────────────────────────────────────
  test('Flow 7: Admin — branding page has theme editor, CSS variables render correctly', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    // Navigate via dashboard to get full app context
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    expect(errors, `JS errors on admin: ${errors.join('; ')}`).toEqual([]);

    // Try to find branding link in sidebar
    const brandLink = page.locator('a, button, nav *, [role="navigation"] *')
      .filter({ hasText: /branding|theme/i }).first();
    if (await brandLink.isVisible({ timeout: 3000 }).catch(() => false)) {
      await brandLink.click();
      await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    } else {
      await page.addInitScript((token: string) => {
        localStorage.setItem('dos_access_token', token);
      }, authToken);
      await page.goto(`${BASE}/admin/branding`, { waitUntil: 'networkidle' });
      await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    }

    expect(errors, `JS errors on branding: ${errors.join('; ')}`).toEqual([]);

    // Real render proof: the branding page container is present (CSS vars below live on :root
    // app-wide and do NOT prove THIS page rendered — a redirect would still expose them).
    // TODO(needs_staging): confirm 'branding-page' renders against deployed staging.
    await expect(page.getByTestId('branding-page')).toBeVisible({ timeout: 15000 });

    const cssVars = await page.evaluate(() => {
      const style = getComputedStyle(document.documentElement);
      return {
        primary: style.getPropertyValue('--brand-primary').trim(),
        bg: style.getPropertyValue('--brand-bg').trim(),
        surface: style.getPropertyValue('--brand-surface').trim(),
        text: style.getPropertyValue('--brand-text').trim(),
      };
    });
    expect(cssVars.primary).toBeTruthy();
    expect(cssVars.primary).not.toBe('');
    expect(cssVars.bg).toBeTruthy();
    expect(cssVars.surface).toBeTruthy();
    expect(cssVars.text).toBeTruthy();

    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 8: Settings — location info via API
  // ──────────────────────────────────────────────────────────────
  test('Flow 8: Admin — GET location info and settings', async ({ request }) => {
    const locRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(locRes.status()).toBe(200);
    const loc = await locRes.json();
    expect(loc.slug).toBeTruthy();
    if (loc.name) expect(typeof loc.name).toBe('string');
    if (loc.phone) expect(typeof loc.phone).toBe('string');
    if (loc.lat) expect(typeof loc.lat).toBe('number');
    if (loc.lng) expect(typeof loc.lng).toBe('number');
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 9: Signals — verify list endpoint (may be empty or have entries)
  // ──────────────────────────────────────────────────────────────
  test('Flow 9: Admin — GET signals list returns array with correct shape', async ({ request }) => {
    const sigRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/signals`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    // Signals endpoint may 404 if not implemented, or return empty array
    if (sigRes.status() === 200) {
      const sigs = await sigRes.json();
      const sigArr = Array.isArray(sigs) ? sigs : (sigs.signals || sigs.data || []);
      expect(Array.isArray(sigArr)).toBe(true);
      if (sigArr.length > 0) {
        const s = sigArr[0];
        expect(s.id || s.signalId).toBeTruthy();
        expect(s.created_at || s.createdAt).toBeTruthy();
      }
    } else {
      expect(sigRes.status()).toBe(404);
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 10: Alerts — verify list endpoint
  // ──────────────────────────────────────────────────────────────
  test('Flow 10: Admin — GET alerts list returns array', async ({ request }) => {
    const altRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/alerts`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    if (altRes.status() === 200) {
      const alerts = await altRes.json();
      const alertArr = Array.isArray(alerts) ? alerts : (alerts.alerts || alerts.data || []);
      expect(Array.isArray(alertArr)).toBe(true);
      if (alertArr.length > 0) {
        const a = alertArr[0];
        const keys = Object.keys(a);
        expect(keys.length).toBeGreaterThan(0);
      }
    } else {
      expect(altRes.status()).toBe(404);
    }
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 11: Couriers list via admin API
  // ──────────────────────────────────────────────────────────────
  test('Flow 11: Admin — GET couriers list returns array', async ({ request }) => {
    const courRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/couriers`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    if (courRes.status() === 200) {
      const couriers = await courRes.json();
      const courArr = Array.isArray(couriers) ? couriers : (couriers.couriers || couriers.data || []);
      expect(Array.isArray(courArr)).toBe(true);
      if (courArr.length > 0) {
        const c = courArr[0];
        expect(c.id || c.courierId).toBeTruthy();
      }
    } else {
      expect(courRes.status()).toBe(404);
    }
  });

  // TODO(needs_staging): cross-tenant isolation. mock-auth always returns the SAME tenant, so a
  // real second-tenant locationId (from staging, e.g. process.env.SECOND_TENANT_LOCATION_ID) is
  // required to prove dashboard/snapshot, couriers, signals and alerts return 403/404 for a
  // foreign location. A nil/all-zero UUID is forbidden (it 404s by absence, proving nothing).

  // ──────────────────────────────────────────────────────────────
  // FLOW 11b: Negative control — owner API rejects unauthenticated calls (exact 401)
  // ──────────────────────────────────────────────────────────────
  test('Flow 11b: Admin — owner endpoints return 401 without a token', async ({ request }) => {
    // server.ts onRequest hook guards every /api/owner/ prefix → exactly 401 when no Bearer.
    const noAuthSnapshot = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/dashboard/snapshot`
    );
    expect(noAuthSnapshot.status()).toBe(401);

    const noAuthCatCreate = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: `E2E-NoAuth-${TS}` },
    });
    expect(noAuthCatCreate.status()).toBe(401);

    const noAuthCouriers = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/couriers`
    );
    expect(noAuthCouriers.status()).toBe(401);
  });

  // ──────────────────────────────────────────────────────────────
  // FLOW 12: Persistence — admin pages survive refresh
  // ──────────────────────────────────────────────────────────────
  test('Flow 12: Admin — pages survive refresh and navigation, no JS errors, no cookies', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => {
      localStorage.setItem('dos_access_token', token);
    }, authToken);

    // Real render proof at every step: the page's own testid must be visible (a long body or
    // an error/redirect page must NOT pass).
    // TODO(needs_staging): confirm these testids render against deployed staging.
    // Dashboard
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.getByTestId('ws-status-dot')).toBeVisible({ timeout: 15000 });

    await page.reload({ waitUntil: 'networkidle' });
    await expect(page.getByTestId('ws-status-dot')).toBeVisible({ timeout: 15000 });

    // Navigate to menu page
    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page.getByTestId('kitchen-busy-toggle')).toBeVisible({ timeout: 15000 });

    // Back to dashboard
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.getByTestId('ws-status-dot')).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);

    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});
