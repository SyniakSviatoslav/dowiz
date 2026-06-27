import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../../helpers/assert-shape';
import { requireStaging } from '../../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'http://localhost:3000';
const SLUG = 'demo';
const ARTIFACTS = 'e2e/artifacts';

test.describe('Promotions — CRUD + Validate', () => {
  let ownerToken: string;
  let createdPromoId: string;
  let createdPromoCode: string;

  test.beforeAll(async ({ request }) => {
    // This spec POSTs/PATCHes/DELETEs promotions — fail fast against prod/unknown targets.
    requireStaging(BASE);
    const mockRes = await request.post(`${BASE}/api/dev/mock-auth`, {
      data: { role: 'owner', locationSlug: SLUG },
    });
    expect(mockRes.status()).toBe(200);
    const body = await mockRes.json();
    ownerToken = body.access_token;
    expectJwt(ownerToken, 'ownerToken');
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
    const sentCode = `E2E-TEST-${Date.now()}`;
    const res = await request.post(
      `${BASE}/api/owner/promotions`,
      {
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        data: {
          code: sentCode,
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
    // Exact echo-back — toContain would pass on a server-side mutation/truncation of the code.
    expect(body.code).toBe(sentCode);
    expect(body.discount_value).toBe(10);
    expect(body.is_active).toBe(true);
    expectUuid(body.id, 'created promo id');
    createdPromoId = body.id;
    createdPromoCode = body.code;
  });

  test('PATCH toggles active status to false', async ({ request }) => {
    expectUuid(createdPromoId);
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

  test('POST validate returns invalid for an inactive promotion', async ({ request }) => {
    // Runs while the promotion is toggled is_active=false (previous test).
    expect(createdPromoCode, 'promo must have been created').toMatch(/^E2E-TEST-/);
    const res = await request.post(
      `${BASE}/api/owner/promotions/validate`,
      {
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        data: { code: createdPromoCode, order_subtotal: 100000 },
      }
    );
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.valid).toBe(false);
    expect(body.error).toContain('not active');
  });

  test('PATCH toggles active status back to true', async ({ request }) => {
    expectUuid(createdPromoId);
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
    expect(createdPromoCode, 'promo must have been created').toMatch(/^E2E-TEST-/);
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
    // 10% of 100000 = 10000 (Math.floor(order_subtotal * discount_value / 100)).
    expect(body.discount_amount).toBe(10000);
  });

  test('POST validate is forbidden (403) for a courier token', async ({ request }) => {
    const courierRes = await request.post(`${BASE}/api/dev/mock-auth`, {
      data: { role: 'courier', locationSlug: SLUG },
    });
    expect(courierRes.status()).toBe(200);
    const courierToken = (await courierRes.json()).access_token;
    expectJwt(courierToken, 'courierToken');

    const res = await request.post(
      `${BASE}/api/owner/promotions/validate`,
      {
        headers: { Authorization: `Bearer ${courierToken}`, 'Content-Type': 'application/json' },
        data: { code: 'ANYCODE', order_subtotal: 100000 },
      }
    );
    // requireRole(['owner']) rejects a courier before the handler (auth.ts:111).
    expect(res.status()).toBe(403);
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
    expect(createdPromoCode, 'promo must have been created').toMatch(/^E2E-TEST-/);
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

  test('POST validate returns invalid for an expired promotion', async ({ request }) => {
    // Self-contained: create a promo whose valid_until is in the past, validate, then clean up.
    const yesterday = new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString();
    const expiredCode = `E2E-EXPIRED-${Date.now()}`;
    const createRes = await request.post(
      `${BASE}/api/owner/promotions`,
      {
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        data: {
          code: expiredCode,
          type: 'percentage',
          discount_value: 10,
          valid_from: new Date(Date.now() - 48 * 60 * 60 * 1000).toISOString(),
          valid_until: yesterday,
          is_active: true,
        },
      }
    );
    expect(createRes.status()).toBe(201);
    const expiredId = (await createRes.json()).id;
    expectUuid(expiredId, 'expired promo id');

    const res = await request.post(
      `${BASE}/api/owner/promotions/validate`,
      {
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        data: { code: expiredCode, order_subtotal: 100000 },
      }
    );
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.valid).toBe(false);
    expect(body.error).toContain('expired');

    const delRes = await request.delete(
      `${BASE}/api/owner/promotions/${expiredId}`,
      { headers: { Authorization: `Bearer ${ownerToken}` } }
    );
    expect(delRes.status()).toBe(200);
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

  test('cross-tenant IDOR: owner A cannot read/patch/delete owner B promotion', async ({ request }) => {
    // TODO(needs_staging): requires a SECOND, distinct tenant ('other-demo') with its OWN owner
    // user seeded on staging. /api/dev/mock-auth always mints for the single dev@deliveryos.com
    // user, so without a real 2nd tenant ownerB resolves to ownerA's location and this test
    // cannot distinguish isolation from a false pass — run only against staging with other-demo.
    const ownerBRes = await request.post(`${BASE}/api/dev/mock-auth`, {
      data: { role: 'owner', locationSlug: 'other-demo' },
    });
    expect(ownerBRes.status()).toBe(200);
    const ownerBToken = (await ownerBRes.json()).access_token;
    expectJwt(ownerBToken, 'ownerBToken');

    const createRes = await request.post(
      `${BASE}/api/owner/promotions`,
      {
        headers: { Authorization: `Bearer ${ownerBToken}`, 'Content-Type': 'application/json' },
        data: { code: `E2E-IDOR-${Date.now()}`, type: 'percentage', discount_value: 10 },
      }
    );
    expect(createRes.status()).toBe(201);
    const victimId = (await createRes.json()).id;
    expectUuid(victimId, 'ownerB promo id');

    // Owner A holds a valid token but must NOT see ownerB's promotion (tenant-scoped 404).
    const getRes = await request.get(
      `${BASE}/api/owner/promotions/${victimId}`,
      { headers: { Authorization: `Bearer ${ownerToken}` } }
    );
    expect(getRes.status()).toBe(404);

    const patchRes = await request.patch(
      `${BASE}/api/owner/promotions/${victimId}`,
      {
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        data: { is_active: false },
      }
    );
    expect(patchRes.status()).toBe(404);

    const delRes = await request.delete(
      `${BASE}/api/owner/promotions/${victimId}`,
      { headers: { Authorization: `Bearer ${ownerToken}` } }
    );
    expect(delRes.status()).toBe(404);

    // Cleanup as the rightful owner (B).
    await request.delete(`${BASE}/api/owner/promotions/${victimId}`, {
      headers: { Authorization: `Bearer ${ownerBToken}` },
    });
  });

  test('DELETE removes the test promotion', async ({ request }) => {
    expectUuid(createdPromoId);
    const res = await request.delete(
      `${BASE}/api/owner/promotions/${createdPromoId}`,
      { headers: { Authorization: `Bearer ${ownerToken}` } }
    );
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.success).toBe(true);
  });

  test('DELETE 404 for already-deleted promotion', async ({ request }) => {
    expectUuid(createdPromoId);
    const res = await request.delete(
      `${BASE}/api/owner/promotions/${createdPromoId}`,
      { headers: { Authorization: `Bearer ${ownerToken}` } }
    );
    expect(res.status()).toBe(404);
  });

  test('DELETE returns 401 without auth', async ({ request }) => {
    expectUuid(createdPromoId);
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
