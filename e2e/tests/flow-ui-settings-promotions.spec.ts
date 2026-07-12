import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// FINDING #1: never default to the PROD host — a mock-auth/dev-backdoor + mutating spec
// must target staging (or an explicit VITE_BASE_URL). requireStaging(BASE) in beforeAll
// fails fast if BASE is prod/unknown.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

test.describe('UI: Admin Settings + Promotions via Form', () => {
  let authToken: string;
  let activeLocationId: string;
  const TS = Date.now();

  test.beforeAll(async ({ request }) => {
    requireStaging(BASE); // FINDING #1: refuse mock-auth + mutations against prod/unknown
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const body = await authRes.json();
    authToken = body.access_token;
    activeLocationId = body.activeLocationId;
    expectJwt(authToken, 'mock-auth access_token');
    expectUuid(activeLocationId, 'mock-auth activeLocationId');
  });

  test('Settings page loads with form fields', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/settings`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // Real render proof: the delivery-fee input is a stable, contract-bound field on
    // SettingsPage (id="settings-deliveryFee") — a 200-char body could be a login bounce.
    await expect(page.locator('#settings-deliveryFee')).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Settings API GET returns all fields', async ({ request }) => {
    const res = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    // FINDING #3: pin to the canonical field names the handler returns (spa-proxy.ts
    // GET /api/owner/settings maps r.name -> locationName). No truthy OR-chains across
    // non-existent aliases (restaurantName/businessName/phoneNumber don't exist).
    expect(typeof body.locationName).toBe('string');
    expect(body.locationName.length).toBeGreaterThan(0);
    expect(typeof body.phone).toBe('string');
    expect(typeof body.slug).toBe('string');
    expect(body.slug.length).toBeGreaterThan(0);
  });

  test('Settings API PUT updates name round-trip', async ({ request }) => {
    const getRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    const before = await getRes.json();
    // FINDING #3: write/read the canonical field. settingsSchema (spa-proxy.ts) accepts
    // `locationName`; it .strip()s an unknown `name`, so PUT { name } is a silent no-op.
    const originalName = before.locationName || '';

    // Update name
    const putRes = await request.put(`${BASE}/api/owner/settings`, {
      data: { locationName: `E2E Settings Test ${TS}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(putRes.status()).toBe(200);

    // Verify round-trip
    const getAfter = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(getAfter.status()).toBe(200);
    const after = await getAfter.json();
    expect(after.locationName).toBe(`E2E Settings Test ${TS}`);

    // Restore
    if (originalName) {
      const restore = await request.put(`${BASE}/api/owner/settings`, {
        data: { locationName: originalName },
        headers: { Authorization: `Bearer ${authToken}` },
<<<<<<< Updated upstream
      }).catch(() => {});
=======
      });
      // Best-effort restore, but still assert it succeeded so a broken PUT can't hide.
      expect(restore.status()).toBe(200);
>>>>>>> Stashed changes
    }
  });

  test('Settings API — delivery fee round-trip (integer ALL)', async ({ request }) => {
    const putRes = await request.put(`${BASE}/api/owner/settings`, {
      data: { deliveryFee: 200 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(putRes.status()).toBe(200);

    const getRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(getRes.status()).toBe(200);
    // FINDING #4: read back and assert the stored integer value (handler returns deliveryFee).
    const after = await getRes.json();
    expect(after.deliveryFee).toBe(200);
  });

  test('Settings API — hours_json round-trip', async ({ request }) => {
    const hours = {
      monday: [{ open: '09:00', close: '22:00' }],
      tuesday: [{ open: '09:00', close: '22:00' }],
    };
    const putRes = await request.put(`${BASE}/api/owner/settings`, {
      data: { hoursJson: hours },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(putRes.status()).toBe(200);
    // FINDING #5: GET and deep-equal the stored structure (handler returns hoursJson).
    const getRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(getRes.status()).toBe(200);
    const after = await getRes.json();
    expect(after.hoursJson).toEqual(hours);
  });

  test('Promotions page loads with list', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/promotions`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    // Real render proof: the page resolves to a loaded state (list OR empty-state),
    // never the loading skeleton or a login bounce. body.length>100 passed on either.
    await expect(
      page.locator('[data-testid="promotions-list"], [data-testid="empty-state"]').first()
    ).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Create promotion (percentage) via API', async ({ request }) => {
    // FINDING #3 (class): use the exact contract field names from promotions.ts
    // (.strict() schema → discount_value/min_order_amount/max_uses/valid_from/valid_until).
    const res = await request.post(`${BASE}/api/owner/promotions`, {
      data: {
        code: `PROMO-SETTINGS-${TS}`,
        type: 'percentage',
        discount_value: 15,
        valid_from: new Date().toISOString(),
        valid_until: new Date(Date.now() + 86400000 * 30).toISOString(),
        min_order_amount: 300,
        max_uses: 50,
      },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(201);
    const promo = await res.json();
    expectUuid(promo.id, 'created promotion id');
    expect(promo.code).toBe(`PROMO-SETTINGS-${TS}`);
    expect(promo.discount_value).toBe(15);
  });

  // FINDING #2: cross-tenant IDOR negative control. The promotions route scopes every
  // mutation by `WHERE id = $1 AND location_id = $2`, so an id outside the caller's tenant
  // yields rowCount 0 -> 404 (hidden, NOT 403 — no existence leak; same convention as
  // cross-tenant-realtime-qa.spec.ts L216). The always-on control below uses a random UUID
  // (provably not owned by this tenant) to prove the scoping. The full second-owner variant
  // (a VALID foreign owner token attacking THIS tenant's real promo id) needs a real second
  // seeded tenant on staging — gated on E2E_FOREIGN_OWNER_TOKEN, listed in needs_staging.
  test('Cross-tenant promotion mutation is rejected (IDOR)', async ({ request }) => {
    const foreignId = '00000000-0000-4000-8000-0000000000ff'; // valid UUID, not owned by this tenant

    const patchForeign = await request.patch(`${BASE}/api/owner/promotions/${foreignId}`, {
      data: { is_active: false },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(patchForeign.status()).toBe(404);

    const deleteForeign = await request.delete(`${BASE}/api/owner/promotions/${foreignId}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(deleteForeign.status()).toBe(404);

    // TODO(needs_staging): exercise a VALID second-owner token against THIS tenant's real
    // promo id (true IDOR, not just not-found). Requires a real second seeded tenant —
    // mock-auth only mints the single dev owner. Provide the foreign owner token via
    // E2E_FOREIGN_OWNER_TOKEN to enable.
    const foreignToken = process.env.E2E_FOREIGN_OWNER_TOKEN;
    test.skip(!foreignToken, 'E2E_FOREIGN_OWNER_TOKEN not set — needs a real second tenant');
    expectJwt(foreignToken, 'foreign owner token');

    const listRes = await request.get(`${BASE}/api/owner/promotions`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    const list = await listRes.json();
    const promos = list.promotions || list.data || list;
    const mine = promos.find((p: any) => p.code === `PROMO-SETTINGS-${TS}`);
    expectUuid(mine.id, 'own promotion id');

    const crossPatch = await request.patch(`${BASE}/api/owner/promotions/${mine.id}`, {
      data: { is_active: false },
      headers: { Authorization: `Bearer ${foreignToken}` },
    });
    expect(crossPatch.status()).toBe(404);

    const crossDelete = await request.delete(`${BASE}/api/owner/promotions/${mine.id}`, {
      headers: { Authorization: `Bearer ${foreignToken}` },
    });
    expect(crossDelete.status()).toBe(404);
  });

  test('Validate promotion returns valid', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/promotions/validate`, {
      // FINDING #3 (class): validate schema requires order_subtotal (integer minor units), not orderTotal.
      data: { code: `PROMO-SETTINGS-${TS}`, order_subtotal: 1000 },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.valid).toBe(true);
  });

  test('Toggle promotion active/inactive via API', async ({ request }) => {
    const listRes = await request.get(`${BASE}/api/owner/promotions`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    const list = await listRes.json();
    const promos = list.promotions || list.data || list;
    const target = promos.find((p: any) => p.code === `PROMO-SETTINGS-${TS}`);
    test.skip(!target, 'Promo not found');
    expectUuid(target.id, 'target promotion id');

    // FINDING #3 (class): the column is is_active, not active.
    const patchRes = await request.patch(`${BASE}/api/owner/promotions/${target.id}`, {
      data: { is_active: false },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(patchRes.status()).toBe(200);

    // FINDING #6: read back the single promotion and assert the toggle actually persisted.
    const fetchRes = await request.get(`${BASE}/api/owner/promotions/${target.id}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(fetchRes.status()).toBe(200);
    const fetched = await fetchRes.json();
    expect(fetched.is_active).toBe(false);
  });

  test('Delete promotion via API', async ({ request }) => {
    const listRes = await request.get(`${BASE}/api/owner/promotions`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    const list = await listRes.json();
    const promos = list.promotions || list.data || list;
    const target = promos.find((p: any) => p.code === `PROMO-SETTINGS-${TS}`);
    test.skip(!target, 'Promo not found');
    expectUuid(target.id, 'target promotion id');

    const delRes = await request.delete(`${BASE}/api/owner/promotions/${target.id}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(delRes.status()).toBe(200);

    // FINDING #7: confirm the resource is actually gone (GET by id -> 404).
    const goneRes = await request.get(`${BASE}/api/owner/promotions/${target.id}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(goneRes.status()).toBe(404);
  });

  test('Promotion page survives navigation', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/promotions`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    await page.goto(`${BASE}/admin/promotions`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    // Survives the round-trip: the promotions view re-renders to a loaded state.
    await expect(
      page.locator('[data-testid="promotions-list"], [data-testid="empty-state"]').first()
    ).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });
});
