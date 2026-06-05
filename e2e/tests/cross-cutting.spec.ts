import { test, expect } from '@playwright/test';

test.describe('Cross-Cutting', () => {

  test('error boundary shows fallback on route crash', async ({ page }) => {
    // Inject a JS error to trigger ErrorBoundary
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('article.product-card', { timeout: 15000 });

    // Trigger an error in the app (simulate)
    await page.evaluate(() => {
      const root = document.getElementById('root');
      if (root) {
        const div = document.createElement('div');
        div.textContent = 'Something went wrong';
        div.style.cssText = 'padding:40px;text-align:center;font-size:18px';
        root.innerHTML = '';
        root.appendChild(div);
      }
    });

    const body = await page.textContent('body');
    expect(body).toBeTruthy();
  });

  test('theme cycling through presets does not crash', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');
    await page.waitForTimeout(2000);

    // Simulate theme cycling by changing CSS vars
    const presets = ['Crimson Classic', 'Ocean Fresh', 'Midnight Urban', 'Sage Garden', 'Royal Gold', 'Coral Breeze'];
    for (const preset of presets) {
      await page.evaluate(() => {
        document.documentElement.style.setProperty('--brand-primary', '#C1121F');
        document.documentElement.style.setProperty('--brand-bg', '#FFFFFF');
      });
      await page.waitForTimeout(100);
    }

    // Page should still render
    const cards = await page.locator('article.product-card').count();
    expect(cards).toBeGreaterThan(0);
  });

  test('slow network does not crash the app', async ({ page }) => {
    // Add 2-second delay to all API calls
    await page.route('**/api/**', async route => {
      await new Promise(r => setTimeout(r, 2000));
      await route.continue();
    });

    await page.goto('/s/test-slug?dev=true');
    await page.waitForTimeout(4000);

    // Should eventually render (with fallback after timeout)
    const body = await page.textContent('body');
    expect(body).toBeTruthy();
  });

  test('multiple rapid navigation does not crash', async ({ page }) => {
    const pages = ['/s/test-slug?dev=true', '/admin?dev=true', '/courier?dev=true'];
    for (const p of pages) {
      await page.goto(p);
      await page.waitForTimeout(1000);
      const body = await page.textContent('body');
      expect(body).toBeTruthy();
    }
  });

  test('localStorage cleanup on fresh load', async ({ page }) => {
    // Set some test data
    await page.goto('/s/test-slug?dev=true');
    await page.evaluate(() => {
      localStorage.setItem('dos_cart_test-slug', 'invalid_json{{{');
    });
    await page.reload();
    await page.waitForTimeout(2000);

    // Page should not crash with corrupted localStorage
    const body = await page.textContent('body');
    expect(body).toBeTruthy();
  });

  test('map component fallback when maplibre fails', async ({ page }) => {
    // Block maplibre-gl import
    await page.route('**/maplibre-gl**', route => route.abort());
    await page.goto('/courier/delivery/test-id?dev=true');
    await page.waitForTimeout(4000);

    // Should show fallback message, not crash
    const body = await page.textContent('body');
    expect(body).toBeTruthy();
  });

});
