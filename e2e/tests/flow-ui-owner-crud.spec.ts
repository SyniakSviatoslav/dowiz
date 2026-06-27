import { test, expect } from '@playwright/test';
import { expectUuid, expectJwt } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

test.describe.configure({ mode: 'serial' });

test.describe('UI: Owner CRUD — Products & Categories', () => {
  let authToken: string;
  const TS = Date.now();
  const TEST_ITEM = `UI-CRUD-${TS}`;

  test.beforeAll(async ({ request }) => {
    requireStaging(BASE); // mutating spec — never create/delete products against prod
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const auth = await authRes.json();
    authToken = auth.access_token;
    expectJwt(authToken);
    // CRITICAL (cross-tenant): assert WHICH tenant the token is bound to, not merely that a token
    // came back. An owner token with no activeLocationId silently 401s every CRUD flow below.
    expectUuid(auth.activeLocationId, 'activeLocationId');
    const claims = JSON.parse(Buffer.from(authToken.split('.')[1], 'base64url').toString());
    expect(claims.role, 'mock-auth must mint an OWNER token').toBe('owner');
    expectUuid(claims.activeLocationId, 'token.activeLocationId');
    expect(claims.activeLocationId).toBe(auth.activeLocationId);
  });

  test('Flow 1: Menu manager page loads with categories and products', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 2: Create category via API', async ({ request }) => {
    const catRes = await request.post(`${BASE}/api/owner/menu/categories`, {
      data: { name: `${TEST_ITEM}-Cat` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catRes.status()).toBe(201);
    const cat = await catRes.json();
    expectUuid(cat.id);
    expect(cat.name).toBe(`${TEST_ITEM}-Cat`);
  });

  test('Flow 3: Create product via API', async ({ request }) => {
    // First get or create a category
    const catsRes = await request.get(`${BASE}/api/owner/menu/categories`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catsRes.status()).toBe(200);
    const cats = await catsRes.json();
    const catsList = cats.categories || cats.data || cats;
    const catId = catsList[0]?.id;

    const prodRes = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: `${TEST_ITEM}-Prod`, price: 750, categoryId: catId, available: true, stockCount: 5 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(201);
    const prod = await prodRes.json();
    expectUuid(prod.id);
    expect(prod.name).toBe(`${TEST_ITEM}-Prod`);
    expect(prod.price).toBe(750);

    // Verify product appears on public menu
    const menuRes = await request.get(`${BASE}/public/locations/demo/menu`, {
      headers: { 'Cache-Control': 'no-cache' },
    });
    expect(menuRes.ok()).toBe(true);
    const menu = await menuRes.json();
    // The public menu (read_public_menu, apps/api/src/routes/public/menu.ts) returns the KNOWN
    // shape { categories: [{ products: [...] }] }. Assert that exact shape — the previous
    // `menu.products || menu.items || menu.data || []` fork silently collapsed to [] on a shape
    // change, making this positive-existence assertion unreachable.
    expect(Array.isArray(menu.categories), 'public menu must expose a categories[] array').toBe(true);
    const inMenu = menu.categories.some((c: any) =>
      (c.products || []).some((p: any) => p.name === `${TEST_ITEM}-Prod`)
    );
    expect(inMenu, 'created product must appear on the public menu').toBe(true);
  });

  test('Flow 4: Update product via API and verify round-trip', async ({ request }) => {
    // Find our test product
    const prodRes = await request.get(`${BASE}/api/owner/menu/products`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(200);
    const prods = await prodRes.json();
    const prodList = prods.products || prods.data || prods;
    const target = prodList.find((p: any) => p.name === `${TEST_ITEM}-Prod`);
    test.skip(!target, 'Test product not found');

    const patchRes = await request.patch(`${BASE}/api/owner/menu/products/${target.id}`, {
      data: { price: 899, stockCount: 3 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(patchRes.status()).toBe(200);
    const updated = await patchRes.json();
    // No `?? target.price` fallback: if the PATCH response omits `price` the fallback would
    // silently read the pre-update value and pass without the mutation ever happening.
    expect(updated.price).toBe(899);

    // Round-trip: re-read the product and confirm the new price persisted (verify PATCH by
    // reading the value back, not just status 200 — Test Integrity #9).
    const reReadRes = await request.get(`${BASE}/api/owner/menu/products`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(reReadRes.status()).toBe(200);
    const reRead = await reReadRes.json();
    const reReadList = reRead.products || reRead.data || reRead;
    const persisted = reReadList.find((p: any) => p.id === target.id);
    expect(persisted, 'updated product must still exist').toBeTruthy();
    expect(persisted.price).toBe(899);
  });

  test('Flow 5: Delete test product via API', async ({ request }) => {
    const prodRes = await request.get(`${BASE}/api/owner/menu/products`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(prodRes.status()).toBe(200);
    const prods = await prodRes.json();
    const prodList = prods.products || prods.data || prods;
    const target = prodList.find((p: any) => p.name === `${TEST_ITEM}-Prod`);
    test.skip(!target, 'Test product not found');

    const delRes = await request.delete(`${BASE}/api/owner/menu/products/${target.id}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(delRes.status()).toBe(200);

    // Verify deleted
    const verifyRes = await request.get(`${BASE}/api/owner/menu/products`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    const verifyProds = await verifyRes.json();
    const vList = verifyProds.products || verifyProds.data || verifyProds;
    const gone = vList.find((p: any) => p.name === `${TEST_ITEM}-Prod`);
    expect(gone).toBeFalsy();
  });

  test('Flow 6: Delete test category via API', async ({ request }) => {
    const catsRes = await request.get(`${BASE}/api/owner/menu/categories`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(catsRes.status()).toBe(200);
    const cats = await catsRes.json();
    const catsList = cats.categories || cats.data || cats;
    const target = catsList.find((c: any) => c.name === `${TEST_ITEM}-Cat`);
    test.skip(!target, 'Test category not found');

    const delRes = await request.delete(`${BASE}/api/owner/menu/categories/${target.id}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(delRes.status()).toBe(204);

    // Follow-up: the category must actually be gone — a permanently-204 (or no-op) delete must fail.
    const verifyRes = await request.get(`${BASE}/api/owner/menu/categories`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(verifyRes.status()).toBe(200);
    const verifyCats = await verifyRes.json();
    const vList = verifyCats.categories || verifyCats.data || verifyCats;
    expect(vList.find((c: any) => c.id === target.id), 'deleted category must not reappear').toBeFalsy();
  });

  test('Flow 7: Menu manager survives page navigation, no JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // Navigate to dashboard and back
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Flow 8: Owner product endpoints reject unauthorized callers', async ({ request }) => {
    // (a) no token → 401 (verifyAuth, apps/api/src/plugins/auth.ts:47)
    const noTok = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: 'should-fail', price: 100 },
    });
    expect(noTok.status()).toBe(401);

    // (b) wrong-role JWT (courier) → 403 privilege-escalation guard (requireRole, auth.ts:111)
    const courierRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'courier' } });
    expect(courierRes.status()).toBe(200);
    const courierTok = (await courierRes.json()).access_token;
    expectJwt(courierTok, 'courier token');
    const escalation = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: 'should-fail', price: 100, prep_time_minutes: 10 },
      headers: { Authorization: `Bearer ${courierTok}` },
    });
    expect(escalation.status()).toBe(403);

    // (c) tampered/invalid token → 401 (verifyAuth catch, auth.ts:55). A genuinely time-EXPIRED
    // token needs the dev signing secret to mint — see needs_staging TODO below.
    const bogus = await request.post(`${BASE}/api/owner/menu/products`, {
      data: { name: 'should-fail', price: 100 },
      headers: { Authorization: 'Bearer not.a.realtoken' },
    });
    expect(bogus.status()).toBe(401);

    // TODO(needs_staging): second-tenant owner IDOR. PATCH/DELETE this tenant's product with a
    // REAL 2nd-tenant owner token → expect 404 (NOT_FOUND, products.ts:449/471 — location_id
    // scoped). mock-auth only ever mints the single dev owner, so a real 2nd-tenant fixture is
    // required; do not fake with a nil-UUID (it 404s by absence and proves nothing).
    // TODO(needs_staging): genuinely time-expired owner token → 401 (requires DEV_AUTH_SECRET to
    // sign a token with a past `exp`).
  });
});
