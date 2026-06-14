import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'http://localhost:3000';
const SLUG = 'demo';
const ARTIFACTS = 'e2e/artifacts';

test.describe('Promotions — CRUD + Validate', () => {
  let ownerToken: string;
  let createdPromoId: string;
  let createdPromoCode: string;

  test.beforeAll(async ({ request }) => {
    const mockRes = await request.post(`${BASE}/api/dev/mock-auth`, {
      data: { role: 'owner', locationSlug: SLUG },
    });
    expect(mockRes.status()).toBe(200);
    const body = await mockRes.json();
    ownerToken = body.access_token;
  });

  test('GET /api/owner/promotions returns list', async ({ request }) => {
    const res = await request.get(
      `${BASE}/api/owner/promotions`,
      { headers: { Authorization: `Bearer ${ownerToken}` } }
    );
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(Array.isArray(body.promotions)).toBe(true);
    expect(typeof body.total).toBe('number');
  });

  test('GET /api/owner/promotions/:id returns 404 for missing', async ({ request }) => {
    const res = await request.get(
      `${BASE}/api/owner/promotions/00000000-0000-0000-0000-000000000000`,
      { headers: { Authorization: `Bearer ${ownerToken}` } }
    );
    expect(res.status()).toBe(404);
  });

  test('POST creates a percentage promotion', async ({ request }) => {
    const res = await request.post(
      `${BASE}/api/owner/promotions`,
      {
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        data: {
          code: `E2E-TEST-${Date.now()}`,
          type: 'percentage',
          discount_value: 10,
          min_order_amount: 50000,
          valid_from: new Date().toISOString(),
          is_active: true,
          description: 'E2E test promotion - 10% off',
        },
      }
    );
    expect(res.status()).toBe(201);
    const body = await res.json();
    expect(body.code).toContain('E2E-TEST');
    expect(body.discount_value).toBe(10);
    expect(body.is_active).toBe(true);
    createdPromoId = body.id;
    createdPromoCode = body.code;
  });

  test('PATCH toggles active status to false', async ({ request }) => {
    expect(createdPromoId).toBeTruthy();
    const res = await request.patch(
      `${BASE}/api/owner/promotions/${createdPromoId}`,
      {
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        data: { is_active: false },
      }
    );
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.is_active).toBe(false);
  });

  test('PATCH toggles active status back to true', async ({ request }) => {
    expect(createdPromoId).toBeTruthy();
    const res = await request.patch(
      `${BASE}/api/owner/promotions/${createdPromoId}`,
      {
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        data: { is_active: true },
      }
    );
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.is_active).toBe(true);
  });

  test('PATCH 404 for non-existent promotion', async ({ request }) => {
    const res = await request.patch(
      `${BASE}/api/owner/promotions/00000000-0000-0000-0000-000000000000`,
      {
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        data: { is_active: true },
      }
    );
    expect(res.status()).toBe(404);
  });

  test('POST validate returns valid for the created promotion', async ({ request }) => {
    expect(createdPromoCode).toBeTruthy();
    const res = await request.post(
      `${BASE}/api/owner/promotions/validate`,
      {
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        data: { code: createdPromoCode, order_subtotal: 100000 },
      }
    );
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.valid).toBe(true);
    expect(typeof body.discount_amount).toBe('number');
  });

  test('POST validate returns invalid for unknown code', async ({ request }) => {
    const res = await request.post(
      `${BASE}/api/owner/promotions/validate`,
      {
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        data: { code: 'NONEXISTENT-CODE-999', order_subtotal: 100000 },
      }
    );
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.valid).toBe(false);
    expect(body.error).toBeTruthy();
  });

  test('POST validate returns invalid when order below minimum', async ({ request }) => {
    expect(createdPromoCode).toBeTruthy();
    const res = await request.post(
      `${BASE}/api/owner/promotions/validate`,
      {
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        data: { code: createdPromoCode, order_subtotal: 100 },
      }
    );
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.valid).toBe(false);
  });

  test('POST validate returns 401 without auth', async ({ request }) => {
    const res = await request.post(
      `${BASE}/api/owner/promotions/validate`,
      {
        headers: { 'Content-Type': 'application/json' },
        data: { code: 'ANYCODE', order_subtotal: 100000 },
      }
    );
    expect(res.status()).toBe(401);
  });

  test('DELETE removes the test promotion', async ({ request }) => {
    expect(createdPromoId).toBeTruthy();
    const res = await request.delete(
      `${BASE}/api/owner/promotions/${createdPromoId}`,
      { headers: { Authorization: `Bearer ${ownerToken}` } }
    );
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.success).toBe(true);
  });

  test('DELETE 404 for already-deleted promotion', async ({ request }) => {
    expect(createdPromoId).toBeTruthy();
    const res = await request.delete(
      `${BASE}/api/owner/promotions/${createdPromoId}`,
      { headers: { Authorization: `Bearer ${ownerToken}` } }
    );
    expect(res.status()).toBe(404);
  });

  test('DELETE returns 401 without auth', async ({ request }) => {
    expect(createdPromoId).toBeTruthy();
    const res = await request.delete(
      `${BASE}/api/owner/promotions/${createdPromoId}`
    );
    expect(res.status()).toBe(401);
  });

  test('GET returns 401 without auth', async ({ request }) => {
    const res = await request.get(
      `${BASE}/api/owner/promotions`
    );
    expect(res.status()).toBe(401);
  });

  test('POST returns 401 without auth', async ({ request }) => {
    const res = await request.post(
      `${BASE}/api/owner/promotions`,
      {
        headers: { 'Content-Type': 'application/json' },
        data: { code: 'TEST', type: 'percentage', discount_value: 10 },
      }
    );
    expect(res.status()).toBe(401);
  });

  test('POST returns 400 with invalid data', async ({ request }) => {
    const res = await request.post(
      `${BASE}/api/owner/promotions`,
      {
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        data: { discount_value: -5, type: 'invalid_type' },
      }
    );
    expect(res.status()).toBe(400);
  });

  test('UI: promotions page renders with list', async ({ page }) => {
    const mockRes = await page.request.post(`${BASE}/api/dev/mock-auth`, {
      data: { role: 'owner', locationSlug: SLUG },
    });
    const mockBody = await mockRes.json();

    await page.goto(`${BASE}/login`);
    await page.evaluate((t) => {
      localStorage.setItem('dos_access_token', t);
    }, mockBody.access_token);
    await page.goto(`${BASE}/admin/promotions`);

    await page.waitForSelector('[data-testid="promotions-list"], [data-testid="empty-state"], [data-testid="promotion-card"]', {
      timeout: 30000,
    });

    await page.screenshot({ path: `${ARTIFACTS}/promotions-page.png`, fullPage: true });
  });

  test('UI: no JS errors on promotions page', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    const mockRes = await page.request.post(`${BASE}/api/dev/mock-auth`, {
      data: { role: 'owner', locationSlug: SLUG },
    });
    const mockBody = await mockRes.json();

    await page.goto(`${BASE}/login`);
    await page.evaluate((t) => {
      localStorage.setItem('dos_access_token', t);
    }, mockBody.access_token);
    await page.goto(`${BASE}/admin/promotions`);
    await page.waitForTimeout(3000);

    const critical = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('ResizeObserver')
    );
    expect(critical).toEqual([]);
  });

  test('UI: no cookies set on promotions page', async ({ page }) => {
    const mockRes = await page.request.post(`${BASE}/api/dev/mock-auth`, {
      data: { role: 'owner', locationSlug: SLUG },
    });
    const mockBody = await mockRes.json();

    await page.goto(`${BASE}/login`);
    await page.evaluate((t) => {
      localStorage.setItem('dos_access_token', t);
    }, mockBody.access_token);
    await page.goto(`${BASE}/admin/promotions`);
    await page.waitForTimeout(2000);

    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });
});
