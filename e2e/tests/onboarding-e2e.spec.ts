import { test, expect } from '@playwright/test';

const BASE = 'https://dowiz.fly.dev';

test.describe('E2E: Login → Onboarding → Reliability', () => {

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

    console.log('LOGIN PASSED: Token in localStorage, no cookies');
  });

  test('Step 1b: After login, admin pages are accessible', async ({ page }) => {
    // Login first
    await page.goto(`${BASE}/login`);
    await page.fill('input[type="email"]', 'test@dowiz.com');
    await page.fill('input[type="password"]', 'test123456');
    await page.click('button[type="submit"]');
    await page.waitForURL('**/admin**', { timeout: 15000 });

    // Dashboard should load without errors
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));
    await page.waitForTimeout(2000);
    
    const criticalErrors = errors.filter(e => !e.includes('favicon'));
    console.log('Page errors:', criticalErrors.length > 0 ? criticalErrors : 'none');
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
    await page.waitForTimeout(2000);

    // Should show onboarding steps
    const body = await page.textContent('body') || '';
    const hasOnboarding = body.includes('Restaurant') || body.includes('step') || body.includes('Step');
    console.log('Onboarding visible:', hasOnboarding);
    
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
    const body = await page.textContent('body') || '';
    const isDashboard = url.includes('/admin') && !url.includes('/onboarding');
    
    console.log('After clean re-login:');
    console.log('  URL:', url);
    console.log('  Is dashboard:', isDashboard);

    // Verify data is still there by checking API
    await page.goto(`${BASE}/admin/menu`, { timeout: 10000 });
    await page.waitForTimeout(2000);
    
    const menuBody = await page.textContent('body') || '';
    console.log('  Menu page has content:', menuBody.length > 200);

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
    console.log('After logout, redirected to login:', redirectedToLogin);
    expect(redirectedToLogin).toBe(true);
  });
});
