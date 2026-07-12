import { test, expect } from '@playwright/test';
<<<<<<< Updated upstream

const BASE = 'https://dowiz.fly.dev';
=======
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
>>>>>>> Stashed changes

test.describe('E2E: Login → Onboarding → Reliability', () => {

  // Exercise the REAL auth path; never run (and never hit the API write/login paths) against prod.
  test.beforeAll(() => requireStaging(BASE));

  test('Step 1: Cold login with test credentials', async ({ page }) => {
    await page.goto(`${BASE}/login`);
    await page.waitForLoadState('networkidle');
    
    // Should see login form
    await expect(page.locator('input[type="email"]')).toBeVisible({ timeout: 10000 });
    
    // Fill credentials
    await page.fill('input[type="email"]', 'test@dowiz.com');
    await page.fill('input[type="password"]', 'test123456');
    
    // Submit
    await page.click('button[type="submit"]');
    
    // Wait for navigation to admin
    await page.waitForURL('**/admin**', { timeout: 15000 });
    
    // Verify token stored in localStorage (not cookie)
    const token = await page.evaluate(() => localStorage.getItem('dos_access_token'));
    expect(token).toBeTruthy();
    
    // Verify no cookies set
    const cookies = await page.context().cookies();
    const authCookies = cookies.filter(c => c.name.includes('token') || c.name.includes('session') || c.name.includes('auth'));
    expect(authCookies.length).toBe(0);

    // Positive control: the token actually authorizes a protected API read (not just a non-null string).
    const apiRes = await page.request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(apiRes.status()).toBe(200);
    const profile = await apiRes.json();
    expectUuid(profile.id, 'owner location id');
  });

  test('Step 1b: After login, admin pages are accessible', async ({ page }) => {
    // Attach the error listener BEFORE any navigation so a crash during login/render is caught.
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    // Login first
    await page.goto(`${BASE}/login`);
    await page.fill('input[type="email"]', 'test@dowiz.com');
    await page.fill('input[type="password"]', 'test123456');
    await page.click('button[type="submit"]');
    await page.waitForURL('**/admin**', { timeout: 15000 });
    await page.waitForTimeout(2000);

    const criticalErrors = errors.filter(e => !e.includes('favicon'));
    expect(criticalErrors, `admin render threw: ${criticalErrors.join(' | ')}`).toHaveLength(0);
  });

  test('Step 2: Onboarding page loads', async ({ page }) => {
    // Login
    await page.goto(`${BASE}/login`);
    await page.fill('input[type="email"]', 'test@dowiz.com');
    await page.fill('input[type="password"]', 'test123456');
    await page.click('button[type="submit"]');
    await page.waitForURL('**/admin**', { timeout: 15000 });

    // Navigate to onboarding
    await page.goto(`${BASE}/admin/onboarding`);
    await page.waitForLoadState('networkidle');

    // Real DOM proof: the onboarding upload step renders (not a 500 / spinner / redirect).
    await expect(page.locator('[data-testid=upload-menu-cta]')).toBeVisible({ timeout: 10000 });

    // Take screenshot for evidence
    await page.screenshot({ path: 'e2e/artifacts/onboarding-loaded.png' });
  });

  test('Step 3: Clean re-login preserves state', async ({ browser }) => {
    // Create fresh context (simulates new browser session)
    const context = await browser.newContext({ storageState: undefined });
    const page = await context.newPage();

    // Login fresh
    await page.goto(`${BASE}/login`);
    await page.fill('input[type="email"]', 'test@dowiz.com');
    await page.fill('input[type="password"]', 'test123456');
    await page.click('button[type="submit"]');
    await page.waitForURL('**/admin**', { timeout: 15000 });
    await page.waitForTimeout(2000);

    // Check what page we land on (dashboard vs onboarding)
    const url = page.url();
    const isDashboard = url.includes('/admin') && !url.includes('/onboarding');
    expect(isDashboard, `expected to land on the admin dashboard, got ${url}`).toBe(true);

    // Verify data persisted: the menu manager renders its real control (not a 500 / redirect / empty shell).
    await page.goto(`${BASE}/admin/menu`, { timeout: 10000 });
    await expect(page.locator('[data-testid=kitchen-busy-toggle]')).toBeVisible({ timeout: 10000 });

    await context.close();
  });

  test('Step 3b: Logout protects admin pages', async ({ page }) => {
    // Login first
    await page.goto(`${BASE}/login`);
    await page.fill('input[type="email"]', 'test@dowiz.com');
    await page.fill('input[type="password"]', 'test123456');
    await page.click('button[type="submit"]');
    await page.waitForURL('**/admin**', { timeout: 15000 });

    // Clear token
    await page.evaluate(() => {
      localStorage.removeItem('dos_access_token');
      sessionStorage.removeItem('dos_access_token');
    });

    // Try to access admin
    await page.goto(`${BASE}/admin`, { timeout: 10000 });
    await page.waitForTimeout(2000);

    const url = page.url();
    const redirectedToLogin = url.includes('/login');
    expect(redirectedToLogin).toBe(true);

    // Server-side enforcement (not just the client redirect): a token-less protected read is 401.
    const protectedRes = await page.request.get(`${BASE}/api/owner/settings`);
    expect(protectedRes.status()).toBe(401);
  });

  test('Step 3c: Wrong password is rejected with a visible error', async ({ page }) => {
    await page.goto(`${BASE}/login`);
    await expect(page.locator('input[type="email"]')).toBeVisible({ timeout: 10000 });
    await page.fill('input[type="email"]', 'test@dowiz.com');
    await page.fill('input[type="password"]', 'definitely-the-wrong-password');
    await page.click('button[type="submit"]');

    // Error matrix (401): a visible alert appears and we are NOT navigated into /admin.
    await expect(page.locator('[role=alert]')).toBeVisible({ timeout: 10000 });
    await expect(page.locator('[role=alert]')).toContainText(/invalid|incorrect|password|failed/i);
    expect(page.url()).toContain('/login');
  });

  // TODO(needs_staging): cross-tenant IDOR control needs a REAL second tenant's location id
  // (nil-UUID 404s by absence, proving nothing — banned by Test Integrity §5). With a valid
  // test-owner token, GET /api/owner/locations/<other-tenant-location-id>/dashboard/snapshot
  // must return 404 (requireLocationAccess denies a non-member owner — see plugins/auth.ts).
  // Provide OTHER_TENANT_LOCATION_ID + run on staging to enable.
});
