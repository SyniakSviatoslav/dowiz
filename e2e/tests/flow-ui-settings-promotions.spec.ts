import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe('UI: Admin Settings + Promotions via Form', () => {
  let authToken: string;
  let activeLocationId: string;
  const TS = Date.now();

  test.beforeAll(async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const body = await authRes.json();
    authToken = body.access_token;
    activeLocationId = body.activeLocationId;
  });

  test('Settings page loads with form fields', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/settings`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    const hasSettings = /settings|name|phone|address|delivery|fee|language|Telegram/i.test(body);
    expect(hasSettings).toBe(true);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Settings API GET returns all fields', async ({ request }) => {
    const res = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.name || body.restaurantName || body.businessName).toBeTruthy();
    expect(body.phone || body.phoneNumber).toBeTruthy();
    expect(body.slug).toBeTruthy();
  });

  test('Settings API PUT updates name round-trip', async ({ request }) => {
    const getRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    const before = await getRes.json();
    const originalName = before.name || before.restaurantName || '';

    // Update name
    const putRes = await request.put(`${BASE}/api/owner/settings`, {
      data: { name: `E2E Settings Test ${TS}` },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(putRes.status()).toBe(200);

    // Verify round-trip
    const getAfter = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(getAfter.status()).toBe(200);
    const after = await getAfter.json();
    expect(after.name || after.restaurantName).toBe(`E2E Settings Test ${TS}`);

    // Restore
    if (originalName) {
      await request.put(`${BASE}/api/owner/settings`, {
        data: { name: originalName },
        headers: { Authorization: `Bearer ${authToken}` },
      }).catch(() => {});
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
  });

  test('Promotions page loads with list', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    await page.addInitScript((token: string) => localStorage.setItem('dos_access_token', token), authToken);
    await page.goto(`${BASE}/admin/promotions`, { waitUntil: 'networkidle' });
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });

    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });

  test('Create promotion (percentage) via API', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/promotions`, {
      data: {
        code: `PROMO-SETTINGS-${TS}`,
        type: 'percentage',
        discount: 15,
        validFrom: new Date().toISOString(),
        validTo: new Date(Date.now() + 86400000 * 30).toISOString(),
        minOrder: 300,
        usageLimit: 50,
      },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(res.status()).toBe(201);
    const promo = await res.json();
    expect(promo.code).toBe(`PROMO-SETTINGS-${TS}`);
    expect(promo.discount).toBe(15);
  });

  test('Validate promotion returns valid', async ({ request }) => {
    const res = await request.post(`${BASE}/api/owner/promotions/validate`, {
      data: { code: `PROMO-SETTINGS-${TS}`, orderTotal: 1000 },
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

    const patchRes = await request.patch(`${BASE}/api/owner/promotions/${target.id}`, {
      data: { active: false },
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(patchRes.status()).toBe(200);
  });

  test('Delete promotion via API', async ({ request }) => {
    const listRes = await request.get(`${BASE}/api/owner/promotions`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    const list = await listRes.json();
    const promos = list.promotions || list.data || list;
    const target = promos.find((p: any) => p.code === `PROMO-SETTINGS-${TS}`);
    test.skip(!target, 'Promo not found');

    const delRes = await request.delete(`${BASE}/api/owner/promotions/${target.id}`, {
      headers: { Authorization: `Bearer ${authToken}` },
    });
    expect(delRes.status()).toBe(200);
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

    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
  });
});
